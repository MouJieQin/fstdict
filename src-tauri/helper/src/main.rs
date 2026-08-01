#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, LogicalPosition, Manager, Position, Theme, WebviewUrl, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
    TrackingAreaOptions, WebviewWindowExt,
};
// 1. Ensure you import MouseButton and MouseButtonState along with TrayIconEvent
use tauri::menu::{Menu, MenuItem};

tauri_panel! {
    panel!(FloatSearchPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
        with: {
            tracking_area: {
                options: TrackingAreaOptions::new()
                    .active_always()
                    .mouse_entered_and_exited()
                    .mouse_moved(),
                auto_resize: true
            }
        }
    })

    panel_event!(MyPanelEventHandler {
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

#[tauri::command]
fn show_panel(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if let Ok(panel) = app.get_webview_panel("float-search") {
        panel.show_and_make_key();
        return Ok(());
    }

    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let panel = PanelBuilder::<_, FloatSearchPanel>::new(&app, "float-search")
        .url(WebviewUrl::External(parsed))
        .with_window(
            |window| {
                window
                    .hidden_title(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .accept_first_mouse(true)
                    .always_on_top(true)
            }, // .theme(Some(Theme::Dark))
        )
        .build()
        .map_err(|e| e.to_string())?;

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            // .can_join_all_spaces()
            .into(),
    );

    panel.show_and_make_key();
    Ok(())
}

#[tauri::command]
fn hide_panel(app: tauri::AppHandle) {
    if let Ok(panel) = app.get_webview_panel("float-search") {
        panel.hide();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![show_panel, hide_panel])
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let quit_item = MenuItem::with_id(app, "quit", "Exit FstDict", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("FstDict Helper")
                .menu(&tray_menu)
                // ADDED: Prevent the menu from automatically opening on a regular Left-Click
                // This preserves Left-Click for your toggle UI and keeps Right-Click for the context menu.
                .show_menu_on_left_click(false)
                .on_menu_event(|app_handle, event| {
                    if event.id.as_ref() == "quit" {
                        app_handle.exit(0);
                    }
                })
                .on_tray_icon_event(|tray_handle, event| {
                    // 2. MODIFIED: Explicitly filter out Left-Clicks only
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray_handle.app_handle();

                        if let Ok(panel) = app_handle.get_webview_panel("main") {
                            panel.show_and_make_key();
                        }
                        if let Ok(panel) = app_handle.get_webview_panel("float-search") {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    panel.hide();
                                } else {
                                    panel.show_and_make_key();
                                }
                            }
                        } else {
                            let _ = show_panel(
                                app_handle.clone(),
                                "tauri://localhost/#/dict/95?env=floating_tauri".to_string()
                            );
                        }
                    }
                    // Right-clicks and two-finger clicks are ignored here,
                    // allowing macOS to cleanly fall back and render your context .menu() layout.
                })
                .build(app)?;

            let _ = init(app.app_handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("helper app failed to start");
}

fn init(app_handle: &AppHandle) -> Result<(), String> {
    let window: WebviewWindow = app_handle.get_webview_window("main").unwrap();
    let _ = window.set_always_on_top(true);
    let url = "tauri://localhost/#/dict/39?env=floating_tauri".to_string();
    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let _ = window.navigate(parsed);

    let panel = window.to_panel::<FloatSearchPanel>().unwrap();
    let handler: Retained<MyPanelEventHandler> = MyPanelEventHandler::new();
    let handle = app_handle.to_owned();

    handler.window_did_become_key(move |_notification| {
        let app_name = handle.package_info().name.to_owned();
        println!("[info]: {:?} panel becomes key window!", app_name);
    });

    handler.window_did_resign_key(|_notification| {
        println!("[info]: panel resigned from key window!");
    });

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );

    panel.set_floating_panel(true);
    panel.set_event_handler(Some(handler.as_ref()));
    // Convert your raw coordinates into a Tauri LogicalPosition wrapper
    // let coordinates = LogicalPosition::new(0.0, 0.0);

    // Pass the coordinates wrapped inside the Position enum structure
    // panel
    //     .to_window()
    //     .unwrap()
    //     .set_position(Position::Logical(coordinates))
    //     .unwrap();
    panel.show_and_make_key();
    Ok(())
}
