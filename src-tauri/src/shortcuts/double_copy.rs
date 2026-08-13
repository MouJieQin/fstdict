use std::time::{Duration, Instant};

use log::{error, info};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::app_state::DoubleCopyTracker;
use fstdict_common::window::notification::show_notification;

/// Maximum interval between two copy presses to count as a double-press (ms).
const DOUBLE_PRESS_THRESHOLD_MS: u64 = 400;

/// Detects double-press of the copy shortcut and triggers lookup.
pub fn handle_double_copy(app: &AppHandle) {
    let tracker = app.state::<DoubleCopyTracker>();
    let mut last_guard = tracker.last_pressed.lock().unwrap();
    let now = Instant::now();

    if let Some(last_time) = *last_guard {
        if now.duration_since(last_time) < Duration::from_millis(DOUBLE_PRESS_THRESHOLD_MS) {
            info!("Double copy detected: Cmd/Ctrl + C pressed twice rapidly");

            if let Ok(text) = app.clipboard().read_text() {
                info!("Clipboard content: {}", text);
                if let Err(e) = show_notification(app, text.into()) {
                    error!("show_notification error: {}", e);
                }
            }

            // Reset to prevent triple-press from triggering again
            *last_guard = None;
            return;
        }
    }

    *last_guard = Some(now);
}
