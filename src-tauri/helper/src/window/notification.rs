use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use log::{error, info};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow};
use tauri_nspanel::{CollectionBehavior, PanelBuilder, PanelLevel};

use super::positioning::{monitor_from_cursor, position_notification_panel};
use crate::panels::NotificationPanel;

/// Globally unique task ID for debouncing notification fade-out.
static CURRENT_TASK_ID: AtomicU64 = AtomicU64::new(0);

/// Guard flag to prevent concurrent panel creation races.
static PANEL_CREATION_LOCK: AtomicBool = AtomicBool::new(false);

/// Notification display duration before fade-out starts (milliseconds).
const NOTIFICATION_DISPLAY_MS: u64 = 2000;

/// Fade-out animation duration (milliseconds).
const NOTIFICATION_FADE_MS: u64 = 550;

/// Poll interval when waiting for an in-progress panel creation (milliseconds).
const CREATION_POLL_INTERVAL_MS: u64 = 100;

/// Maximum number of poll attempts while waiting for panel creation.
const MAX_CREATION_POLLS: u32 = 10;

/// Displays a transient notification panel with the given message.
///
/// If the panel already exists, it updates the message and resets the timer.
/// If creation is already in progress on another thread, the message is delivered
/// once the panel becomes available.
pub fn show_notification(app: &AppHandle, message: String) -> Result<(), String> {
    let task_id = CURRENT_TASK_ID.fetch_add(1, Ordering::SeqCst) + 1;

    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window("notify-layer") {
        update_and_show_panel(&win, app, message, task_id);
        return Ok(());
    }

    // Concurrent creation guard
    if PANEL_CREATION_LOCK.swap(true, Ordering::SeqCst) {
        wait_and_deliver_message(app, message);
        return Ok(());
    }

    // Slow path: first-time creation
    let result = create_notification_panel(app, message, task_id);
    PANEL_CREATION_LOCK.store(false, Ordering::SeqCst);
    result
}

fn update_and_show_panel(win: &WebviewWindow, app: &AppHandle, message: String, task_id: u64) {
    let _ = win.emit_to("notify-layer", "update-message", &message);
    let _ = set_panel_position(app, win);
    let _ = win.show();

    let win_clone = win.clone();
    tauri::async_runtime::spawn(async move {
        schedule_fade_out(win_clone, task_id).await;
    });
}

fn wait_and_deliver_message(app: &AppHandle, message: String) {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        for _ in 0..MAX_CREATION_POLLS {
            tokio::time::sleep(Duration::from_millis(CREATION_POLL_INTERVAL_MS)).await;
            if let Some(win) = app_clone.get_webview_window("notify-layer") {
                let _ = win.emit_to("notify-layer", "update-message", &message);
                let _ = set_panel_position(&app_clone, &win);
                let _ = win.show();
                break;
            }
        }
    });
}

fn create_notification_panel(app: &AppHandle, message: String, task_id: u64) -> Result<(), String> {
    let encoded = urlencoding::encode(&message);
    let target_url = format!("notification.html?message={}", encoded);

    let panel = PanelBuilder::<_, NotificationPanel>::new(app, "notify-layer")
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
        .level(PanelLevel::Status)
        .build()
        .map_err(|e| format!("Failed to build notification panel: {:?}", e))?;

    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .into(),
    );

    let win: WebviewWindow = panel
        .to_window()
        .expect("Notification panel must have a valid window")
        .clone();

    let _ = set_panel_position(app, &win);
    panel.show();

    tauri::async_runtime::spawn(async move {
        schedule_fade_out(win, task_id).await;
    });

    Ok(())
}

fn set_panel_position(app: &AppHandle, win: &WebviewWindow) -> Result<(), String> {
    match monitor_from_cursor(app) {
        Ok(Some(monitor)) => {
            info!("Placing notification on cursor's monitor.");
            position_notification_panel(win, &monitor);
        }
        Ok(None) => {
            info!("Cursor monitor not found; falling back to primary.");
            if let Ok(Some(primary)) = app.primary_monitor() {
                position_notification_panel(win, &primary);
            }
        }
        Err(err) => {
            error!("Failed to detect cursor monitor: {}", err);
        }
    }
    Ok(())
}

async fn schedule_fade_out(win: WebviewWindow, task_id: u64) {
    tokio::time::sleep(Duration::from_millis(NOTIFICATION_DISPLAY_MS)).await;

    if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
        let _ = win.emit_to("notify-layer", "start-fade-out", ());
    }

    tokio::time::sleep(Duration::from_millis(NOTIFICATION_FADE_MS)).await;

    if CURRENT_TASK_ID.load(Ordering::SeqCst) == task_id {
        let _ = win.close();
    }
}
