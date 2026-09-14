#![cfg(target_os = "macos")]
use crate::commands;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show_permission_window(app: &AppHandle) -> Result<(), tauri::Error> {
    reset_and_request_permissions();
    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window("permission") {
        let _ = win.show();
        return Ok(());
    }
    create_permission_window(app)
}

fn create_permission_window(app: &AppHandle) -> Result<(), tauri::Error> {
    #[cfg(not(dev))]
    let updater_url = "tauri://localhost/#/permission";
    #[cfg(dev)]
    let updater_url = "http://localhost:9595/#/permission";

    let win = WebviewWindowBuilder::new(app, "permission", WebviewUrl::App(updater_url.into()))
        .inner_size(400.0, 400.0)
        .resizable(false)
        .minimizable(false)
        .center()
        .title("Permission")
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .build()?;
    let _ = win.show();
    Ok(())
}

/// Auto clean stale TCC permission entry and re-request
fn reset_and_request_permissions() {
    use std::process::Command;
    // Replace with your fixed bundle id
    let bundle_id = "FstDict";
    // Reset Accessibility & ScreenCapture
    if !commands::check_accessibility() {
        let _ = Command::new("tccutil")
            .args(["reset", "Accessibility", bundle_id])
            .status();
    }
    if !commands::check_screen_recording() {
        let _ = Command::new("tccutil")
            .args(["reset", "ScreenCapture", bundle_id])
            .status();
    }
}
