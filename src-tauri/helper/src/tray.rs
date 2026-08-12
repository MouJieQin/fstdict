use tauri::menu::{Menu, MenuItem};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager,
};

/// Sets up the system tray icon with a simple quit menu.
pub fn setup_tray(app: &mut App) -> Result<(), tauri::Error> {
    let quit_item = MenuItem::with_id(app, "quit", "Exit FstDict", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("FstDict Helper")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| {
            if event.id.as_ref() == "quit" {
                app_handle.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("helper-main") {
                    let _ = w.show();
                }
            }
        })
        .build(app)?;

    Ok(())
}
