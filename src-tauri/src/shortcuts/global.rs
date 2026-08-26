use std::str::FromStr;
use std::time::Duration;

use crate::app_state::MainWindowWsSender;
use enigo::Keyboard;
use log::{error, info};
use tauri::{App, AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

use super::double_copy::handle_double_copy;

fn register_global_shortcut(app: &App, shortcut_keys: &str) {
    let screenshot = Shortcut::from_str(shortcut_keys).expect("Invalid shortcut string");
    app.global_shortcut()
        .register(screenshot)
        .expect("Failed to register screenshot shortcut");
}

/// Registers all global system shortcuts.
pub fn register_global_shortcuts(app: &App) {
    // Screenshot / OCR trigger
    register_global_shortcut(app, "alt+shift+s");
    register_global_shortcut(app, "alt+shift+o");

    // Copy key interception (platform-specific)
    #[cfg(target_os = "macos")]
    {
        let copy_shortcut = Shortcut::from_str("super+c").expect("Invalid shortcut string");
        let _ = app.global_shortcut().register(copy_shortcut);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let copy_shortcut = Shortcut::from_str("control+c").expect("Invalid shortcut string");
        let _ = app.global_shortcut().register(copy_shortcut);
    }
}

/// Top-level dispatcher for global shortcut events.
pub fn handle_shortcut_event(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    use tauri_plugin_global_shortcut::ShortcutState;

    if event.state() != ShortcutState::Pressed {
        return;
    }

    let shortcut_str = shortcut.to_string();
    info!("Global shortcut triggered: {}", shortcut_str);

    match shortcut_str.as_str() {
        s if s == "super+KeyC" || s == "control+KeyC" => {
            passthrough_native_copy(app.clone(), *shortcut);
            if let Some(ws_state) = app.try_state::<MainWindowWsSender>() {
                handle_double_copy(ws_state, app);
            }
        }
        "shift+alt+KeyS" => {
            handle_toggle_selection_trigger(app);
        }
        "shift+alt+KeyO" => {
            handle_start_ocr_trigger(app);
        }
        _ => {}
    }
}

/// Temporarily releases the copy shortcut, simulates a native copy keypress,
/// then re-registers the shortcut. This lets the foreground app receive the
/// copy event normally while we still detect the double-press pattern.
fn passthrough_native_copy<R: Runtime>(app: AppHandle<R>, shortcut: Shortcut) {
    tauri::async_runtime::spawn(async move {
        let gs = app.global_shortcut();
        let _ = gs.unregister(shortcut);

        // Brief delay to let the unregister propagate
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Simulate physical keypress on the main thread (required on macOS)
        let _ = app.run_on_main_thread(move || {
            let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();

            #[cfg(target_os = "macos")]
            {
                let _ = enigo.key(enigo::Key::Meta, enigo::Direction::Press);
                let _ = enigo.key(enigo::Key::Unicode('c'), enigo::Direction::Click);
                let _ = enigo.key(enigo::Key::Meta, enigo::Direction::Release);
            }

            #[cfg(not(target_os = "macos"))]
            {
                let _ = enigo.key(enigo::Key::Control, enigo::Direction::Press);
                let _ = enigo.key(enigo::Key::Unicode('c'), enigo::Direction::Click);
                let _ = enigo.key(enigo::Key::Control, enigo::Direction::Release);
            }
        });

        // Allow the target app time to process the copy and update the clipboard
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Re-register the shortcut interceptor
        let _ = gs.register(shortcut);
    });
}

fn handle_toggle_selection_trigger(app: &AppHandle) {
    // toggle_selection
    let payload = serde_json::json!({
        "type": "toggle_selection",
        "data": {}
    });

    if let Some(ws_state) = app.try_state::<MainWindowWsSender>() {
        if let Err(e) = ws_state.ws_sender.try_send(payload.to_string()) {
            error!("Failed to send toggle selection over WebSocket: {:?}", e);
        }
    }
}

fn handle_start_ocr_trigger(app: &AppHandle) {
    let payload = serde_json::json!({
        "type": "start_ocr",
        "data": {}
    });

    if let Some(ws_state) = app.try_state::<MainWindowWsSender>() {
        if let Err(e) = ws_state.ws_sender.try_send(payload.to_string()) {
            error!("Failed to send start ocr over WebSocket: {:?}", e);
        }
    }
}
