#![cfg(target_os = "macos")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_state;
mod panels;
mod tray;
mod websocket;
mod window;

use std::fs;
use std::path::PathBuf;

use fstdict_common::logger::init_logging;
use tauri::{ActivationPolicy, Manager};
use tokio::sync::mpsc;

use app_state::{MainWindowPinState, MainWindowWsSender, SelectionWindowPinState};
use tray::setup_tray;
use websocket::client::start_cgevent_ws_client;
use window::setup::setup_float_panels;

/// WebSocket server endpoint for the CGEvent helper service.
const WS_ENDPOINT: &str = "ws://127.0.0.1:5959/ws/fstdict/helper";

/// Size of the bounded MPSC channel for outbound WebSocket messages.
const WS_CHANNEL_CAPACITY: usize = 32;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![
            window::commands::set_selection_window_pinned,
            window::commands::set_main_window_pinned,
            window::commands::hide_panel,
            window::commands::trigger_notification
        ])
        .setup(|app| {
            // Run as accessory (dockless) application on macOS
            app.set_activation_policy(ActivationPolicy::Accessory);

            // Ensure application data directories exist
            let app_config_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory")
                .join("Storage/config");
            fs::create_dir_all(&app_config_dir)?;

            // Initialize logging subsystem
            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("./logs"));
            init_logging(&log_dir, "fstdict-helper".to_string());

            // ── WebSocket client setup ──
            let (main_tx, main_rx) = mpsc::channel::<String>(WS_CHANNEL_CAPACITY);
            let (selection_tx, selection_rx) = mpsc::channel::<String>(WS_CHANNEL_CAPACITY);
            let (main_pin_tx, main_pin_rx) = mpsc::channel::<String>(WS_CHANNEL_CAPACITY);

            app.manage(SelectionWindowPinState::new(selection_tx));
            app.manage(MainWindowPinState::new(main_pin_tx));
            app.manage(MainWindowWsSender::new(main_tx));

            let app_handle = app.handle().clone();
            let ws_url = WS_ENDPOINT.to_string();
            tokio::spawn(async move {
                start_cgevent_ws_client(&ws_url, app_handle, main_rx, main_pin_rx, selection_rx)
                    .await;
            });

            // ── Create floating panel windows ──
            setup_float_panels(app)?;

            // ── System tray icon ──
            setup_tray(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("FstDict Helper application failed to start");
}
