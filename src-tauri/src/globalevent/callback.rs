#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
use crate::commands;
use crate::websocket::client::try_ws_send;
use log::info;
use selection::get_text;

use tauri::AppHandle;

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn hide_helper_main_window(app: &AppHandle) {
    commands::hide_window_if_unpinned_and_outside(app, "helper-main");
}

#[cfg(all(target_os = "macos", not(feature = "dev-non-macos")))]
pub fn hide_helper_main_window(app: &AppHandle) {
    let payload = serde_json::json!({
        "type": "hide_helper_main_window",
        "data": {
        }
    });
    try_ws_send(app, &payload.to_string());
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub fn hide_helper_selection_window(app: &AppHandle) {
    commands::hide_window_if_unpinned_and_outside(app, "selection-float-search");
}

#[cfg(all(target_os = "macos", not(feature = "dev-non-macos")))]
pub fn hide_helper_selection_window(app: &AppHandle) {
    let payload = serde_json::json!({
        "type": "hide_helper_selection_window",
        "data": {
        }
    });
    try_ws_send(app, &payload.to_string());
}

pub fn handle_selection_event(app: &AppHandle) {
    let text: String = get_text();
    info!("Selected text: {}", text);
    if text.is_empty() {
        return;
    }
    send_selection_event(app, &text);
}

fn send_selection_event(app: &AppHandle, text: &str) {
    let payload = serde_json::json!({
        "type": "text_selection",
        "data": {
            "text_selected": text
        }
    });

    try_ws_send(app, &payload.to_string());
}
