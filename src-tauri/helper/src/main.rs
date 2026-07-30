#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
    TrackingAreaOptions, WebviewWindowExt,
};
// 1. UPDATE YOUR IMPORTS (Replace tauri_plugin_tray with core tauri modules)
use tauri::image::Image;
use tauri::tray::{TrayIconBuilder, TrayIconEvent}; // Used to safely load raw icon buffers if needed

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
        panel.show_and_make_key(); // Changed to show_and_make_key for better usability
        return Ok(());
    }

    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let panel = PanelBuilder::<_, FloatSearchPanel>::new(&app, "float-search")
        .url(WebviewUrl::External(parsed))
        .with_window(|window| window.always_on_top(true))
        .build()
        .map_err(|e| e.to_string())?;

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces() // Allow it to follow the user across full screen desktops
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

            // 2. BUILD THE TRAY ICON USING NATIVE CORE API
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("FstDict Helper")
                .on_tray_icon_event(|tray_handle, event| {
                    // Match a standard click gesture via core event mappings
                    if let TrayIconEvent::Click { .. } = event {
                        let app_handle = tray_handle.app_handle();

                        if let Ok(panel) = app_handle.get_webview_panel("main") {
                            panel.show_and_make_key(); // Changed to show_and_make_key for better usability
                        }

                        // if let Ok(panel) = app_handle.get_webview_panel("float-search") {
                        //     if let Some(window) = app_handle.get_webview_window("main") {
                        //         if window.is_visible().unwrap_or(false) {
                        //             panel.hide();
                        //         } else {
                        //             panel.show_and_make_key();
                        //         }
                        //     }
                        // }
                    }
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
    let url = "tauri://localhost/#/dict/39".to_string();
    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let _ = window.navigate(parsed);

    let panel = window.to_panel::<FloatSearchPanel>().unwrap();
    let handler = MyPanelEventHandler::new();
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
            .can_join_all_spaces() // Ensures it follows spaces cleanly on Tray interaction
            .into(),
    );

    panel.set_event_handler(Some(handler.as_ref()));

    // Explicitly show it once assets load
    panel.show_and_make_key();
    Ok(())
}
