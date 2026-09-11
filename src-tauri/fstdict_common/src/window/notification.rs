use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use super::positioning::{monitor_from_cursor, panel_position, position_notification_panel};
use log::{error, info};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_utils::config::Color;

/// Globally unique task ID for debouncing notification fade-out.
static CURRENT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Guard flag to prevent concurrent panel creation races.
static PANEL_CREATION_LOCK: AtomicBool = AtomicBool::new(false);

pub const NOTIFICATION_INNER_WIDTH: f64 = 360.0;
pub const NOTIFICATION_INNER_HEIGHT: f64 = 90.0;

/// Notification display duration before fade-out starts (milliseconds).
const NOTIFICATION_DISPLAY_MS: u64 = 2000;

/// Fade-out animation duration (milliseconds).
const NOTIFICATION_FADE_MS: u64 = 550;

/// Poll interval when waiting for an in-progress panel creation (milliseconds).
const CREATION_POLL_INTERVAL_MS: u64 = 100;

/// Maximum number of poll attempts while waiting for panel creation.
const MAX_CREATION_POLLS: u32 = 10;

/// Displays a transient notification panel with the given message.
///
/// If the panel already exists, it updates the message and resets the timer.
/// If creation is already in progress on another thread, the message is delivered
/// once the panel becomes available.
pub fn show_notification(app: &AppHandle, message: String) -> Result<(), tauri::Error> {
    let task_id = CURRENT_TASK_ID.fetch_add(1, Ordering::SeqCst) + 1;

    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window("notify-layer") {
        update_and_show_panel(&win, app, message, task_id);
        return Ok(());
    }

    // Concurrent creation guard
    if PANEL_CREATION_LOCK.swap(true, Ordering::SeqCst) {
        wait_and_deliver_message(app, message);
        return Ok(());
    }

    // Slow path: first-time creation
    let result = create_notification_panel(app, message, task_id);
    PANEL_CREATION_LOCK.store(false, Ordering::SeqCst);
    result
}

fn update_and_show_panel(win: &WebviewWindow, app: &AppHandle, message: String, task_id: u64) {
    let _ = win.emit_to("notify-layer", "update-message", &message);
    let _ = set_panel_position(app, win);
    let _ = win.show();

    let win_clone = win.clone();
    tauri::async_runtime::spawn(async move {
        schedule_fade_out(win_clone, task_id).await;
    });
}

fn wait_and_deliver_message(app: &AppHandle, message: String) {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        for _ in 0..MAX_CREATION_POLLS {
            tokio::time::sleep(Duration::from_millis(CREATION_POLL_INTERVAL_MS)).await;
            if let Some(win) = app_clone.get_webview_window("notify-layer") {
                let _ = win.emit_to("notify-layer", "update-message", &message);
                let _ = set_panel_position(&app_clone, &win);
                let _ = win.show();
                break;
            }
        }
    });
}

fn create_notification_panel(
    app: &AppHandle,
    message: String,
    task_id: u64,
) -> Result<(), tauri::Error> {
    let encoded = urlencoding::encode(&message);

    #[cfg(not(debug_assertions))]
    let base_url = "tauri://localhost";
    #[cfg(debug_assertions)]
    let base_url = "http://localhost:9595";
    let target_url = format!("{}/#/notification?message={}", base_url, encoded);

    // Fix 1 (flash): resolve the target position BEFORE building.
    // Windows paints the window the moment it's created; macOS defers the
    // first paint to the next run-loop iteration (which is why the old
    // code only flashed on Windows). Creating at the final position makes
    // the flash impossible. Refactor `position_notification_panel` into
    // `panel_position(monitor) -> LogicalPosition` + `apply_position(win)`.
    let pos = resolve_panel_position(app)?;

    let win = WebviewWindowBuilder::new(app, "notify-layer", WebviewUrl::App(target_url.into()))
        .inner_size(NOTIFICATION_INNER_WIDTH, NOTIFICATION_INNER_HEIGHT)
        .position(pos.x, pos.y) // create directly at the top-right spot
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .focusable(false)
        .transparent(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .build()?;

    // Fix 2 (white edges): WebView2's canvas defaults to opaque white.
    // "transparent: true" alone is NOT enough on Windows — set an explicit
    // alpha-0 background color (any alpha in 1..=255 is forced back to 255).
    let _ = win.set_background_color(Some(Color(0, 0, 0, 0)));

    // macOS may ignore builder .position() (tao#1023) — re-apply here.
    // On Windows this is a no-op move to the same spot.
    let _ = set_panel_position(app, &win);
    let _ = win.show();

    tauri::async_runtime::spawn(async move {
        schedule_fade_out(win, task_id).await;
    });

    Ok(())
}

/// Returns the panel's final position (cursor monitor, primary as fallback).
fn resolve_panel_position(app: &AppHandle) -> Result<tauri::LogicalPosition<f64>, tauri::Error> {
    match monitor_from_cursor(app) {
        Ok(Some(monitor)) => Ok(panel_position(&monitor)),
        Ok(None) => {
            info!("Cursor monitor not found; falling back to primary.");
            match app.primary_monitor() {
                Ok(Some(primary)) => Ok(panel_position(&primary)),
                _ => Err(tauri::Error::FailedToReceiveMessage), // or a meaningful error
            }
        }
        Err(err) => {
            error!("Failed to detect cursor monitor: {}", err);
            Err(err)
        }
    }
}

fn set_panel_position(app: &AppHandle, win: &WebviewWindow) -> Result<(), tauri::Error> {
    match monitor_from_cursor(app) {
        Ok(Some(monitor)) => {
            info!("Placing notification on cursor's monitor.");
            position_notification_panel(win, &monitor);
        }
        Ok(None) => {
            info!("Cursor monitor not found; falling back to primary.");
            if let Ok(Some(primary)) = app.primary_monitor() {
                position_notification_panel(win, &primary);
            }
        }
        Err(err) => {
            error!("Failed to detect cursor monitor: {}", err);
            return Err(err);
        }
    }
    Ok(())
}

async fn schedule_fade_out(win: WebviewWindow, task_id: u64) {
    tokio::time::sleep(Duration::from_millis(NOTIFICATION_DISPLAY_MS)).await;

    if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
        let _ = win.emit_to("notify-layer", "start-fade-out", ());
    }

    tokio::time::sleep(Duration::from_millis(NOTIFICATION_FADE_MS)).await;

    if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
        let _ = win.close();
    }
}
