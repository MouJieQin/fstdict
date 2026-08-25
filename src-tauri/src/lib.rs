mod app_state;
mod commands;
mod shortcuts;
mod sidecar;
mod theme;
mod updater;
mod websocket;
mod window;

use std::fs;
use std::path::PathBuf;

use fstdict_common::logger::init_logging;
use log::{debug, error, info, warn};
use tauri::{Manager, RunEvent};
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
use app_state::{
    CGEventHelperProcess, HelperProcess, GLOBAL_CGEVENT_SERVER, GLOBAL_HELPER_PROCESS,
};
use app_state::{DoubleCopyTracker, MainWindowWsSender, PythonServer, GLOBAL_PYTHON_SERVER};
use ctrlc;
use shortcuts::global::register_global_shortcuts;
use sidecar::common::terminate_child_process;
use sidecar::python::start_python_sidecar;
use websocket::client::start_ws_client;
use window::main_window::setup_main_window;

const WS_ENDPOINT: &str = "ws://127.0.0.1:5959/ws/fstdict/main";
/// Size of the bounded MPSC channel for outbound WebSocket messages.
const WS_CHANNEL_CAPACITY: usize = 32;

/// Performs graceful termination of all sidecar processes.
///
/// Triggered in two scenarios:
/// 1. Normal application shutdown (window close, Cmd+Q) via Tauri RunEvent
/// 2. Terminal interruption (Ctrl+C / SIGINT / SIGTERM) via OS signal handler
pub fn cleanup_all_sidecars() {
    info!("Initiating global sidecar cleanup");

    // Terminate Python backend server
    if let Some(handle) = GLOBAL_PYTHON_SERVER.get() {
        if let Ok(mut guard) = handle.lock() {
            terminate_child_process(&mut guard, "Python server");
        }
    }

    // Terminate macOS-specific helper processes
    #[cfg(target_os = "macos")]
    {
        if let Some(handle) = GLOBAL_HELPER_PROCESS.get() {
            if let Ok(mut guard) = handle.lock() {
                terminate_child_process(&mut guard, "fstdict-helper");
            }
        }

        if let Some(handle) = GLOBAL_CGEVENT_SERVER.get() {
            if let Ok(mut guard) = handle.lock() {
                terminate_child_process(&mut guard, "CGEvent server");
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // Register OS signal handler for graceful termination on Ctrl+C
    // Ensures sidecars are reaped when `tauri dev` is killed from the terminal
    #[cfg(not(mobile))]
    ctrlc::set_handler(move || {
        info!("Received SIGINT/SIGTERM signal, cleaning up sidecars");
        cleanup_all_sidecars();
        std::process::exit(130); // Standard exit code for SIGINT (128 + 2)
    })
    .expect("Failed to register termination signal handler");

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    shortcuts::global::handle_shortcut_event(app, shortcut, event);
                })
                .build(),
        )
        // Single invoke_handler call with all commands (fixes overwrite bug)
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::set_theme,
            commands::show_updater_window,
            #[cfg(target_os = "macos")]
            commands::check_accessibility,
            #[cfg(target_os = "macos")]
            commands::request_accessibility,
            #[cfg(target_os = "macos")]
            commands::launch_helper,
            #[cfg(target_os = "macos")]
            commands::launch_cgevent_server
        ])
        .manage(DoubleCopyTracker::default())
        .manage(PythonServer::default());

    // Register macOS-only state
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .manage(CGEventHelperProcess::default())
            .manage(HelperProcess::default());
    }

    let app = builder
        .setup(|app| {
            // Initialize logging subsystem
            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("./logs"));
            init_logging(&log_dir, "fstdict-main".to_string());

            // Share sidecar state handles with global registry for signal handler access
            let _ = GLOBAL_PYTHON_SERVER.set(app.state::<PythonServer>().0.clone());
            #[cfg(target_os = "macos")]
            {
                let _ = GLOBAL_HELPER_PROCESS.set(app.state::<HelperProcess>().0.clone());
                let _ = GLOBAL_CGEVENT_SERVER.set(app.state::<CGEventHelperProcess>().0.clone());
            }

            // Ensure application data directory exists
            let app_data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_dir)?;
            info!("Application data directory: {:?}", app_data_dir);

            let app_handle = app.handle().clone();

            // Create and configure main window
            setup_main_window(app)?;

            // Register system-wide keyboard shortcuts
            register_global_shortcuts(app);

            // Start Python backend sidecar (skipped in dev mode)
            match start_python_sidecar(app) {
                Ok(Some(child)) => {
                    *app.state::<PythonServer>().0.lock().unwrap() = Some(child);
                }
                Ok(None) => {
                    debug!("Python sidecar not started (dev mode or binary not found)");
                }
                Err(e) => {
                    error!("Failed to start Python server: {}", e);
                    return Err(e);
                }
            }

            // ── WebSocket client setup ──
            let (main_tx, main_rx) = mpsc::channel::<String>(WS_CHANNEL_CAPACITY);
            app.manage(MainWindowWsSender::new(main_tx));
            let ws_url = WS_ENDPOINT.to_string();
            tokio::spawn(async move {
                start_ws_client(&ws_url, app_handle, main_rx).await;
            });

            // Start macOS-specific helper processes
            #[cfg(target_os = "macos")]
            {
                use macos_accessibility_client::accessibility::application_is_trusted;
                use sidecar::{cgevent::start_cgevent_sidecar, helper::start_helper};

                if application_is_trusted() {
                    // Start CGEvent server
                    match start_cgevent_sidecar(app.handle()) {
                        Ok(Some(child)) => {
                            *app.state::<CGEventHelperProcess>().0.lock().unwrap() = Some(child);
                        }
                        Ok(None) => warn!("CGEvent server sidecar binary not found at startup"),
                        Err(e) => {
                            error!("Failed to start CGEvent server: {}", e);
                            return Err(e);
                        }
                    }

                    // Start floating helper app
                    match start_helper() {
                        Ok(Some(child)) => {
                            *app.state::<HelperProcess>().0.lock().unwrap() = Some(child);
                        }
                        Ok(None) => warn!("Helper binary not found at startup"),
                        Err(e) => error!("Failed to start helper at launch: {}", e),
                    }
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application");

    // Application event loop with graceful shutdown
    app.run(|app_handle, event| {
        match event {
            // Handle both normal close and forced termination (Cmd+Q on macOS)
            RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                // Terminate Python server
                if let Ok(mut guard) = app_handle.state::<PythonServer>().0.lock() {
                    terminate_child_process(&mut guard, "Python server");
                }

                // Terminate macOS helper processes
                #[cfg(target_os = "macos")]
                {
                    if let Ok(mut guard) = app_handle.state::<HelperProcess>().0.lock() {
                        terminate_child_process(&mut guard, "fstdict-helper");
                    }
                    if let Ok(mut guard) = app_handle.state::<CGEventHelperProcess>().0.lock() {
                        terminate_child_process(&mut guard, "CGEvent server");
                    }
                }
            }
            _ => {}
        }
    });
}
