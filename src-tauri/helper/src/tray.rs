use crate::app_state::MainWindowWsSender;
use crate::websocket::protocol::build_event_request;
use log::{error, info, warn};

use tauri::menu::{Menu, MenuItem};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

/// Sets up the system tray icon with a simple quit menu.
pub fn setup_tray(app: &mut App) -> Result<(), tauri::Error> {
    let show_helper_main_item = MenuItem::with_id(
        app,
        "show_helper_main",
        "Show Helper Main Window",
        true,
        None::<&str>,
    )?;
    let show_helper_selection_item = MenuItem::with_id(
        app,
        "show_helper_selection",
        "Show Helper Selection Window",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Exit FstDict", true, None::<&str>)?;
    let tray_menu = Menu::with_items(
        app,
        &[
            &show_helper_main_item,
            &show_helper_selection_item,
            &quit_item,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("FstDict Helper")
        .menu(&tray_menu)
        // .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| {
            let event_id = event.id.as_ref();
            if event_id == "show_helper_main" {
                show_helper_panel(app_handle, "helper-main");
            } else if event_id == "show_helper_selection" {
                show_helper_panel(app_handle, "selection-float-search");
            } else if event_id == "quit" {
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

fn show_helper_panel(app_handle: &AppHandle, label: &str) {
    if let Some(w) = app_handle.get_webview_window(label) {
        match w.is_visible() {
            Ok(true) => info!("helper-main window already visible"),
            Ok(false) => {}
            Err(e) => {
                warn!("Failed to get window visible state: {}", e);
            }
        }
        let _ = w.show();
        let payload = build_event_request("register_request", "kCGEventLeftMouseDown", label);
        if let Some(ws_state) = app_handle.try_state::<MainWindowWsSender>() {
            if let Err(e) = ws_state.ws_sender.try_send(payload.to_string()) {
                error!(
                    "Failed to send kCGEventLeftMouseDown over WebSocket: {:?}",
                    e
                );
            }
        }
    }
}
