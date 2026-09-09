use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::commands;
use super::mac_rounded_corners;
use crate::panels::{FloatSearchPanel, PublicPanelEventHandler};
use fstdict_common::window::state::{create_debounced_saver, WindowState};
use log::{debug, info, warn};
use tauri::{App, WebviewUrl, WebviewWindowBuilder};
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
            url: "tauri://localhost/#/dict/39?env=helper_main".to_string(),
        },
        PanelConfig {
            label: "helper-selection",
            config_filename: "helper-selection-window-state.json".to_string(),
            url: "tauri://localhost/#/dict/95?env=helper_selection".to_string(),
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
            .min_inner_size(300.0, 300.0)
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .minimizable(false)
            .maximizable(false);
    // NOTE: .decorations(false) is intentionally NOT set here because
    // the borderless style is applied at the NSWindow level via
    // enable_modern_window_style, which also handles rounded corners
    // and shadow. Tauri's decorations(false) alone would strip shadow
    // support on macOS.

    // Restore saved position if it is still on-screen.
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

    // Apply borderless style with rounded corners and native shadow.
    let _ = mac_rounded_corners::enable_modern_window_style(app_handle.clone(), win.clone(), None);

    let _ = win.hide();

    // Suppress state saving during initial layout.
    let is_ready = Arc::new(AtomicBool::new(false));
    let task_counter = Arc::new(Mutex::new(0u64));

    let save_trigger = create_debounced_saver(
        win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        Arc::clone(&task_counter),
        Arc::clone(&config_name),
    );

    // Arm the tracker after layout settles.
    let ready_flag = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(TRACKER_ARM_DELAY_MS)).await;
        ready_flag.store(true, Ordering::Relaxed);
    });

    // Convert to NSPanel and attach event handlers.
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
        debug!(
            "{} panel became key window",
            handle_clone.package_info().name
        );
        commands::disable_listen_hide(&handle_clone, config.label);
    });

    let app_clone = app_handle.clone();
    handler.window_did_resign_key(move |_| {
        debug!("Panel resigned key window status");
        let _ = commands::hide_window_if_unpinned_and_outside(&app_clone, config.label);
    });

    panel.set_level(PanelLevel::ModalPanel.value());
    panel.set_collection_behavior(CollectionBehavior::new().full_screen_auxiliary().into());

    // Attach the pre-cast public delegate reference. This avoids all
    // "private type" errors that would occur if passing the handler directly.
    panel.set_event_handler(Some(handler.as_protocol_delegate()));

    Ok(())
}
