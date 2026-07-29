#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow};
// use tauri_nspanel::{tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
    TrackingAreaOptions, WebviewWindowExt,
};

tauri_panel! {
    panel!(FloatSearchPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
        with: {
            // Enable mouse tracking for the panel's content view
            // This allows the panel to receive mouse events even when not key/active
            tracking_area: {
                options: TrackingAreaOptions::new()
                    .active_always()           // Track mouse even when app is not active
                    .mouse_entered_and_exited() // Get notified when mouse enters/exits
                    .mouse_moved(),             // Track mouse movement
                auto_resize: true               // Resize tracking area with window
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
        panel.show();
        return Ok(());
    }

    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let panel = PanelBuilder::<_, FloatSearchPanel>::new(&app, "float-search")
        .url(WebviewUrl::External(parsed))
        .with_window(|window| {
            window
                // .min_inner_size(300.0, 200.0)
                // .max_inner_size(800.0, 600.0)
                // .resizable(false)
                // .decorations(false)
                // .title_bar_style(TitleBarStyle::Overlay)
                .always_on_top(true)
            // .visible_on_all_workspaces(true)
            // .skip_taskbar(true)
        })
        // .level(PanelLevel::ModalPanel)
        // .size(600.0, 400.0)
        // .decorations(false)
        // .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Set the window to float level
    panel.set_level(PanelLevel::ModalPanel.value());

    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            // .can_join_all_spaces()
            .into(),
    );

    panel.show();
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
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![show_panel, hide_panel])
        .setup(|app| {
            // Helper 全程 Accessory，无 Dock 图标，天然支持全屏覆盖
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let _ = init(app.app_handle());

            // if let Err(e) = show_panel(app.handle().clone(), "http://localhost:9595/#/dict/39".to_string())
            // {
            //     eprintln!("Failed to show panel: {}", e);
            // }
            // 可在此处添加菜单栏图标（System Tray）
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("helper app failed to start");
}

fn init(app_handle: &AppHandle) -> Result<(), String> {
    let window: WebviewWindow = app_handle.get_webview_window("main").unwrap();
    let url = "http://localhost:9595/#/dict/39".to_string();
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

    // Set the window to float level
    panel.set_level(PanelLevel::Floating.value());

    // Ensures the panel cannot activate the app
    // panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());

    // Allows the panel to:
    // - display on the same space as the full screen window
    // - join all spaces
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            // .can_join_all_spaces()
            .into(),
    );

    panel.set_event_handler(Some(handler.as_ref()));
    Ok(())
    // Note: The tracking area is configured in the panel definition above.
    // Mouse events (mouseEntered, mouseExited, mouseMoved) will be sent to the
    // panel's content view. To handle these events, you would need to:
    // 1. Create a custom NSView subclass that overrides these methods
    // 2. Use JavaScript in your webview to listen for mouse events
    // 3. Or use Tauri's event system to communicate mouse positions
}
