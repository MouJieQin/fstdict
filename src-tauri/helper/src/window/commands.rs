use log::info;
use tauri::{AppHandle, Manager, State};

use crate::app_state::{MainWindowPinState, SelectionWindowPinState};
use crate::websocket::client::try_ws_send;
use fstdict_common::theme::set_app_theme;
use fstdict_common::window::notification::show_notification;
use fstdict_common::window::positioning::{is_cursor_over_window, position_window_near_cursor};

#[tauri::command]
pub fn set_theme(app_handle: AppHandle, theme: &str) {
    set_app_theme(&app_handle, theme);
}

/// Tauri command: update the pin state of the selection search panel.
#[tauri::command]
pub fn set_selection_window_pinned(state: State<'_, SelectionWindowPinState>, pinned: bool) {
    state.set_pinned(pinned);
    info!("Selection window pin state updated to: {}", pinned);

    let request_type = if pinned {
        "unregister_request"
    } else {
        "register_request"
    };

    let payload = serde_json::json!({
        "type": request_type,
        "data": {
            "event": "kCGEventLeftMouseDown",
            "window": "selection-float-search"
        }
    });

    if let Err(e) = state.ws_sender.try_send(payload.to_string()) {
        log::error!("Failed to send pin state over WebSocket: {:?}", e);
    }
}

/// Tauri command: update the pin state of the main helper panel.
#[tauri::command]
pub fn set_main_window_pinned(state: State<'_, MainWindowPinState>, pinned: bool) {
    state.set_pinned(pinned);
    info!("Main window pin state updated to: {}", pinned);

    let request_type = if pinned {
        "unregister_request"
    } else {
        "register_request"
    };

    let payload = serde_json::json!({
        "type": request_type,
        "data": {
            "event": "kCGEventLeftMouseDown",
            "window": "helper-main"
        }
    });

    if let Err(e) = state.ws_sender.try_send(payload.to_string()) {
        log::error!("Failed to send pin state over WebSocket: {:?}", e);
    }
}

/// Tauri command: trigger a notification banner from the frontend.
#[tauri::command]
pub fn trigger_notification(app: AppHandle, message: String) -> Result<(), tauri::Error> {
    show_notification(&app, message)
}

/// Shows the selection panel near the cursor (unless pinned).
pub fn show_selection_panel(app: &AppHandle) -> Result<(), String> {
    let label = "selection-float-search";
    let Some(win) = app.get_webview_window(label) else {
        return Ok(());
    };

    if let Some(pin_state) = app.try_state::<SelectionWindowPinState>() {
        if pin_state.is_pinned() {
            let _ = win.show();
            return Ok(());
        }
    }

    let _ = position_window_near_cursor(app, &win);
    let _ = win.show();
    enable_listen_hide(app, label);
    Ok(())
}

/// Shows the main helper panel near the cursor (unless pinned).
pub fn show_main_panel(app: &AppHandle) -> Result<(), String> {
    let label = "helper-main";
    let Some(win) = app.get_webview_window(label) else {
        return Ok(());
    };

    if let Some(pin_state) = app.try_state::<MainWindowPinState>() {
        if pin_state.is_pinned() {
            let _ = win.show();
            return Ok(());
        }
    }

    let _ = position_window_near_cursor(app, &win);
    let _ = win.show();
    enable_listen_hide(app, label);
    Ok(())
}

/// Hides a window if the cursor is outside its bounds and it's not pinned.
///
/// Returns `true` if the window was hidden or was already hidden.
pub fn hide_window_if_unpinned_and_outside(app: &AppHandle, label: &str) -> bool {
    let is_pinned = match label {
        "helper-main" => app
            .try_state::<MainWindowPinState>()
            .map(|s| s.is_pinned())
            .unwrap_or(false),
        "selection-float-search" => app
            .try_state::<SelectionWindowPinState>()
            .map(|s| s.is_pinned())
            .unwrap_or(false),
        _ => false,
    };

    if is_pinned {
        disable_listen_hide(app, label);
        return false;
    }

    let Some(win) = app.get_webview_window(label) else {
        disable_listen_hide(app, label);
        return true;
    };

    if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        disable_listen_hide(app, label);
        return true;
    }

    if !is_cursor_over_window(app, label) {
        let _ = win.hide();
        disable_listen_hide(app, label);
        return true;
    }

    false
}

fn disable_listen_hide(app: &AppHandle, label: &str) {
    toggle_listen_hide(app, label, false);
}

fn enable_listen_hide(app: &AppHandle, label: &str) {
    toggle_listen_hide(app, label, true);
}

fn toggle_listen_hide(app: &AppHandle, label: &str, enabled: bool) {
    let type_str = match label {
        "selection-float-search" => "toggle_selection_float_hide",
        "helper-main" => "toggle_helper_main_hide",
        _ => return,
    };

    let payload = serde_json::json!({
        "type": type_str,
        "data": {
            "enabled": enabled
        }
    });
    try_ws_send(app, &payload.to_string());
}
