use std::time::{Duration, Instant};

use crate::app_state::MainWindowWsSender;
use log::{error, info};
#[cfg(not(target_os = "macos"))]
use tauri::Emitter;
use tauri::{AppHandle, Manager, State};

use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::app_state::DoubleCopyTracker;

/// Maximum interval between two copy presses to count as a double-press (ms).
const DOUBLE_PRESS_THRESHOLD_MS: u64 = 400;

/// Detects double-press of the copy shortcut and triggers lookup.
pub fn handle_double_copy(state: State<'_, MainWindowWsSender>, app: &AppHandle) {
    let tracker = app.state::<DoubleCopyTracker>();
    let mut last_guard = tracker.last_pressed.lock().unwrap();
    let now = Instant::now();

    if let Some(last_time) = *last_guard {
        if now.duration_since(last_time) < Duration::from_millis(DOUBLE_PRESS_THRESHOLD_MS) {
            info!("Double copy detected: Cmd/Ctrl + C pressed twice rapidly");

            if let Ok(text) = app.clipboard().read_text() {
                info!("Clipboard content: {}", text);

                #[cfg(not(target_os = "macos"))]
                let _ = app.emit_to("main", "cgevent-select", text);

                #[cfg(target_os = "macos")]
                {
                    let payload = serde_json::json!({
                        "type": "double_copy",
                        "data": {
                            "text": text
                        }
                    });

                    if let Err(e) = state.ws_sender.try_send(payload.to_string()) {
                        error!("Failed to send pin state over WebSocket: {:?}", e);
                    }
                }
            }
            // Reset to prevent triple-press from triggering again
            *last_guard = None;
            return;
        }
    }

    *last_guard = Some(now);
}
