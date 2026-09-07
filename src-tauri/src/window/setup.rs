use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::WindowEvent;

use crate::commands;
use fstdict_common::window::state::{create_debounced_saver, WindowState};
use log::{info, warn};
use tauri::{App, WebviewUrl, WebviewWindowBuilder};

/// Delay before arming the state tracker after window creation (milliseconds).
const TRACKER_ARM_DELAY_MS: u64 = 500;

/// Configuration for a float search panel window.
pub struct PanelConfig {
    pub label: &'static str,
    pub config_filename: String,
    pub url: String,
}

/// Creates and configures both floating search panels.
pub fn setup_float_panels(app: &mut App) -> Result<(), tauri::Error> {
    #[cfg(not(dev))]
    let base_url = "tauri://localhost";
    #[cfg(dev)]
    let base_url = "http://localhost:9595";
    let panels = [
        PanelConfig {
            label: "helper-main",
            config_filename: "helper-main-window-state.json".to_string(),
            url: format!("{}/#/dict/39?env=helper_main_tauri", base_url),
        },
        PanelConfig {
            label: "selection-float-search",
            config_filename: "helper-selection-window-state.json".to_string(),
            url: format!("{}/#/dict/95?env=selection_float_search", base_url),
        },
    ];

    for config in panels {
        setup_panel(app, config)?;
    }

    Ok(())
}

fn setup_panel(app: &mut App, config: PanelConfig) -> Result<(), tauri::Error> {
    let app_handle = app.handle().clone();
    let config_name: Arc<str> = Arc::from(config.config_filename);
    let state = WindowState::load(&app_handle, &config_name);

    let mut builder =
        WebviewWindowBuilder::new(app, config.label, WebviewUrl::App(config.url.into()))
            .hidden_title(true)
            .inner_size(state.width, state.height)
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);

    // Platform-specific window builder configuration
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .title_bar_style(tauri::TitleBarStyle::Transparent);
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

    let win = builder.build()?;
    let _ = win.hide();

    // Suppress state saves during initial window layout
    let is_ready = Arc::new(AtomicBool::new(false));
    let task_counter = Arc::new(Mutex::new(0u64));
    let save_trigger = create_debounced_saver(
        win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        Arc::clone(&task_counter),
        Arc::clone(&config_name),
    );

    // Attach window event listeners
    win.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => save_trigger(),
        WindowEvent::Focused(focused) => {
            if !focused {
                info!("window lost focus");
                let _ = commands::hide_window_if_unpinned_and_outside(&app_handle, config.label);
            } else {
                info!("window gained focus");
            }
        }
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
