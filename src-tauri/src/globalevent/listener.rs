// src/globalevent/listener.rs
use log::{debug, info, warn};
use monio::channel::listen_async_channel;
use monio::{Button, Event, EventType};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tauri::AppHandle;
use tokio::task::JoinHandle;

use super::callback;

// ---------------------------------------------------------------------------
// Subscriber bit flags.
// Each independent feature that needs global input events owns one bit.
// The hook runs while ANY bit is set; when all bits are cleared the hook
// is stopped to release the system-level hook and drop CPU usage to zero.
// ---------------------------------------------------------------------------
const SUB_HELPER_MAIN_HIDE: u8 = 1 << 0;
const SUB_HELPER_SELECTION_HIDE: u8 = 1 << 1;
const SUB_TEXT_SELECTION_CAPTURE: u8 = 1 << 2;

/// Bitmask of currently active subscribers. 0 means the hook must be stopped.
static ACTIVE_SUBSCRIBERS: AtomicU8 = AtomicU8::new(0);

// ---------------------------------------------------------------------------
// Global resources initialized once during Tauri setup.
// ---------------------------------------------------------------------------

/// AppHandle stored so that subscriber toggles (called from arbitrary threads)
/// can spawn the hook without receiving an AppHandle argument.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Running hook state. Only Some while the system hook is installed.
struct ListenerRuntime {
    monio_handle: monio::channel::ChannelHookHandle,
    task_join_handle: JoinHandle<()>,
}

static LISTENER_RUNTIME: Mutex<Option<ListenerRuntime>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Text-selection gesture tracking.
// We only need press + release (no MouseMoved processing) to detect a
// drag-select, which keeps CPU overhead minimal.
// ---------------------------------------------------------------------------
struct SelectionGesture {
    start_x: f64,
    start_y: f64,
    start_time: Instant,
}

// Tracks the previous left-click so we can detect double-click gestures.
struct LastClick {
    x: f64,
    y: f64,
    time: Instant,
}

static LAST_CLICK: Mutex<Option<LastClick>> = Mutex::new(None);

/// Maximum time between two presses to count as a double-click.
const DOUBLE_CLICK_MAX_INTERVAL_MS: u128 = 300;
/// Maximum cursor movement between two presses to count as a double-click.
const DOUBLE_CLICK_MAX_DISTANCE_PX: f64 = 5.0;

static SELECTION_GESTURE: Mutex<Option<SelectionGesture>> = Mutex::new(None);

/// Minimum cursor displacement to qualify as a text-selection drag.
const SELECTION_MIN_DRAG_PX: f64 = 5.0;
/// Minimum press-hold duration to qualify as a text-selection drag.
const SELECTION_MIN_DURATION_MS: u128 = 100;

// ---------------------------------------------------------------------------
// Initialization — call once in tauri::Builder setup.
// ---------------------------------------------------------------------------

/// Store the AppHandle for later use by subscriber toggles.
pub fn init(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

// ---------------------------------------------------------------------------
// Public subscriber controls.
// Call these from window show/hide handlers or Tauri commands.
// ---------------------------------------------------------------------------

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn enable_helper_main_hide() {
    toggle_subscriber(SUB_HELPER_MAIN_HIDE, true);
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn disable_helper_main_hide() {
    toggle_subscriber(SUB_HELPER_MAIN_HIDE, false);
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn enable_helper_selection_hide() {
    toggle_subscriber(SUB_HELPER_SELECTION_HIDE, true);
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn disable_helper_selection_hide() {
    toggle_subscriber(SUB_HELPER_SELECTION_HIDE, false);
}

pub fn toggle_helper_main_hide(enabled: bool) {
    toggle_subscriber(SUB_HELPER_MAIN_HIDE, enabled);
}

pub fn toggle_helper_selection_hide(enabled: bool) {
    toggle_subscriber(SUB_HELPER_SELECTION_HIDE, enabled);
}

pub fn toggle_text_selection_capture(enabled: bool) {
    toggle_subscriber(SUB_TEXT_SELECTION_CAPTURE, enabled);
}

// ---------------------------------------------------------------------------
// Core: reference-counted hook lifecycle.
// ---------------------------------------------------------------------------

/// Atomically set/clear a subscriber bit. Starts the hook on the
/// 0 → non-zero transition and stops it on the non-zero → 0 transition.
fn toggle_subscriber(flag: u8, enable: bool) {
    let prev = if enable {
        ACTIVE_SUBSCRIBERS.fetch_or(flag, Ordering::SeqCst)
    } else {
        ACTIVE_SUBSCRIBERS.fetch_and(!flag, Ordering::SeqCst)
    };
    let current = if enable { prev | flag } else { prev & !flag };

    // First subscriber arrived → install the single global hook
    if prev == 0 && current != 0 {
        start_hook();
    }
    // Last subscriber left → uninstall hook, free CPU
    if prev != 0 && current == 0 {
        stop_hook();
    }
}

fn start_hook() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        warn!("Cannot start global listener: AppHandle not initialized. Call init() first.");
        return;
    };
    tauri::async_runtime::spawn(async move {
        start_hook_inner(app).await;
    });
}

async fn start_hook_inner(app_handle: AppHandle) {
    // Lock scope only for checking & inserting runtime
    let maybe_runtime = {
        let mut guard = LISTENER_RUNTIME.lock().unwrap();
        if guard.is_some() {
            // Hook already running (possible race with rapid toggles); skip.
            return;
        }

        let (monio_handle, mut rx) = match listen_async_channel(64) {
            Ok(v) => v,
            Err(e) => {
                warn!("monio listen_async_channel failed: {e}");
                return;
            }
        };

        let app_clone = app_handle.clone();
        let task_join_handle = tokio::spawn(async move {
            event_loop(&app_clone, &mut rx).await;
            info!("monio event loop exited");
        });

        let rt = ListenerRuntime {
            monio_handle,
            task_join_handle,
        };
        *guard = Some(rt);
        // Release guard immediately by returning the mask
        ACTIVE_SUBSCRIBERS.load(Ordering::Relaxed)
    };

    info!("global monio hook started (subscriber mask: {maybe_runtime:#010b})");
}

fn stop_hook() {
    // Spawn the async work; do NOT hold mutex inside async closure
    tauri::async_runtime::spawn(async {
        if let Err(e) = stop_hook_inner().await {
            warn!("stop_hook_inner error: {e}");
        }
    });
}

async fn stop_hook_inner() -> Result<(), String> {
    // Take ownership of runtime, drop mutex guard BEFORE any await
    let runtime_opt = {
        let mut guard = LISTENER_RUNTIME.lock().unwrap();
        guard.take()
    };

    let Some(runtime) = runtime_opt else {
        return Ok(());
    };

    // Uninstall the system-level hook
    runtime
        .monio_handle
        .stop()
        .map_err(|e| format!("monio handle stop failed: {e}"))?;

    // Wait for task to finish, NO mutex guard held here
    let _ = runtime.task_join_handle.await;

    // Reset gesture state
    *SELECTION_GESTURE.lock().unwrap() = None;

    // Reset double-click tracker as well
    *LAST_CLICK.lock().unwrap() = None;

    info!("global monio hook stopped (all subscribers inactive)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Event dispatch: single hook feeds all active subscribers.
// ---------------------------------------------------------------------------

async fn event_loop(app: &AppHandle, rx: &mut tokio::sync::mpsc::Receiver<Event>) {
    while let Some(event) = rx.recv().await {
        let subscribers = ACTIVE_SUBSCRIBERS.load(Ordering::Relaxed);
        if subscribers == 0 {
            // Should not happen while hook runs, but guard anyway.
            continue;
        }
        dispatch_event(app, &event, subscribers);
    }
}

/// Route a single event to every active subscriber that cares about it.
fn dispatch_event(app: &AppHandle, event: &Event, subscribers: u8) {
    match event.event_type {
        EventType::MousePressed => handle_mouse_pressed(app, event, subscribers),
        EventType::MouseReleased => handle_mouse_released(app, event, subscribers),
        // MouseMoved / MouseDragged / keyboard / wheel are intentionally
        // ignored: none of the three subscribers need them, so we return
        // immediately to keep per-event cost near zero.
        _ => {}
    }
}

fn handle_mouse_pressed(app: &AppHandle, event: &Event, subscribers: u8) {
    let Some(mouse) = &event.mouse else {
        return;
    };
    // All three subscribers only care about the LEFT button.
    if mouse.button != Some(Button::Left) {
        return;
    }

    // Subscriber 1: hide helper-main window on outside click
    if subscribers & SUB_HELPER_MAIN_HIDE != 0 {
        callback::hide_helper_main_window(app);
    }

    // Subscriber 2: hide helper-selection window on outside click
    if subscribers & SUB_HELPER_SELECTION_HIDE != 0 {
        callback::hide_helper_selection_window(app);
    }

    // Subscriber 3: detect both double-click and drag-select gestures.
    if subscribers & SUB_TEXT_SELECTION_CAPTURE != 0 {
        let now = Instant::now();

        // --- Double-click detection: compare with previous press ---
        let is_double_click = {
            let mut last = LAST_CLICK.lock().unwrap();
            let double = match last.as_ref() {
                Some(prev) => {
                    let dt = now.duration_since(prev.time).as_millis();
                    let dx = mouse.x - prev.x;
                    let dy = mouse.y - prev.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    dt <= DOUBLE_CLICK_MAX_INTERVAL_MS && dist <= DOUBLE_CLICK_MAX_DISTANCE_PX
                }
                None => false,
            };
            // Record this press as the new "last click" for future detection.
            *last = Some(LastClick {
                x: mouse.x,
                y: mouse.y,
                time: now,
            });
            double
        };

        if is_double_click {
            info!(
                "Double-click text-selection detected at ({:.0}, {:.0})",
                mouse.x, mouse.y
            );
            #[cfg(target_os = "linux")]
            {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            callback::handle_selection_event(app);
        }

        // --- Also record press origin for drag-select detection on release ---
        // A double-click's second release will have near-zero displacement,
        // so it will NOT falsely trigger the drag-select path (5px threshold).
        *SELECTION_GESTURE.lock().unwrap() = Some(SelectionGesture {
            start_x: mouse.x,
            start_y: mouse.y,
            start_time: now,
        });
    }
}

fn handle_mouse_released(app: &AppHandle, event: &Event, subscribers: u8) {
    // Only the text-selection subscriber cares about release events.
    if subscribers & SUB_TEXT_SELECTION_CAPTURE == 0 {
        return;
    }

    let Some(mouse) = &event.mouse else {
        return;
    };
    if mouse.button != Some(Button::Left) {
        return;
    }

    // Consume the gesture state recorded at press time.
    let mut state = SELECTION_GESTURE.lock().unwrap();
    let Some(gesture) = state.take() else {
        return;
    };

    let dx = mouse.x - gesture.start_x;
    let dy = mouse.y - gesture.start_y;
    let distance = (dx * dx + dy * dy).sqrt();
    let duration_ms = gesture.start_time.elapsed().as_millis();

    // A real text selection involves dragging beyond a tiny threshold
    // and holding for more than an instant click.
    if distance >= SELECTION_MIN_DRAG_PX && duration_ms >= SELECTION_MIN_DURATION_MS {
        debug!("Text-selection gesture detected (dist={distance:.1}px, {duration_ms}ms)");
        callback::handle_selection_event(app);
    }
}
