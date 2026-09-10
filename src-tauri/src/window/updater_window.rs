use log::error;
use tauri::{AppHandle, Manager, Size, WebviewUrl, WebviewWindowBuilder};

pub fn show_updater_window(app: &AppHandle) -> Result<(), tauri::Error> {
    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window("updater") {
        let _ = win.show();
        return Ok(());
    }
    let handle = app.clone();
    if let Err(e) = create_updater_window(&handle) {
        error!("failed to create updater window: {e}");
    }
    Ok(())
}

pub fn set_updater_window_size(
    app: &AppHandle,
    width: f64,
    height: f64,
) -> Result<(), tauri::Error> {
    if let Some(win) = app.get_webview_window("updater") {
        let _ = win.set_size(Size::Logical((width, height).into()));
        let _ = win.show();
        return Ok(());
    }
    Ok(())
}

fn create_updater_window(app: &AppHandle) -> Result<(), tauri::Error> {
    let updater_url = WebviewUrl::App("#/updater".into());

    let win = WebviewWindowBuilder::new(app, "updater", updater_url)
        .inner_size(360.0, 180.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .title("Updater")
        .build()?;

    let _ = win.show();
    Ok(())
}
