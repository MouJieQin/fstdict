use crate::window::updater_window;
use fstdict_common::theme::set_app_theme;
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
}

#[cfg(target_os = "macos")]
pub use macos_impl::*;
