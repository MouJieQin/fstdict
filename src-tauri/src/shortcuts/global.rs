use std::str::FromStr;
use std::time::Duration;

use super::double_copy::handle_double_copy;
use crate::app_state::MainWindowWsSender;
use enigo::Keyboard;
use log::{error, info};
#[cfg(target_os = "macos")]
use macos_accessibility_client::accessibility;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

pub fn register_global_shortcut(app: &AppHandle, shortcut_keys: &str) {
    match Shortcut::from_str(shortcut_keys) {
        Ok(shortcut) => {
            if let Err(e) = app.global_shortcut().register(shortcut) {
                error!(
                    "Failed to register global shortcut '{}': {:?}",
                    shortcut_keys, e
                );
            } else {
                info!("Successfully registered global shortcut: {}", shortcut_keys);
            }
        }
        Err(e) => {
            error!(
                "Invalid global shortcut string '{}': {:?}",
                shortcut_keys, e
            );
        }
    }
}

pub fn unregister_global_shortcut(app: &AppHandle, shortcut_keys: &str) {
    match Shortcut::from_str(shortcut_keys) {
        Ok(shortcut) => {
            if let Err(e) = app.global_shortcut().unregister(shortcut) {
                error!(
                    "Failed to unregister global shortcut '{}': {:?}",
                    shortcut_keys, e
                );
            } else {
                info!(
                    "Successfully unregistered global shortcut: {}",
                    shortcut_keys
                );
            }
        }
        Err(e) => {
            error!(
                "Invalid global shortcut string '{}': {:?}",
                shortcut_keys, e
            );
        }
    }
}

/// Registers all global system shortcuts.
pub fn register_global_shortcuts(app: &AppHandle) {
    // Copy key interception (platform-specific)
    #[cfg(target_os = "macos")]
    {
        if accessibility::application_is_trusted() {
            let copy_shortcut =
                Shortcut::from_str("super+c").expect("Invalid keyboard shortcut string");
            let _ = app.global_shortcut().register(copy_shortcut);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let copy_shortcut =
            Shortcut::from_str("control+c").expect("Invalid keyboard shortcut string");
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
        // "shift+alt+KeyO" => {
        //     handle_start_ocr_trigger(app);
        // }
        _ => {
            send_shortcut_event(app, &shortcut_str);
        }
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
            #[cfg(target_os = "macos")]
            {
                if accessibility::application_is_trusted() {
                    let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
                    let _ = enigo.key(enigo::Key::Meta, enigo::Direction::Press);
                    let _ = enigo.key(enigo::Key::Unicode('c'), enigo::Direction::Click);
                    let _ = enigo.key(enigo::Key::Meta, enigo::Direction::Release);
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                let mut enigo = enigo::Enigo::new(&enigo::Settings::default()).unwrap();
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

fn send_shortcut_event(app: &AppHandle, shortcut_str: &str) {
    let payload = serde_json::json!({
        "type": "shortcut_triggered",
        "data": {
            "shortcut": shortcut_str
        }
    });

    if let Some(ws_state) = app.try_state::<MainWindowWsSender>() {
        if let Err(e) = ws_state.ws_sender.try_send(payload.to_string()) {
            error!("Failed to send shortcut triggered over WebSocket: {:?}", e);
        }
    }
}

