use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fstdict_common::window::state::{create_debounced_saver, WindowState};

use log::{info, warn};
use tauri::{App, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_decorum::WebviewWindowExt;
use window_vibrancy::{apply_blur, apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

/// Delay before arming the state tracker after window creation (milliseconds).
const TRACKER_ARM_DELAY_MS: u64 = 500;

/// Creates the main application window with saved state restoration.
pub fn setup_main_window(app: &mut App) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();
    let config_file = "main-window-state.json";
    let state = WindowState::load(&app_handle, config_file);
    let main_url = WebviewUrl::App("#/dict/1".into());

    let mut builder = WebviewWindowBuilder::new(app, "main", main_url)
        .title("FstDict")
        .inner_size(state.width, state.height)
        .min_inner_size(400.0, 300.0)
        .transparent(true)
        .accept_first_mouse(true);

    // Platform-specific window builder configuration
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);
    }

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
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        main_win.create_overlay_titlebar()?;
    }

    #[cfg(target_os = "windows")]
    apply_blur(&main_win, Some((18, 18, 18, 125)))
        .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

    #[cfg(target_os = "macos")]
    {
        main_win.set_traffic_lights_inset(25.0, 30.0)?;
        apply_vibrancy(
            &main_win,
            NSVisualEffectMaterial::Sidebar,
            // NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            // Some(NSVisualEffectState::FollowsWindowActiveState),
            None,
        )
        .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
        // Make window transparent without privateApi
        // main_win.make_transparent().unwrap();
    }

    if state.maximized {
        let _ = main_win.maximize();
    }

    // Suppress state saves during initial window layout
    let is_ready = Arc::new(AtomicBool::new(false));
    let task_counter = Arc::new(Mutex::new(0u64));
    let config_name: Arc<str> = Arc::from(config_file);
    let save_trigger = create_debounced_saver(
        main_win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        Arc::clone(&task_counter),
        Arc::clone(&config_name),
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
