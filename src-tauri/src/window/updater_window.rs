use tauri::{AppHandle, Manager, Size, WebviewUrl, WebviewWindowBuilder};

pub fn show_updater_window(app: &AppHandle) -> Result<(), tauri::Error> {
    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window("updater") {
        let _ = win.show();
        return Ok(());
    }
    create_updater_window(app)
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
    #[cfg(not(dev))]
    let updater_url = "tauri://localhost/#/updater";
    #[cfg(dev)]
    let updater_url = "http://localhost:9595/#/updater";

    let win = WebviewWindowBuilder::new(app, "updater", WebviewUrl::App(updater_url.into()))
        .inner_size(360.0, 180.0)
        .resizable(false)
        .center()
        .title("Updater")
        // .closable(false)
        .build()?;

    let _ = win.show();
    Ok(())
}
