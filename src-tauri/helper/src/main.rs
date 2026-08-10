#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use fstdict_common::logger::init_logging;
use fstdict_common::window_state::WindowState;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    App, AppHandle, Emitter, LogicalPosition, Manager, Position, State, Theme, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
    TrackingAreaOptions, WebviewWindowExt,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
// 1. Ensure you import MouseButton and MouseButtonState along with TrayIconEvent
use tauri::menu::{Menu, MenuItem};

tauri_panel! {
    panel!(FloatSearchPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
        with: {
            tracking_area: {
                options: TrackingAreaOptions::new()
                    .active_always()
                    .mouse_entered_and_exited()
                    .mouse_moved(),
                auto_resize: true
            }
        }
    })

    panel!(NotificationPanel {
        config: {
            // allows it to interact while being a background element
            can_become_key_window: false,
            is_floating_panel: true
        }
    })

    panel_event!(MyPanelEventHandler {
        window_did_move(notification: &NSNotification) -> (),
        window_did_resize(notification: &NSNotification) -> (),
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

// 1. Define the type wrapper you will register with Tauri
// pub struct SelectionWindowPinState(pub AtomicBool);
pub struct SelectionWindowPinState {
    pub is_pinned: AtomicBool,
    pub ws_sender: mpsc::Sender<String>, // Thread-safe channel sender
}

pub struct MainWindowPinState {
    pub is_pinned: AtomicBool,
    pub ws_sender: mpsc::Sender<String>, // Thread-safe channel sender
}

// Use a global or state-managed counter to manage debouncing tasks across commands safely.
// You can also add this to your app state via `.manage(NotificationState::default())`.

static CURRENT_TASK_ID: AtomicU64 = AtomicU64::new(0);
static PANEL_IS_CREATING: AtomicBool = AtomicBool::new(false);

fn set_noti_pannel_position(app: &AppHandle, w: &WebviewWindow) -> Result<(), String> {
    // Position window at the top-right corner of the monitor with user cursor focus
    if let Ok(cursor_pos) = app.cursor_position() {
        // println!(
        //     "[info]: Cursor position: ({}, {})",
        //     cursor_pos.x, cursor_pos.y
        // );
        if let Some(monitor) = app
            .available_monitors()
            .unwrap_or_default()
            .into_iter()
            .find(|m| {
                let m_pos = m.position();
                let m_size = m.size();
                // println!(
                //     "m_pos:({}, {}), m_size:({}, {})",
                //     m_pos.x, m_pos.y, m_size.width, m_size.height
                // );
                cursor_pos.x >= m_pos.x as f64
                    && cursor_pos.x <= (m_pos.x as f64 + m_size.width as f64)
                    && cursor_pos.y >= m_pos.y as f64
                    && cursor_pos.y <= (m_pos.y as f64 + m_size.height as f64)
            })
        {
            let scale = monitor.scale_factor();
            let screen_pos = monitor.position().to_logical::<f64>(scale);
            let screen_size = monitor.size().to_logical::<f64>(scale);

            let panel_width = 360.0;
            let edge_padding = 24.0;
            let top_padding = 40.0;

            let target_x = screen_pos.x + screen_size.width - panel_width - edge_padding;
            let target_y = screen_pos.y + top_padding;

            // Set layout position coordinates safely
            let _ = w.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
                target_x, target_y,
            )));
        }
    }
    return Ok(());
}

fn show_notification(app: &AppHandle, message: String) -> Result<(), String> {
    let task_id = CURRENT_TASK_ID.fetch_add(1, Ordering::SeqCst) + 1;

    // 1. REUSE PATH: Panel already exists in memory.
    if let Some(w) = app.get_webview_window("notify-layer") {
        // FIX: Force event delivery directly to this specific webview label
        let _ = w.emit_to("notify-layer", "update-message", &message);
        let _ = set_noti_pannel_position(&app, &w);
        let _ = w.show();

        // Start fade out timer
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(2000)).await;
            if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
                let _ = w.emit_to("notify-layer", "start-fade-out", ());
            }
            tokio::time::sleep(Duration::from_millis(550)).await;

            if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
                let _ = w.close();
            }
        });

        return Ok(());
    }

    // 2. CONCURRENCY PROTECTION: Prevent rapid double creations
    if PANEL_IS_CREATING.swap(true, Ordering::SeqCst) {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if let Some(w) = app_clone.get_webview_window("notify-layer") {
                    let _ = w.emit_to("notify-layer", "update-message", &message);
                    let _ = set_noti_pannel_position(&app_clone, &w);
                    let _ = w.show();
                    break;
                }
            }
        });
        return Ok(());
    }

    // 3. FIRST TIME CREATION PATH
    let encoded_message = urlencoding::encode(&message);
    let target_url = format!("notification.html?message={}", encoded_message);

    let panel_res = tauri_nspanel::PanelBuilder::<_, NotificationPanel>::new(&app, "notify-layer")
        .url(WebviewUrl::App(target_url.into()))
        .with_window(|window| {
            window
                .hidden_title(true)
                .inner_size(360.0, 90.0)
                .accept_first_mouse(true)
                .always_on_top(true)
                .transparent(true)
                .decorations(false)
                .resizable(false)
                .focusable(false)
        })
        .level(tauri_nspanel::PanelLevel::Status)
        .build();

    PANEL_IS_CREATING.store(false, Ordering::SeqCst);

    let panel = match panel_res {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to build panel: {:?}", e)),
    };

    panel.set_collection_behavior(
        tauri_nspanel::CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );

    let w: WebviewWindow = panel.to_window().unwrap().clone();

    let _ = set_noti_pannel_position(&app, &w);
    panel.show();

    // Fade out first-time notice after 3 seconds
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2000)).await;
        if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
            let _ = w.emit_to("notify-layer", "start-fade-out", ());
        }
        tokio::time::sleep(Duration::from_millis(550)).await;

        if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
            let _ = w.close();
        }
    });

    Ok(())
}

pub fn hide_window_if_need(app: &AppHandle, label: &str) -> bool {
    // 1. Fetch the globally managed window pin state from Tauri's registry map safely
    if label == "helper-main" {
        if let Some(pin_state) = app.try_state::<MainWindowPinState>() {
            // Load the atomic boolean value cleanly across thread boundaries
            let is_pinned = pin_state.is_pinned.load(Ordering::SeqCst);
            if is_pinned {
                return false; // Return false to indicate the window should remain untouched
            }
        }
    } else {
        if let Some(pin_state) = app.try_state::<SelectionWindowPinState>() {
            // Load the atomic boolean value cleanly across thread boundaries
            let is_pinned = pin_state.is_pinned.load(Ordering::SeqCst);
            if is_pinned {
                return false; // Return false to indicate the window should remain untouched
            }
        }
    }

    // 2. Fetch the target webview window handle
    let Some(win) = app.get_webview_window(label) else {
        return true;
    };

    // 3. Optimization Guard: If the window is already hidden or minimized, report true
    if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        return true;
    }

    // 4. If the cursor is completely outside the panel boundary box, hide it
    if !is_cursor_over_window(app, label) {
        let _ = win.hide();
        return true;
    }

    false
}

/// Determines whether the mouse cursor is currently positioned inside the bounds
/// of the specified webview window.
pub fn is_cursor_over_window(app: &AppHandle, label: &str) -> bool {
    // 1. Fetch the target webview window handle from Tauri
    let Some(win) = app.get_webview_window(label) else {
        log::warn!("Window with label '{}' not found.", label);
        return false;
    };

    // 2. Optimization Guard: If the window is hidden or minimized, the cursor cannot be over it
    if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        return false;
    }

    // 3. Fetch the current PHYSICAL mouse coordinates (as confirmed by your snippet)
    let Ok(cursor_physical) = app.cursor_position() else {
        log::error!("Failed to retrieve cursor position from system.");
        return false;
    };

    // 4. Fetch the window's raw physical position and size metrics
    // outer_position includes the window window frame title bar/shadow constraints
    let Ok(physical_pos) = win.outer_position() else {
        return false;
    };
    // inner_size tracks the active clickable viewport content body space
    let Ok(physical_size) = win.inner_size() else {
        return false;
    };

    // 5. Setup physical math bounding box coordinates
    let min_x = physical_pos.x as f64;
    let max_x = (physical_pos.x + physical_size.width as i32) as f64;

    // Note: On macOS desktop coordinates, Y increases downwards from the main screen's top-left corner
    let min_y = physical_pos.y as f64;
    let max_y = (physical_pos.y + physical_size.height as i32) as f64;

    // 6. Direct physical pixel comparison check
    cursor_physical.x >= min_x
        && cursor_physical.x <= max_x
        && cursor_physical.y >= min_y
        && cursor_physical.y <= max_y
}

#[tauri::command]
fn trigger_notification(app: AppHandle, message: String) -> Result<(), String> {
    return show_notification(&app, message);
}

// 2. Command to let the Vue frontend update the state directly
#[tauri::command]
fn set_selction_window_pinned(state: State<'_, SelectionWindowPinState>, pinned: bool) {
    // 1. Update the local state atomic flag
    state.is_pinned.store(pinned, Ordering::SeqCst);
    println!("[info]: Window pin status updated to: {}", pinned);

    // 2. Construct the matching target registration payload dictionary request
    let payload = serde_json::json!({
        "type": if pinned {"unregister_request"} else {"register_request"},
        "data": {
            "event": "kCGEventLeftMouseDown",
            "window": "selection-float-search"
        }
    });

    // 3. Serialize and push down the channel tube queue
    let json_string = payload.to_string();

    // try_send handles cross-thread transmission instantaneously without needing an async block layout
    if let Err(e) = state.ws_sender.try_send(json_string) {
        eprintln!("Failed to schedule outbound WS pin message: {:?}", e);
    }
}

#[tauri::command]
fn set_main_window_pinned(state: State<'_, MainWindowPinState>, pinned: bool) {
    // 1. Update the local state atomic flag
    state.is_pinned.store(pinned, Ordering::SeqCst);
    println!("[info]: Window pin status updated to: {}", pinned);

    // 2. Construct the matching target registration payload dictionary request
    let payload = serde_json::json!({
        "type": if pinned {"unregister_request"} else {"register_request"},
        "data": {
            "event": "kCGEventLeftMouseDown",
            "window": "helper-main"
        }
    });

    // 3. Serialize and push down the channel tube queue
    let json_string = payload.to_string();

    // try_send handles cross-thread transmission instantaneously without needing an async block layout
    if let Err(e) = state.ws_sender.try_send(json_string) {
        eprintln!("Failed to schedule outbound WS pin message: {:?}", e);
    }
}

fn set_window_position_near_cursor(app: &AppHandle, w: &WebviewWindow) -> Result<(), String> {
    if let Ok(mouse_physical) = app.cursor_position() {
        let monitors = app.available_monitors().unwrap_or_default();

        // 2 & 3. Find the monitor that physically contains the physical mouse cursor
        let mut target_monitor = monitors.first().cloned();
        for monitor in &monitors {
            let m_pos = monitor.position();
            let m_size = monitor.size();

            if mouse_physical.x >= m_pos.x as f64
                && mouse_physical.x <= (m_pos.x + m_size.width as i32) as f64
                && mouse_physical.y >= m_pos.y as f64
                && mouse_physical.y <= (m_pos.y + m_size.height as i32) as f64
            {
                target_monitor = Some(monitor.clone());
                break;
            }
        }

        if let Some(monitor) = target_monitor {
            let scale_factor = monitor.scale_factor();

            // 4. Transform physical screen bounds into logical workspace units
            let screen_pos = monitor.position().to_logical::<f64>(scale_factor);
            let screen_size = monitor.size().to_logical::<f64>(scale_factor);

            // FIX: Explicitly convert the physical mouse coordinates to logical coordinates
            let mouse_logical_x = mouse_physical.x / scale_factor;
            let mouse_logical_y = mouse_physical.y / scale_factor;

            // 5. Get window logical dimensions using the target scale factor
            let win_physical_size = w.inner_size().unwrap_or_default();
            let win_width = win_physical_size.width as f64 / scale_factor;
            let win_height = win_physical_size.height as f64 / scale_factor;

            // 6. FRIENDLY PLACEMENT MECHANISM
            let mut x;
            let mut y;
            let cursor_padding = 12.0; // Visual spacing between mouse tip and window border

            // --- Dynamic Horizontal Placement ---
            let monitor_center_x = screen_pos.x + (screen_size.width / 2.0);
            if mouse_logical_x > monitor_center_x {
                // Cursor is on the RIGHT half of the monitor -> Spawn panel safely to the LEFT
                x = mouse_logical_x - win_width - cursor_padding;
            } else {
                // Cursor is on the LEFT half of the monitor -> Spawn panel safely to the RIGHT
                x = mouse_logical_x + cursor_padding;
            }

            // --- Dynamic Vertical Placement ---
            let monitor_center_y = screen_pos.y + (screen_size.height / 2.0);
            if mouse_logical_y > monitor_center_y {
                // Cursor is on the BOTTOM half of the monitor -> Spawn panel safely ABOVE
                y = mouse_logical_y - win_height - cursor_padding;
            } else {
                // Cursor is on the TOP half of the monitor -> Spawn panel safely BELOW
                y = mouse_logical_y + cursor_padding;
            }

            // ===================== Hard Safety Boundary Clamp Fallbacks =====================
            let outer_margin = 8.0; // Screen border safety padding

            // Clamp Horizontal Edges
            if x + win_width > screen_pos.x + screen_size.width - outer_margin {
                x = screen_pos.x + screen_size.width - win_width - outer_margin;
            }
            if x < screen_pos.x + outer_margin {
                x = screen_pos.x + outer_margin;
            }

            // Clamp Vertical Edges
            if y + win_height > screen_pos.y + screen_size.height - outer_margin {
                y = screen_pos.y + screen_size.height - win_height - outer_margin;
            }
            if y < screen_pos.y + outer_margin {
                y = screen_pos.y + outer_margin;
            }

            // 7. Update Window Position safely using the computed Logical Coordinates
            let _ = w.set_position(Position::Logical(LogicalPosition::new(x, y)));
        }
    }
    Ok(())
}

fn show_selection_panel(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("selection-float-search") {
        if let Some(pin_state) = app.try_state::<SelectionWindowPinState>() {
            // Load the atomic boolean value cleanly across thread boundaries
            let is_pinned = pin_state.is_pinned.load(Ordering::SeqCst);
            if is_pinned {
                let _ = w.show();
                return Ok(());
            }
        }
        let _ = set_window_position_near_cursor(&app, &w);
        let _ = w.show();
        return Ok(());
    }
    Ok(())
}

fn show_main_panel(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("helper-main") {
        if let Some(pin_state) = app.try_state::<MainWindowPinState>() {
            // Load the atomic boolean value cleanly across thread boundaries
            let is_pinned = pin_state.is_pinned.load(Ordering::SeqCst);
            if is_pinned {
                let _ = w.show();
                return Ok(());
            }
        }
        let _ = set_window_position_near_cursor(&app, &w);
        let _ = w.show();
        return Ok(());
    }
    Ok(())
}

#[tauri::command]
fn hide_panel(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("selection-float-search") {
        let _ = w.hide();
    }
}

fn window_setup(
    app: &mut App,
    label: &str,
    config_filename: String,
    url: &str,
) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();

    // Convert the config filename into a thread-safe atomic reference counted slice
    let shared_config_name: Arc<str> = Arc::from(config_filename);

    let state = WindowState::load(&app_handle, &shared_config_name);

    let mut win_builder = WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
        .hidden_title(true)
        .inner_size(state.width, state.height)
        .accept_first_mouse(true)
        .zoom_hotkeys_enabled(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay);

    // Correct coordinate matching boundary check
    if let (Some(x), Some(y)) = (state.x, state.y) {
        if WindowState::is_position_visible(&app_handle, x, y, state.width, state.height) {
            win_builder = win_builder.position(x, y);
            log::info!("Restoring main window position to ({}, {})", x, y);
        } else {
            win_builder = win_builder.center();
            log::warn!(
                "Saved {} window position ({}, {}) is off-screen. Centering instead.",
                label,
                x,
                y
            );
        }
    } else {
        win_builder = win_builder.center();
        log::info!("No saved {} window position. Centering window.", label);
    }

    let win = win_builder.build()?;
    let _ = win.hide();

    // ===== Guard to suppress background events during initialization framework setup =====
    let is_ready = Arc::new(AtomicBool::new(false));

    // ===== Tokio Thread-Safe Debouncer Implementation =====
    let task_id = Arc::new(Mutex::new(0u64));
    const DEBOUNCE_MS: u64 = 350;

    // Use a single cohesive event controller to avoid reference move duplication
    let trigger_save = {
        let w = win.clone();
        let ah = app_handle.clone();
        let is_ready_clone = Arc::clone(&is_ready);
        let config_clone = Arc::clone(&shared_config_name);

        move || {
            if !is_ready_clone.load(Ordering::Relaxed) {
                return;
            }

            let task_id_clone = Arc::clone(&task_id);
            let w_clone = w.clone();
            let ah_clone = ah.clone();
            let async_config_target = Arc::clone(&config_clone);

            tauri::async_runtime::spawn(async move {
                let current_id = {
                    let mut guard = task_id_clone.lock().unwrap();
                    *guard += 1;
                    *guard
                };

                tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                let latest_id = {
                    let guard = task_id_clone.lock().unwrap();
                    *guard
                };
                if current_id == latest_id {
                    let current_state = WindowState::from_window(&w_clone);
                    current_state.save(&ah_clone, &async_config_target);
                }
            });
        }
    };

    // Remove the old win.on_window_event handler since it's blocked by NSPanel

    // Allow the layout thread to settle, then arm the tracker to safely accept events
    let is_ready_arm = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        is_ready_arm.store(true, Ordering::Relaxed);
    });

    // Convert the window to panel
    let panel = win.to_panel::<FloatSearchPanel>().unwrap();
    let handler: Retained<MyPanelEventHandler> = MyPanelEventHandler::new();
    let handle = app_handle.to_owned();

    // ====== Mount the position/resize listeners onto the active event handler delegate ======
    let move_trigger = trigger_save.clone();
    handler.window_did_move(move |_notification| {
        move_trigger();
    });

    let resize_trigger = trigger_save;
    handler.window_did_resize(move |_notification| {
        resize_trigger();
    });

    handler.window_did_become_key(move |_notification| {
        let app_name = handle.package_info().name.to_owned();
        println!("[info]: {:?} panel becomes key window!", app_name);
    });

    handler.window_did_resign_key(|_notification| {
        println!("[info]: panel resigned from key window!");
    });

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );
    // panel.set_hides_on_deactivate(true);
    panel.set_floating_panel(true);
    panel.set_event_handler(Some(handler.as_ref()));
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum CgEvent {
    #[serde(rename = "tauri_notification")]
    TauriNotification { data: TauriNotifyData },

    #[serde(rename = "ocr_result")]
    OcrResult { data: OcrResult },

    #[serde(rename = "handlerEventTextSelection")]
    HandlerEventTextSelection { data: TextSelectedData },

    #[serde(rename = "kCGEventLeftMouseDown")]
    kCGEventLeftMouseDown {},
}

#[derive(Debug, Deserialize)]
struct TextSelectedData {
    text_selected: String,
}

#[derive(Debug, Deserialize)]
struct OcrResult {
    ocr_txt: String,
}

#[derive(Debug, Deserialize)]
struct TauriNotifyData {
    message: String,
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    #[serde(rename = "type")]
    request_type: String,
    data: RegisterData,
}

#[derive(Debug, Serialize)]
struct RegisterData {
    event: String,
}

pub async fn start_cgevent_ws_client(
    ws_url: &str,
    app_handle: AppHandle,
    mut outbound_main_rx: mpsc::Receiver<String>,
    mut outbound_rx: mpsc::Receiver<String>,
) {
    loop {
        println!("connecting to cgevent ws: {}", ws_url);
        match connect_async(ws_url).await {
            Ok((ws_stream, _response)) => {
                println!("ws connected");
                let (mut write, mut read) = ws_stream.split();
                // Inner select event loop block
                loop {
                    tokio::select! {
                        // ── Branch A: Handle incoming network messages from C++ server ──
                        msg_result = read.next() => {
                            match msg_result {
                                Some(Ok(WsMessage::Text(text))) => {
                                    // ... [Your existing JSON parse parsing logic handles selections here] ...
                                match serde_json::from_str::<CgEvent>(&text) {
                                Ok(event) => match event {
                                    // ========= SAFE DISPATCH tauri_notification =========
                                    CgEvent::TauriNotification { data } => {
                                        println!("receive tauri_notification: {}", data.message);

                                        let app_clone = app_handle.clone();
                                        // CRITICAL FIX: Safe execution jump directly back onto macOS Thread 0
                                        let _ = app_handle.run_on_main_thread(move || {
                                            if let Err(e) =
                                                show_notification(&app_clone, data.message)
                                            {
                                                eprintln!(
                                                    "show_notification main thread call err: {}",
                                                    e
                                                );
                                            }
                                        });
                                    }
                                    CgEvent::OcrResult { data } => {
                                        let app_clone = app_handle.clone();
                                        // Dispatch to main thread to safely touch webview window maps
                                        let _ = app_handle.run_on_main_thread(move || {
                                            if data.ocr_txt.is_empty(){
                                                if let Err(e) =
                                                show_notification(&app_clone, "未识别到有效结果".into())
                                            {
                                                eprintln!(
                                                    "show_notification main thread call err: {}",
                                                    e
                                                );
                                            }
                                            return;
                                            }

                                            if is_cursor_over_window(&app_clone, "helper-main") {
                                                return;
                                            }
                                            let _ = show_main_panel(&app_clone);
                                            app_clone
                                            .emit_to(
                                                "helper-main",
                                                "cgevent-ocr",
                                                data.ocr_txt,
                                            )
                                            .ok();
                                            // println!("data.text_selected:{}", data.text_selected);
                                        });
                                        let text_str = serde_json::json!({
                                            "type": "register_request",
                                            "data": { "event": "kCGEventLeftMouseDown",
                                            "window":"helper-main"
                                        }
                                        })
                                        .to_string();

                                        // `.into()` handles the String -> Utf8Bytes translation implicitly
                                        let _ = write.send(WsMessage::Text(text_str.into())).await;
                                    }
                                    CgEvent::HandlerEventTextSelection { data } => {
                                        let app_clone = app_handle.clone();
                                        // Dispatch to main thread to safely touch webview window maps
                                        let _ = app_handle.run_on_main_thread(move || {
                                            if is_cursor_over_window(&app_clone, "selection-float-search") {
                                                return;
                                            }
                                            let _ = show_selection_panel(&app_clone);
                                            app_clone
                                            .emit_to(
                                                "selection-float-search",
                                                "cgevent-select",
                                                data.text_selected,
                                            )
                                            .ok();
                                            // println!("data.text_selected:{}", data.text_selected);
                                        });
                                        let text_str = serde_json::json!({
                                            "type": "register_request",
                                            "data": { "event": "kCGEventLeftMouseDown",
                                            "window":"selection-float-search"
                                        }
                                        })
                                        .to_string();

                                        // `.into()` handles the String -> Utf8Bytes translation implicitly
                                        let _ = write.send(WsMessage::Text(text_str.into())).await;
                                    }
                                    CgEvent::kCGEventLeftMouseDown {} => {
                                        let app_clone = app_handle.clone();
                                        // 1. Create a thread-safe single-use (oneshot) communication channel
                                        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                                        let _ = app_handle.run_on_main_thread(move || {
                                            println!("kCGEventLeftMouseDown");
                                            // Execute your window evaluation function inside the AppKit loop context
                                            let result = hide_window_if_need(
                                                &app_clone,
                                                "selection-float-search",
                                            );

                                            // Send the return value back across the channel to the background listener
                                            // .send() will return an error if the background thread was dropped prematurely
                                            let _ = tx.send(result);

                                        });
                                        // 3. Since this workspace runs inside a tokio async loop environment,
                                        // we can cleanly wait for the main thread to reply without blocking the socket execution
                                        match rx.await {
                                            Ok(was_hidden) => {
                                                println!("Main thread completed task. Window hidden status: {}", was_hidden);
                                                if was_hidden {
                                                    let text_str = serde_json::json!({
                                                        "type": "unregister_request",
                                                        "data": { "event": "kCGEventLeftMouseDown",
                                                        "window":"selection-float-search"
                                                    }
                                                    })
                                                    .to_string();
                                                    let _ = write
                                                        .send(WsMessage::Text(text_str.into()))
                                                        .await;
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to get return value from main thread: {:?}", e);
                                            }
                                        }
                                        let app_main_clone = app_handle.clone();
                                        // 1. Create a thread-safe single-use (oneshot) communication channel
                                        let (main_tx, main_rx) = tokio::sync::oneshot::channel::<bool>();
                                        let _ = app_handle.run_on_main_thread(move || {
                                            println!("kCGEventLeftMouseDown");
                                            // Execute your window evaluation function inside the AppKit loop context
                                            let result = hide_window_if_need(
                                                &app_main_clone,
                                                "helper-main",
                                            );

                                            // Send the return value back across the channel to the background listener
                                            // .send() will return an error if the background thread was dropped prematurely
                                            let _ = main_tx.send(result);

                                        });
                                        // 3. Since this workspace runs inside a tokio async loop environment,
                                        // we can cleanly wait for the main thread to reply without blocking the socket execution
                                        match main_rx.await {
                                            Ok(was_hidden) => {
                                                println!("Main thread completed task. Window hidden status: {}", was_hidden);
                                                if was_hidden {
                                                    let text_str = serde_json::json!({
                                                        "type": "unregister_request",
                                                        "data": { "event": "kCGEventLeftMouseDown",
                                                        "window":"helper-main"
                                                    }
                                                    })
                                                    .to_string();
                                                    let _ = write
                                                        .send(WsMessage::Text(text_str.into()))
                                                        .await;
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to get return value from main thread: {:?}", e);
                                            }
                                        }
                                    }
                                },
                                Err(e) => eprintln!("json parse error: {} raw={}", e, text),
                            }
                                }
                                Some(Ok(WsMessage::Close(_))) => { println!("ws close"); break; }
                                Some(Err(e)) => { eprintln!("ws read error: {}", e); break; }
                                None => break,
                                _ => {}
                            }
                        }

                        // ── Branch B: Handle outbound messages sent from Tauri Commands ──
                        Some(outbound_main_json) = outbound_main_rx.recv() => {
                            // Convert String down into Utf8Bytes for newer Tungstenite builds safely
                            let utf8_payload = Utf8Bytes::from(outbound_main_json);
                            if let Err(e) = write.send(WsMessage::Text(utf8_payload)).await {
                                eprintln!("Failed to transmit outbound JSON payload: {}", e);
                                break; // Break loop to trigger reconnection if write fails
                            }
                        }
                                                // ── Branch B: Handle outbound messages sent from Tauri Commands ──
                        Some(outbound_json) = outbound_rx.recv() => {
                            // Convert String down into Utf8Bytes for newer Tungstenite builds safely
                            let utf8_payload = Utf8Bytes::from(outbound_json);
                            if let Err(e) = write.send(WsMessage::Text(utf8_payload)).await {
                                eprintln!("Failed to transmit outbound JSON payload: {}", e);
                                break; // Break loop to trigger reconnection if write fails
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("ws connect failed: {}", e);
            }
        }
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

#[tokio::main]
async fn main() {
    let ws_endpoint = "ws://127.0.0.1:5959/ws/fstdict/helper";

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![
            set_selction_window_pinned,
            set_main_window_pinned,
            hide_panel,
            trigger_notification
        ])
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_config_dir = app
                .path()
                .app_data_dir()
                .expect("get app data dir failed")
                .join("Storage/config");
            fs::create_dir_all(&app_config_dir)?;

            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("./logs"));
            init_logging(&log_dir, "fstdict-helper".to_string());

            // ========== 后台spawn websocket客户端，不阻塞setup ==========
            // 1. Create a bounded channel queue (size 32 provides plenty of headroom)
            let (tx, rx) = mpsc::channel::<String>(32);
            let (main_tx, main_rx) = mpsc::channel::<String>(32);

            // 2. Initialize and handle your managed state payload injection
            app.manage(SelectionWindowPinState {
                is_pinned: AtomicBool::new(false),
                ws_sender: tx,
            });

            app.manage(MainWindowPinState {
                is_pinned: AtomicBool::new(false),
                ws_sender: main_tx,
            });

            // 3. Hand the receiver over to your background client task
            let app_handle = app.handle().clone();
            let ws_url = ws_endpoint.to_string();
            tokio::spawn(async move {
                start_cgevent_ws_client(&ws_url, app_handle, main_rx, rx).await;
            });

            window_setup(
                app,
                "helper-main",
                "helper-main-window-state.json".to_string(),
                "tauri://localhost/#/dict/39?env=helper_main_tauri",
            )?;
            window_setup(
                app,
                "selection-float-search",
                "helper-selection-window-state.json".to_string(),
                "tauri://localhost/#/dict/95?env=selection_float_search",
            )?;

            let quit_item = MenuItem::with_id(app, "quit", "Exit FstDict", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("FstDict Helper")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app_handle, event| {
                    if event.id.as_ref() == "quit" {
                        app_handle.exit(0);
                    }
                })
                .on_tray_icon_event(|tray_handle, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray_handle.app_handle();

                        if let Some(w) = app_handle.get_webview_window("helper-main") {
                            let _ = w.show();
                        }
                    }
                })
                .build(app)?;

            // let _ = init(app.app_handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("helper app failed to start");
}
