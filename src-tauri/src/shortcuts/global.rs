#[cfg(target_os = "linux")]
use std::collections::HashMap;
use std::str::FromStr;
#[cfg(target_os = "linux")]
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::sync::OnceLock;
use std::time::Duration;

use super::double_copy::handle_double_copy;
use crate::websocket::client::try_ws_send;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::{error, info};
use serde_json::json;
use tauri::{AppHandle, Runtime};

#[cfg(target_os = "macos")]
use macos_accessibility_client::accessibility;

// tauri-plugin-global-shortcut types are imported on all platforms so that
// handle_shortcut_event keeps a consistent signature (Linux uses them only
// as type placeholders; the actual event dispatch runs on a background thread).
use tauri_plugin_global_shortcut::{Shortcut, ShortcutEvent, ShortcutState};

#[cfg(not(target_os = "linux"))]
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[cfg(target_os = "linux")]
use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState};

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn send_shortcut_event(app: &AppHandle, shortcut_str: &str) {
    let payload = json!({
        "type": "shortcut_triggered",
        "data": {
            "shortcut": shortcut_str
        }
    });
    try_ws_send(app, &payload.to_string());
}

/// Returns true when the accelerator represents a native copy shortcut
/// (Ctrl+C on Linux/Windows, Cmd+C on macOS), regardless of the casing or
/// "KeyC" vs "c" convention used by different backends.
fn is_copy_shortcut(s: &str) -> bool {
    let lower = s.to_lowercase();
    let has_modifier = lower.contains("control")
        || lower.contains("ctrl")
        || lower.contains("super")
        || lower.contains("cmd");
    let has_c = lower.ends_with("+c") || lower.ends_with("+keyc");
    has_modifier && has_c
}

/// Platform-agnostic shortcut dispatcher.
fn dispatch_shortcut(app: &AppHandle, shortcut_str: &str, pressed: bool) {
    if !pressed {
        return;
    }

    if is_copy_shortcut(shortcut_str) {
        passthrough_native_copy(app.clone(), shortcut_str);
        handle_double_copy(app);
    } else if shortcut_str.eq_ignore_ascii_case("shift+alt+k")
        || shortcut_str.eq_ignore_ascii_case("shift+alt+KeyK")
    {
        info!("Global shortcut triggered: {}", shortcut_str);
        let _ = screencapture::interactively_capture("screenshot.png");
    } else {
        info!("Global shortcut triggered: {}", shortcut_str);
        send_shortcut_event(app, shortcut_str);
    }
}

// ─── Non-Linux backend (macOS / Windows) ────────────────────────────────────

#[cfg(not(target_os = "linux"))]
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

#[cfg(not(target_os = "linux"))]
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

#[cfg(not(target_os = "linux"))]
pub fn register_global_shortcuts(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        if accessibility::application_is_trusted() {
            if let Ok(copy_shortcut) = Shortcut::from_str("super+c") {
                let _ = app.global_shortcut().register(copy_shortcut);
            }
        }
    }

    if let Ok(copy_shortcut) = Shortcut::from_str("shift+alt+k") {
        let _ = app.global_shortcut().register(copy_shortcut);
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(copy_shortcut) = Shortcut::from_str("control+c") {
            let _ = app.global_shortcut().register(copy_shortcut);
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn handle_shortcut_event(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let shortcut_str = shortcut.to_string();
    dispatch_shortcut(app, &shortcut_str, true);
}

/// Temporarily releases the copy shortcut, simulates a native copy keypress,
/// then re-registers the shortcut. This lets the foreground app receive the
/// copy event normally while we still detect the double-press pattern.
#[cfg(not(target_os = "linux"))]
fn passthrough_native_copy<R: Runtime>(app: AppHandle<R>, shortcut_str: &str) {
    let shortcut = match Shortcut::from_str(shortcut_str) {
        Ok(s) => s,
        Err(_) => return,
    };

    tauri::async_runtime::spawn(async move {
        let gs = app.global_shortcut();
        let _ = gs.unregister(shortcut);

        // Let unregister propagate to the event tap
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Simulate on main thread, then WAIT for completion via oneshot.
        // Without this, enigo may execute AFTER re-register, and the
        // simulated keypress gets captured by our own interceptor.
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let _ = app.run_on_main_thread(move || {
            #[cfg(target_os = "macos")]
            {
                if accessibility::application_is_trusted() {
                    if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                        let _ = enigo.key(Key::Meta, Direction::Press);
                        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
                        let _ = enigo.key(Key::Meta, Direction::Release);
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                    let _ = enigo.key(Key::Control, Direction::Press);
                    let _ = enigo.key(Key::Unicode('c'), Direction::Click);
                    let _ = enigo.key(Key::Control, Direction::Release);
                }
            }

            let _ = tx.send(());
        });

        // Block until enigo has actually finished on the main thread
        let _ = rx.await;

        // Give the target app time to process the copy
        tokio::time::sleep(Duration::from_millis(30)).await;

        let _ = gs.register(shortcut);
    });
}

// ─── Linux backend (handy-keys / evdev) ─────────────────────────────────────
// HotkeyManager is !Sync (it owns an mpsc::Receiver), so it must live on a
// single dedicated thread. Register/unregister requests are sent to that
// thread via a command channel; the thread drains all pending commands before
// each blocking recv(), so startup registrations are always applied before
// the first hotkey event is processed.

#[cfg(target_os = "linux")]
enum HotkeyCommand {
    Register(String),
    Unregister(String),
}

/// Sender half of the command channel. mpsc::Sender is Clone + Send + Sync,
/// so it can live in a static OnceLock and be shared safely.
#[cfg(target_os = "linux")]
static COMMAND_TX: OnceLock<mpsc::Sender<HotkeyCommand>> = OnceLock::new();

/// Ensures the background event loop is running and returns the command sender.
#[cfg(target_os = "linux")]
fn get_command_tx(app: &AppHandle) -> &'static mpsc::Sender<HotkeyCommand> {
    COMMAND_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<HotkeyCommand>();
        let app_clone = app.clone();

        std::thread::Builder::new()
            .name("hotkey-event-loop".into())
            .spawn(move || {
                info!("Linux hotkey event loop started (handy-keys / evdev)");

                // Try intercepting (grab) mode first; fall back to listen-only
                // if /dev/uinput write permission is unavailable.
                let manager = HotkeyManager::new_with_blocking().unwrap_or_else(|e| {
                    info!(
                        "Blocking grab unavailable ({}), falling back to listen-only mode",
                        e
                    );
                    HotkeyManager::new().expect("Failed to create HotkeyManager")
                });

                // Mapping tables live exclusively on this thread — no locks
                // needed because no other thread touches them.
                let mut accel_to_id: HashMap<String, HotkeyId> = HashMap::new();
                let mut id_to_accel: HashMap<HotkeyId, String> = HashMap::new();

                loop {
                    // Drain all pending commands before blocking. This ensures
                    // registrations sent during startup are applied before the
                    // first hotkey event is received.
                    while let Ok(cmd) = rx.try_recv() {
                        match cmd {
                            HotkeyCommand::Register(accel) => match Hotkey::from_str(&accel) {
                                Ok(hotkey) => match manager.register(hotkey) {
                                    Ok(id) => {
                                        accel_to_id.insert(accel.clone(), id);
                                        id_to_accel.insert(id, accel.clone());
                                        info!("Registered hotkey: {}", accel);
                                    }
                                    Err(e) => {
                                        error!("Failed to register '{}': {:?}", accel, e)
                                    }
                                },
                                Err(e) => error!("Invalid hotkey '{}': {:?}", accel, e),
                            },
                            HotkeyCommand::Unregister(accel) => {
                                if let Some(id) = accel_to_id.remove(&accel) {
                                    id_to_accel.remove(&id);
                                    match manager.unregister(id) {
                                        Ok(_) => info!("Unregistered hotkey: {}", accel),
                                        Err(e) => {
                                            error!("Failed to unregister '{}': {:?}", accel, e)
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Block until the next hotkey event. This is the only
                    // blocking call in the loop; commands sent while blocked
                    // will be processed on the next iteration (after this
                    // event returns).
                    match manager.recv() {
                        Ok(event) => {
                            let pressed = matches!(event.state, HotkeyState::Pressed);
                            if let Some(accel) = id_to_accel.get(&event.id) {
                                dispatch_shortcut(&app_clone, accel, pressed);
                            }
                        }
                        Err(e) => {
                            error!("Hotkey event loop disconnected: {:?}", e);
                            break;
                        }
                    }
                }
                info!("Linux hotkey event loop exited");
            })
            .expect("Failed to spawn hotkey event loop");

        tx
    })
}

#[cfg(target_os = "linux")]
pub fn register_global_shortcut(app: &AppHandle, shortcut_keys: &str) {
    let tx = get_command_tx(app);
    if let Err(e) = tx.send(HotkeyCommand::Register(shortcut_keys.to_string())) {
        error!(
            "Failed to send register command for '{}': {:?}",
            shortcut_keys, e
        );
    }
}

#[cfg(target_os = "linux")]
pub fn unregister_global_shortcut(app: &AppHandle, shortcut_keys: &str) {
    let Some(tx) = COMMAND_TX.get() else {
        return;
    };
    if let Err(e) = tx.send(HotkeyCommand::Unregister(shortcut_keys.to_string())) {
        error!(
            "Failed to send unregister command for '{}': {:?}",
            shortcut_keys, e
        );
    }
    let _ = app;
}

#[cfg(target_os = "linux")]
pub fn register_global_shortcuts(app: &AppHandle) {
    // Initialize the event loop (idempotent)
    let _ = get_command_tx(app);

    // handy-keys uses "ctrl" not "control" for string parsing
    register_global_shortcut(app, "ctrl+c");
    register_global_shortcut(app, "shift+alt+k");
}

/// No-op on Linux: the tauri plugin callback is never wired up because
/// event dispatch happens on the handy-keys background thread. The function
/// exists only to keep the public API identical across platforms.
#[cfg(target_os = "linux")]
pub fn handle_shortcut_event(_app: &AppHandle, _shortcut: &Shortcut, _event: ShortcutEvent) {
    // Intentionally empty — see background event loop in get_command_tx()
}

/// Temporarily unregisters the copy hotkey, simulates a native Ctrl+C press
/// via enigo, then re-registers the hotkey. This preserves foreground app
/// copy behavior while maintaining double-press detection.
#[cfg(target_os = "linux")]
fn passthrough_native_copy(app: AppHandle, shortcut_str: &str) {
    let shortcut_str = shortcut_str.to_string();

    tauri::async_runtime::spawn(async move {
        // Temporarily remove our interception so the simulated keypress
        // reaches the foreground application
        unregister_global_shortcut(&app, &shortcut_str);

        // Brief delay for the command to be picked up by the event loop
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Simulate on main thread with completion synchronization
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let _ = app.run_on_main_thread(move || {
            if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                let _ = enigo.key(Key::Control, Direction::Press);
                let _ = enigo.key(Key::Unicode('c'), Direction::Click);
                let _ = enigo.key(Key::Control, Direction::Release);
            }
            let _ = tx.send(());
        });

        // Ensure simulation completes before re-registering
        let _ = rx.await;

        // Give the foreground app time to process the copy event
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Restore our hotkey interception
        register_global_shortcut(&app, &shortcut_str);
    });
}
