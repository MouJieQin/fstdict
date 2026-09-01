use crate::window::permission_window;
use crate::window::updater_window;
use enigo::{Direction, Enigo, Keyboard, Settings};
use fstdict_common::theme::set_app_theme;
use std::{thread, time};
use tauri::AppHandle;
use tauri::Manager;

#[cfg(target_os = "macos")]
use crate::app_state::{CGEventHelperProcess, HelperProcess};

/// Basic greeting command for testing IPC connectivity.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn set_theme(app_handle: AppHandle, theme: &str) {
    set_app_theme(&app_handle, theme);
}

#[tauri::command]
pub fn simulate_key_press(key_code_str: String) {
    thread::spawn(move || {
        // Essential micro-delay allowing OS window management to attach text fields
        thread::sleep(time::Duration::from_millis(15));

        let mut enigo = Enigo::new(&Settings::default()).unwrap();

        // Resolve key_code_str (e.g., "KeyM", "KeyA") to native hardware IDs
        let native_code: Option<u16> = match key_code_str.as_str() {
            // macOS Scan codes vs Windows Virtual Key (VK) codes mapping
            "KeyA" => Some(if cfg!(target_os = "macos") { 0 } else { 0x41 }),
            "KeyB" => Some(if cfg!(target_os = "macos") { 11 } else { 0x42 }),
            "KeyC" => Some(if cfg!(target_os = "macos") { 8 } else { 0x43 }),
            "KeyD" => Some(if cfg!(target_os = "macos") { 2 } else { 0x44 }),
            "KeyE" => Some(if cfg!(target_os = "macos") { 14 } else { 0x45 }),
            "KeyF" => Some(if cfg!(target_os = "macos") { 3 } else { 0x46 }),
            "KeyG" => Some(if cfg!(target_os = "macos") { 5 } else { 0x47 }),
            "KeyH" => Some(if cfg!(target_os = "macos") { 4 } else { 0x48 }),
            "KeyI" => Some(if cfg!(target_os = "macos") { 34 } else { 0x49 }),
            "KeyJ" => Some(if cfg!(target_os = "macos") { 38 } else { 0x4A }),
            "KeyK" => Some(if cfg!(target_os = "macos") { 40 } else { 0x4B }),
            "KeyL" => Some(if cfg!(target_os = "macos") { 37 } else { 0x4C }),
            "KeyM" => Some(if cfg!(target_os = "macos") { 46 } else { 0x4D }),
            "KeyN" => Some(if cfg!(target_os = "macos") { 45 } else { 0x4E }),
            "KeyO" => Some(if cfg!(target_os = "macos") { 31 } else { 0x4F }),
            "KeyP" => Some(if cfg!(target_os = "macos") { 35 } else { 0x50 }),
            "KeyQ" => Some(if cfg!(target_os = "macos") { 12 } else { 0x51 }),
            "KeyR" => Some(if cfg!(target_os = "macos") { 15 } else { 0x52 }),
            "KeyS" => Some(if cfg!(target_os = "macos") { 1 } else { 0x53 }),
            "KeyT" => Some(if cfg!(target_os = "macos") { 17 } else { 0x54 }),
            "KeyU" => Some(if cfg!(target_os = "macos") { 32 } else { 0x55 }),
            "KeyV" => Some(if cfg!(target_os = "macos") { 9 } else { 0x56 }),
            "KeyW" => Some(if cfg!(target_os = "macos") { 13 } else { 0x57 }),
            "KeyX" => Some(if cfg!(target_os = "macos") { 7 } else { 0x58 }),
            "KeyY" => Some(if cfg!(target_os = "macos") { 16 } else { 0x59 }),
            "KeyZ" => Some(if cfg!(target_os = "macos") { 6 } else { 0x5A }),
            _ => None,
        };

        if let Some(scancode) = native_code {
            let _ = enigo.raw(scancode, Direction::Press);
            thread::sleep(time::Duration::from_millis(25));
            let _ = enigo.raw(scancode, Direction::Release);
        }
    });
}

#[tauri::command]
pub fn show_updater_window(app_handle: AppHandle) {
    let _ = updater_window::show_updater_window(&app_handle);
}

#[tauri::command]
pub fn set_updater_window_size(app_handle: AppHandle, width: f64, height: f64) {
    let _ = updater_window::set_updater_window_size(&app_handle, width, height);
}

// ── macOS-only accessibility & launch commands ──
#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    use ghost_permissions;
    use macos_accessibility_client::accessibility;

    /// Checks whether the app has been granted Accessibility permissions.
    #[tauri::command]
    pub fn check_accessibility() -> bool {
        accessibility::application_is_trusted()
    }

    /// Prompts the user for Accessibility permissions and opens System Preferences if denied.
    #[tauri::command]
    pub fn request_accessibility() -> bool {
        let is_trusted = accessibility::application_is_trusted_with_prompt();
        is_trusted
    }

    #[tauri::command]
    pub fn check_screen_recording() -> bool {
        ghost_permissions::screen_recording_granted()
    }

    #[tauri::command]
    pub fn request_screen_recording() -> bool {
        let granted = ghost_permissions::screen_recording_granted();
        // ghost_permissions::request_screen_recording()
        // open System Settings "Screen & System Audio Recording" pane
        if !granted {
            open_screen_audio_settings();
        }
        granted
    }

    #[tauri::command]
    pub fn show_permission_window(app_handle: AppHandle) -> Result<(), tauri::Error> {
        permission_window::show_permission_window(&app_handle)
    }

    /// Launches the floating helper application.
    #[tauri::command]
    pub fn launch_helper(app_handle: AppHandle) -> Result<String, String> {
        use crate::sidecar::helper::start_helper;

        if !accessibility::application_is_trusted() {
            return Err("Accessibility permission is required to launch the helper.".into());
        }

        let state = app_handle.state::<HelperProcess>();
        let mut lock = state.0.lock().unwrap();
        if lock.is_some() {
            return Ok("Helper is already running.".into());
        }

        match start_helper() {
            Ok(Some(child)) => {
                *lock = Some(child);
                Ok("Helper started successfully.".into())
            }
            Ok(None) => Err("Helper binary could not be located on disk.".into()),
            Err(e) => Err(format!("Failed to spawn helper process: {}", e)),
        }
    }

    /// Launches the CGEvent monitoring sidecar.
    #[tauri::command]
    pub fn launch_cgevent_server(app_handle: AppHandle) -> Result<String, String> {
        use crate::sidecar::cgevent::start_cgevent_sidecar;

        if !accessibility::application_is_trusted() {
            return Err("Accessibility permission is required to launch the sidecar.".into());
        }

        let state = app_handle.state::<CGEventHelperProcess>();
        let mut lock = state.0.lock().unwrap();
        if lock.is_some() {
            return Ok("CGEvent sidecar is already running.".into());
        }

        match start_cgevent_sidecar(&app_handle) {
            Ok(Some(child)) => {
                *lock = Some(child);
                Ok("CGEvent sidecar started successfully.".into())
            }
            Ok(None) => Err("Sidecar binary could not be located on disk.".into()),
            Err(e) => Err(format!("Failed to spawn sidecar process: {}", e)),
        }
    }

    /// Open System Settings > Privacy & Security > Screen & System Audio Recording
    fn open_screen_audio_settings() {
        // URL scheme for Screen & System Audio Recording pane (macOS Ventura+)
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
    }
}

#[cfg(target_os = "macos")]
pub use macos_impl::*;
