use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fstdict_common::window_state::WindowState;
use log::{info, warn};
use tauri::{App, AppHandle, WebviewWindow, WebviewWindowBuilder, WindowEvent};

/// Debounce delay for saving window position/size (milliseconds).
const STATE_SAVE_DEBOUNCE_MS: u64 = 350;

/// Delay before arming the state tracker after window creation (milliseconds).
const TRACKER_ARM_DELAY_MS: u64 = 500;

/// Creates the main application window with saved state restoration.
pub fn setup_main_window(app: &mut App) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();
    let config_file = "main-window-state.json";
    let state = WindowState::load(&app_handle, config_file);

    #[cfg(not(dev))]
    let main_url = "tauri://localhost/#/dict/1";
    #[cfg(dev)]
    let main_url = "http://localhost:9595/#/dict/1";

    // Platform-specific window builder configuration
    #[cfg(target_os = "macos")]
    let mut builder =
        WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(main_url.into()))
            .title("main")
            .hidden_title(true)
            .inner_size(state.width, state.height)
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);

    #[cfg(not(target_os = "macos"))]
    let mut builder =
        WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(main_url.into()))
            .title("main")
            .inner_size(state.width, state.height)
            .accept_first_mouse(true);

    // Restore saved position if still within visible screen bounds
    if let (Some(x), Some(y)) = (state.x, state.y) {
        if WindowState::is_position_visible(&app_handle, x, y, state.width, state.height) {
            builder = builder.position(x, y);
            info!("Restoring main window position to ({}, {})", x, y);
        } else {
            builder = builder.center();
            warn!(
                "Saved main window position ({}, {}) is off-screen. Centering instead.",
                x, y
            );
        }
    } else {
        builder = builder.center();
        info!("No saved main window position. Centering window.");
    }

    let main_win = builder.build()?;

    if state.maximized {
        let _ = main_win.maximize();
    }

    // Suppress state saves during initial window layout
    let is_ready = Arc::new(AtomicBool::new(false));
    let save_trigger = create_debounced_saver(
        main_win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        config_file,
    );

    // Attach window event listeners
    main_win.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => save_trigger(),
        _ => {}
    });

    // Arm the state tracker after layout has settled
    let ready_flag = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(TRACKER_ARM_DELAY_MS)).await;
        ready_flag.store(true, Ordering::Relaxed);
    });

    Ok(())
}

/// Creates a debounced closure that saves window state to disk.
///
/// Multiple rapid calls within the debounce window are collapsed into one write.
fn create_debounced_saver(
    win: WebviewWindow,
    app_handle: AppHandle,
    is_ready: Arc<AtomicBool>,
    config_filename: &'static str,
) -> impl Fn() + Clone {
    let task_counter = Arc::new(Mutex::new(0u64));

    move || {
        if !is_ready.load(Ordering::Relaxed) {
            return;
        }

        let counter = Arc::clone(&task_counter);
        let win_clone = win.clone();
        let ah_clone = app_handle.clone();

        tauri::async_runtime::spawn(async move {
            let current_id = {
                let mut guard = counter.lock().unwrap();
                *guard += 1;
                *guard
            };

            tokio::time::sleep(Duration::from_millis(STATE_SAVE_DEBOUNCE_MS)).await;

            let latest_id = {
                let guard = counter.lock().unwrap();
                *guard
            };

            // Only persist if no newer changes were queued during the wait
            if current_id == latest_id {
                let current_state = WindowState::from_window(&win_clone);
                current_state.save(&ah_clone, config_filename);
            }
        });
    }
}
