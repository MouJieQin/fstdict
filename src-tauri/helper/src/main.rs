#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use fstdict_common::logger::init_logging;
use fstdict_common::window_state::WindowState;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    App, AppHandle, Emitter, LogicalPosition, Manager, Position, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
    TrackingAreaOptions, WebviewWindowExt,
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
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

    panel!(NotificationPanel {
        config: {
            // allows it to interact while being a background element
            can_become_key_window: false,
            is_floating_panel: true
        }
    })

    panel_event!(MyPanelEventHandler {
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

// Use a global or state-managed counter to manage debouncing tasks across commands safely.
// You can also add this to your app state via `.manage(NotificationState::default())`.

static CURRENT_TASK_ID: AtomicU64 = AtomicU64::new(0);
static PANEL_IS_CREATING: AtomicBool = AtomicBool::new(false);

fn set_noti_pannel_position(app: AppHandle, w: &WebviewWindow) -> Result<(), String> {
    // Position window at the top-right corner of the monitor with user cursor focus
    if let Ok(cursor_pos) = app.cursor_position() {
        // println!(
        //     "[info]: Cursor position: ({}, {})",
        //     cursor_pos.x, cursor_pos.y
        // );
        if let Some(monitor) = app
            .available_monitors()
            .unwrap_or_default()
            .into_iter()
            .find(|m| {
                let m_pos = m.position();
                let m_size = m.size();
                // println!(
                //     "m_pos:({}, {}), m_size:({}, {})",
                //     m_pos.x, m_pos.y, m_size.width, m_size.height
                // );
                cursor_pos.x >= m_pos.x as f64
                    && cursor_pos.x <= (m_pos.x as f64 + m_size.width as f64)
                    && cursor_pos.y >= m_pos.y as f64
                    && cursor_pos.y <= (m_pos.y as f64 + m_size.height as f64)
            })
        {
            let scale = monitor.scale_factor();
            let screen_pos = monitor.position().to_logical::<f64>(scale);
            let screen_size = monitor.size().to_logical::<f64>(scale);

            let panel_width = 360.0;
            let edge_padding = 24.0;
            let top_padding = 40.0;

            let target_x = screen_pos.x + screen_size.width - panel_width - edge_padding;
            let target_y = screen_pos.y + top_padding;

            // Set layout position coordinates safely
            let _ = w.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
                target_x, target_y,
            )));
        }
    }
    return Ok(());
}

fn show_notification(app: AppHandle, message: String) -> Result<(), String> {
    let task_id = CURRENT_TASK_ID.fetch_add(1, Ordering::SeqCst) + 1;

    // 1. REUSE PATH: Panel already exists in memory.
    if let Some(w) = app.get_webview_window("notify-layer") {
        // FIX: Force event delivery directly to this specific webview label
        let _ = w.emit_to("notify-layer", "update-message", &message);
        let _ = set_noti_pannel_position(app, &w);
        let _ = w.show();

        // Start fade out timer
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(2000)).await;
            if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
                let _ = w.emit_to("notify-layer", "start-fade-out", ());
            }
            tokio::time::sleep(Duration::from_millis(550)).await;

            if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
                let _ = w.close();
            }
        });

        return Ok(());
    }

    // 2. CONCURRENCY PROTECTION: Prevent rapid double creations
    if PANEL_IS_CREATING.swap(true, Ordering::SeqCst) {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if let Some(w) = app_clone.get_webview_window("notify-layer") {
                    let _ = w.emit_to("notify-layer", "update-message", &message);
                    let _ = set_noti_pannel_position(app, &w);
                    let _ = w.show();
                    break;
                }
            }
        });
        return Ok(());
    }

    // 3. FIRST TIME CREATION PATH
    let encoded_message = urlencoding::encode(&message);
    let target_url = format!("notification.html?message={}", encoded_message);

    let panel_res = tauri_nspanel::PanelBuilder::<_, NotificationPanel>::new(&app, "notify-layer")
        .url(WebviewUrl::App(target_url.into()))
        .with_window(|window| {
            window
                .hidden_title(true)
                .inner_size(360.0, 90.0)
                .accept_first_mouse(true)
                .always_on_top(true)
                .transparent(true)
                .decorations(false)
                .resizable(false)
                .focusable(false)
        })
        .level(tauri_nspanel::PanelLevel::Status)
        .build();

    PANEL_IS_CREATING.store(false, Ordering::SeqCst);

    let panel = match panel_res {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to build panel: {:?}", e)),
    };

    panel.set_collection_behavior(
        tauri_nspanel::CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );

    let w: WebviewWindow = panel.to_window().unwrap().clone();

    let _ = set_noti_pannel_position(app, &w);
    panel.show();

    // Fade out first-time notice after 3 seconds
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2000)).await;
        if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
            let _ = w.emit_to("notify-layer", "start-fade-out", ());
        }
        tokio::time::sleep(Duration::from_millis(550)).await;

        if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
            let _ = w.close();
        }
    });

    Ok(())
}

#[tauri::command]
fn trigger_notification(app: AppHandle, message: String) -> Result<(), String> {
    return show_notification(app, message);
}

#[tauri::command]
fn show_panel(app: tauri::AppHandle, url: String) -> Result<(), String> {
    println!("Receive show_panel");
    if let Ok(panel) = app.get_webview_panel("selection-float-search") {
        println!(" show()");
        panel.show();
        return Ok(());
    }

    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let panel = PanelBuilder::<_, FloatSearchPanel>::new(&app, "selection-float-search")
        .url(WebviewUrl::External(parsed))
        .with_window(
            |window| {
                window
                    .hidden_title(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .accept_first_mouse(true)
                    .always_on_top(true)
                    .minimizable(false)
            }, // .theme(Some(Theme::Dark))
        )
        .build()
        .map_err(|e| e.to_string())?;

    let handle = app.to_owned();
    let handler: Retained<MyPanelEventHandler> = MyPanelEventHandler::new();
    handler.window_did_become_key(move |_notification| {
        let app_name = handle.package_info().name.to_owned();
        println!("[info]: {:?} panel becomes key window!", app_name);
    });

    // let panel_clone = panel.clone();
    handler.window_did_resign_key(move |_notification| {
        // panel_clone.hide();
        println!("[info]: panel resigned from key window!");
    });

    panel.set_floating_panel(true);
    // panel.set_hides_on_deactivate(true);
    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );
    // panel.set_hides_on_deactivate(true);
    panel.set_event_handler(Some(handler.as_ref()));
    Ok(())
}

#[tauri::command]
fn hide_panel(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("selection-float-search") {
        let _ = w.close();
    }
}

fn init(app_handle: &AppHandle) -> Result<(), String> {
    let window: WebviewWindow = app_handle.get_webview_window("main").unwrap();
    let url = "tauri://localhost/#/dict/39?env=floating_tauri".to_string();
    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let _ = window.navigate(parsed);

    let panel = window.to_panel::<FloatSearchPanel>().unwrap();
    let handler: Retained<MyPanelEventHandler> = MyPanelEventHandler::new();
    let handle = app_handle.to_owned();
    // panel.set_released_when_closed(false);

    handler.window_did_become_key(move |_notification| {
        let app_name = handle.package_info().name.to_owned();
        println!("[info]: {:?} panel becomes key window!", app_name);
    });

    handler.window_did_resign_key(|_notification| {
        println!("[info]: panel resigned from key window!");
    });

    // panel.set_level(PanelLevel::ModalPanel.value());
    // panel.set_collection_behavior(
    //     CollectionBehavior::new()
    //         .full_screen_auxiliary()
    //         .can_join_all_spaces()
    //         .into(),
    // );

    // panel.set_floating_panel(true);
    panel.set_event_handler(Some(handler.as_ref()));
    // panel.close();
    // let _ = window.close();
    // panel.hide();

    // let _ = show_panel(
    //     app_handle.clone(),
    //     "tauri://localhost/#/dict/95?env=selection_float_search".to_string(),
    // );

    // let _ = hide_panel(app_handle.clone());
    Ok(())
}

fn main_window_setup(app: &mut App) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();
    let config_filename = "helper-main-window-state.json";
    let state = WindowState::load(&app_handle, config_filename);

    let main_url = "tauri://localhost/#/dict/39?env=floating_tauri";
    let mut win_builder =
        WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(main_url.into()))
            .title("main")
            .hidden_title(true)
            .inner_size(state.width, state.height)
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);

    // Correct coordinate matching boundary check
    if let (Some(x), Some(y)) = (state.x, state.y) {
        if WindowState::is_position_visible(&app_handle, x, y, state.width, state.height) {
            win_builder = win_builder.position(x, y);
            log::info!("Restoring main window position to ({}, {})", x, y);
        } else {
            win_builder = win_builder.center();
            log::warn!(
                "Saved main window position ({}, {}) is off-screen. Centering instead.",
                x,
                y
            );
        }
    } else {
        win_builder = win_builder.center();
        log::info!("No saved main window position. Centering window.");
    }

    let main_win = win_builder.build()?;
    let _ = main_win.hide();
    // ===== Guard to suppress background events during initialization framework setup =====
    let is_ready = Arc::new(AtomicBool::new(false));

    // ===== Tokio Thread-Safe Debouncer Implementation =====
    let task_id = Arc::new(Mutex::new(0u64));
    const DEBOUNCE_MS: u64 = 350;
    // Use a single cohesive event controller to avoid reference move duplication
    let trigger_save = {
        let w = main_win.clone();
        let ah = app_handle.clone();
        let is_ready_clone = Arc::clone(&is_ready);
        move || {
            // Refuse hooks if window structure creation sequencing hasn't finished
            if !is_ready_clone.load(Ordering::Relaxed) {
                return;
            }

            let task_id_clone = Arc::clone(&task_id);
            let w_clone = w.clone();
            let ah_clone = ah.clone();

            tauri::async_runtime::spawn(async move {
                let current_id = {
                    let mut guard = task_id_clone.lock().unwrap();
                    *guard += 1;
                    *guard
                };

                tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;

                let latest_id = {
                    let guard = task_id_clone.lock().unwrap();
                    *guard
                };
                if current_id == latest_id {
                    let current_state = WindowState::from_window(&w_clone);
                    current_state.save(&ah_clone, config_filename);
                }
            });
        }
    };

    main_win.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            trigger_save();
        }
        _ => {}
    });

    // Allow the layout thread to settle, then arm the tracker to safely accept events
    let is_ready_arm = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        is_ready_arm.store(true, Ordering::Relaxed);
    });

    // Convert the window to panel
    let panel = main_win.to_panel::<FloatSearchPanel>().unwrap();
    let handler: Retained<MyPanelEventHandler> = MyPanelEventHandler::new();
    let handle = app_handle.to_owned();
    // panel.set_released_when_closed(false);

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
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum CgEvent {
    #[serde(rename = "handlerEventTextSelection")]
    HandlerEventTextSelection { text_selected: String },

    #[serde(rename = "tauri_notification")]
    TauriNotification { data: TauriNotifyData },
}

#[derive(Debug, Deserialize)]
struct TauriNotifyData {
    message: String,
}

pub async fn start_cgevent_ws_client(ws_url: &str, app_handle: AppHandle) {
    loop {
        println!("connecting to cgevent ws: {}", ws_url);
        match connect_async(ws_url).await {
            Ok((ws_stream, _response)) => {
                println!("ws connected");
                let (mut write, mut read) = ws_stream.split();

                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(WsMessage::Text(text)) => {
                            match serde_json::from_str::<CgEvent>(&text) {
                                Ok(event) => match event {
                                    CgEvent::HandlerEventTextSelection { text_selected } => {
                                        let app_clone = app_handle.clone();
                                        // Dispatch to main thread to safely touch webview window maps
                                        let _ = app_handle.run_on_main_thread(move || {
                                            app_clone
                                                .emit_to("main", "cgevent-select", text_selected)
                                                .ok();
                                        });
                                    }
                                    // ========= SAFE DISPATCH tauri_notification =========
                                    CgEvent::TauriNotification { data } => {
                                        println!("receive tauri_notification: {}", data.message);

                                        let app_clone = app_handle.clone();
                                        // CRITICAL FIX: Safe execution jump directly back onto macOS Thread 0
                                        let _ = app_handle.run_on_main_thread(move || {
                                            if let Err(e) =
                                                show_notification(app_clone, data.message)
                                            {
                                                eprintln!(
                                                    "show_notification main thread call err: {}",
                                                    e
                                                );
                                            }
                                        });
                                    }
                                },
                                Err(e) => eprintln!("json parse error: {} raw={}", e, text),
                            }
                        }
                        Ok(WsMessage::Close(_)) => {
                            println!("ws close");
                            break;
                        }
                        Ok(WsMessage::Binary(_)) => {}
                        Ok(WsMessage::Ping(ping)) => {
                            let _ = write.send(WsMessage::Pong(ping)).await;
                        }
                        Ok(WsMessage::Pong(_)) => {}
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("ws read error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("ws connect failed: {}", e);
            }
        }
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

#[tokio::main]
async fn main() {
    let ws_endpoint = "ws://127.0.0.1:5959/ws/fstdict/helper";

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![
            show_panel,
            hide_panel,
            trigger_notification
        ])
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_config_dir = app
                .path()
                .app_data_dir()
                .expect("get app data dir failed")
                .join("Storage/config");
            fs::create_dir_all(&app_config_dir)?;

            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("./logs"));
            init_logging(&log_dir, "fstdict-helper".to_string());

            // ========== 后台spawn websocket客户端，不阻塞setup ==========
            let app_handle = app.handle().clone();
            let ws_url = ws_endpoint.to_string();
            tokio::spawn(async move {
                start_cgevent_ws_client(&ws_url, app_handle).await;
            });

            main_window_setup(app)?;

            let quit_item = MenuItem::with_id(app, "quit", "Exit FstDict", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("FstDict Helper")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app_handle, event| {
                    if event.id.as_ref() == "quit" {
                        app_handle.exit(0);
                    }
                })
                .on_tray_icon_event(|tray_handle, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray_handle.app_handle();
                        // if let Ok(panel) = app_handle.get_webview_panel("main") {
                        //     panel.show_and_make_key();
                        // }

                        if let Some(w) = app_handle.get_webview_window("main") {
                            let _ = w.show();
                        }
                    }
                })
                .build(app)?;

            // let _ = init(app.app_handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("helper app failed to start");
}
