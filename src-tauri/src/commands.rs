#[cfg(target_os = "macos")]
use crate::window::permission_window;
use crate::window::updater_window;
use tauri::{AppHandle, Manager};

use fstdict_common::theme::set_app_theme;

#[cfg(target_os = "macos")]
use crate::app_state::HelperProcess;

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

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
mod non_macos_impl {
    use super::*;
    use crate::app_state::{HelperMainWindowPinState, HelperSelectionWindowPinState};
    use crate::globalevent::listener;
    use fstdict_common::window::positioning::{is_cursor_over_window, position_window_near_cursor};
    use log::info;
    use tauri::State;

    /// Tauri command: update the pin state of the selection search panel.
    #[tauri::command]
    pub fn set_selection_window_pinned(
        state: State<'_, HelperSelectionWindowPinState>,
        pinned: bool,
    ) {
        state.set_pinned(pinned);
        info!("Selection window pin state updated to: {}", pinned);
    }

    /// Tauri command: update the pin state of the main helper panel.
    #[tauri::command]
    pub fn set_main_window_pinned(state: State<'_, HelperMainWindowPinState>, pinned: bool) {
        state.set_pinned(pinned);
        info!("Main window pin state updated to: {}", pinned);
    }

    /// Shows the selection panel near the cursor (unless pinned).
    pub fn show_selection_panel(app: &AppHandle) -> Result<(), String> {
        let Some(win) = app.get_webview_window("helper-selection") else {
            return Ok(());
        };

        if let Some(pin_state) = app.try_state::<HelperSelectionWindowPinState>() {
            if pin_state.is_pinned() {
                let _ = win.show();
                return Ok(());
            }
        }

        let _ = position_window_near_cursor(app, &win);
        let _ = win.show();
        listener::enable_helper_selection_hide();

        Ok(())
    }

    /// Shows the main helper panel near the cursor (unless pinned).
    pub fn show_main_panel(app: &AppHandle) -> Result<(), String> {
        let Some(win) = app.get_webview_window("helper-main") else {
            return Ok(());
        };

        if let Some(pin_state) = app.try_state::<HelperMainWindowPinState>() {
            if pin_state.is_pinned() {
                let _ = win.show();
                return Ok(());
            }
        }

        let _ = position_window_near_cursor(app, &win);
        listener::enable_helper_main_hide();
        let _ = win.show();
        Ok(())
    }

    /// Hides a window if the cursor is outside its bounds and it's not pinned.
    ///
    /// Returns `true` if the window was hidden or was already hidden.
    pub fn hide_window_if_unpinned_and_outside(app: &AppHandle, label: &str) -> bool {
        let is_pinned = match label {
            "helper-main" => app
                .try_state::<HelperMainWindowPinState>()
                .map(|s| s.is_pinned())
                .unwrap_or(false),
            "helper-selection" => app
                .try_state::<HelperSelectionWindowPinState>()
                .map(|s| s.is_pinned())
                .unwrap_or(false),
            _ => false,
        };

        if is_pinned {
            disable_listener(label);
            return false;
        }

        let Some(win) = app.get_webview_window(label) else {
            disable_listener(label);
            return true;
        };

        if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
            disable_listener(label);
            return true;
        }

        if !is_cursor_over_window(app, label) {
            let _ = win.hide();
            disable_listener(label);
            return true;
        }

        false
    }

    fn disable_listener(label: &str) {
        if label == "helper-selection" {
            listener::disable_helper_selection_hide();
        } else if label == "helper-main" {
            listener::disable_helper_main_hide();
        }
    }
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub use non_macos_impl::*;
