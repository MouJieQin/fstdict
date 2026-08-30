use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show_permission_window(app: &AppHandle) -> Result<(), tauri::Error> {
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
