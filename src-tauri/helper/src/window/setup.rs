use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::panels::{FloatSearchPanel, PublicPanelEventHandler};
use fstdict_common::window::state::{create_debounced_saver, WindowState};
use log::{info, warn};
use tauri::{App, WebviewUrl, WebviewWindowBuilder};
// Removed the redundant tauri_nspanel::objc2::rc::Retained import
use tauri_nspanel::{CollectionBehavior, PanelLevel, WebviewWindowExt};

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
    let panels = [
        PanelConfig {
            label: "helper-main",
            config_filename: "helper-main-window-state.json".to_string(),
            url: "tauri://localhost/#/dict/39?env=helper_main_tauri".to_string(),
        },
        PanelConfig {
            label: "selection-float-search",
            config_filename: "helper-selection-window-state.json".to_string(),
            url: "tauri://localhost/#/dict/95?env=selection_float_search".to_string(),
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
            .title_bar_style(tauri::TitleBarStyle::Overlay);

    // Restore saved position if it's still on-screen
    if let (Some(x), Some(y)) = (state.x, state.y) {
        if WindowState::is_position_visible(&app_handle, x, y, state.width, state.height) {
            builder = builder.position(x, y);
            info!(
                "Restoring {} window position to ({}, {})",
                config.label, x, y
            );
        } else {
            builder = builder.center();
            warn!(
                "Saved {} position ({}, {}) is off-screen. Centering.",
                config.label, x, y
            );
        }
    } else {
        builder = builder.center();
        info!("No saved position for {}. Centering window.", config.label);
    }

    let win = builder.build()?;
    let _ = win.hide();

    // Suppress state saving during initial layout
    let is_ready = Arc::new(AtomicBool::new(false));
    let task_counter = Arc::new(Mutex::new(0u64));

    let save_trigger = create_debounced_saver(
        win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        Arc::clone(&task_counter),
        Arc::clone(&config_name),
    );

    // Arm the tracker after layout settles
    let ready_flag = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(TRACKER_ARM_DELAY_MS)).await;
        ready_flag.store(true, Ordering::Relaxed);
    });

    // Convert to NSPanel and attach event handlers
    let panel = win
        .to_panel::<FloatSearchPanel>()
        .expect("Failed to convert window to NSPanel");

    let handler = PublicPanelEventHandler::new();

    let move_trigger = save_trigger.clone();
    handler.window_did_move(move |_| {
        move_trigger();
    });

    let resize_trigger = save_trigger;
    handler.window_did_resize(move |_| {
        resize_trigger();
    });

    let handle_clone = app_handle.clone();
    handler.window_did_become_key(move |_| {
        info!(
            "{} panel became key window",
            handle_clone.package_info().name
        );
    });

    handler.window_did_resign_key(|_| {
        info!("Panel resigned key window status");
    });

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            // .can_join_all_spaces()
            .into(),
    );

    // ✨ Core Fix: Consume the pre-cast public delegate reference seamlessly
    // This completely removes all "private type" errors from setup.rs
    panel.set_event_handler(Some(handler.as_protocol_delegate()));
    Ok(())
}
