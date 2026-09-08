use log::info;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::app_state::DoubleCopyTracker;

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

                #[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
                {
                    use crate::commands;
                    use tauri::Emitter;

                    let _ = app.emit_to("helper-selection", "cgevent-select", text);
                    let _ = commands::show_selection_panel(app);
                }

                #[cfg(all(target_os = "macos", not(feature = "dev-non-macos")))]
                {
                    use crate::websocket::client::try_ws_send;
                    try_ws_send(
                        app,
                        &serde_json::json!({
                            "type": "double_copy",
                            "data": {
                                "text": text
                            }
                        })
                        .to_string(),
                    );
                }
            }
            // Reset to prevent triple-press from triggering again
            *last_guard = None;
            return;
        }
    }

    *last_guard = Some(now);
}
