use log::{debug, error, warn};
use tauri::{AppHandle, LogicalPosition, Manager, Monitor, Position, WebviewWindow};

/// Returns the monitor that currently contains the mouse cursor.
///
/// Falls back to the primary monitor if detection fails.
pub fn monitor_from_cursor(app: &AppHandle) -> Result<Option<Monitor>, tauri::Error> {
    let Some(primary_monitor) = app.primary_monitor()? else {
        return Ok(None);
    };

    let scale = primary_monitor.scale_factor();
    let cursor_pos = app.cursor_position()?.to_logical::<f64>(scale);

    debug!(
        "Cursor logical position: ({}, {})",
        cursor_pos.x, cursor_pos.y
    );
    app.monitor_from_point(cursor_pos.x, cursor_pos.y)
}

/// Checks whether the mouse cursor is currently inside the window bounds.
pub fn is_cursor_over_window(app: &AppHandle, label: &str) -> bool {
    let Some(win) = app.get_webview_window(label) else {
        warn!("Window with label '{}' not found.", label);
        return false;
    };

    if !win.is_visible().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        return false;
    }

    let Ok(cursor_physical) = app.cursor_position() else {
        error!("Failed to retrieve system cursor position.");
        return false;
    };

    let Ok(physical_pos) = win.outer_position() else {
        return false;
    };
    let Ok(physical_size) = win.inner_size() else {
        return false;
    };

    let min_x = physical_pos.x as f64;
    let max_x = (physical_pos.x + physical_size.width as i32) as f64;
    let min_y = physical_pos.y as f64;
    let max_y = (physical_pos.y + physical_size.height as i32) as f64;

    cursor_physical.x >= min_x
        && cursor_physical.x <= max_x
        && cursor_physical.y >= min_y
        && cursor_physical.y <= max_y
}

/// Positions a window near the cursor, keeping it fully on-screen.
///
/// The panel is placed on the side of the cursor that has more screen real estate,
/// with a safety margin to prevent it from extending past display edges.
pub fn position_window_near_cursor(
    app: &AppHandle,
    win: &WebviewWindow,
) -> Result<(), tauri::Error> {
    let monitor = match monitor_from_cursor(app)? {
        Some(m) => m,
        None => {
            warn!("Could not determine current monitor; using primary.");
            match app.primary_monitor()? {
                Some(m) => m,
                None => {
                    warn!("No primary monitor available; skipping position update.");
                    return Ok(());
                }
            }
        }
    };

    position_window_on_monitor(app, &monitor, win)
}

fn position_window_on_monitor(
    app: &AppHandle,
    monitor: &Monitor,
    win: &WebviewWindow,
) -> Result<(), tauri::Error> {
    let mouse_physical = app.cursor_position()?;
    let scale = monitor.scale_factor();

    // Use primary monitor scale for cursor conversion (system global coordinate space)
    let cursor_scale = app
        .primary_monitor()?
        .map(|m| m.scale_factor())
        .unwrap_or(scale);

    let screen_pos = monitor.position().to_logical::<f64>(scale);
    let screen_size = monitor.size().to_logical::<f64>(scale);

    let mouse_x = mouse_physical.x / cursor_scale;
    let mouse_y = mouse_physical.y / cursor_scale;

    let win_physical_size = win.inner_size().unwrap_or_default();
    let win_width = win_physical_size.width as f64 / scale;
    let win_height = win_physical_size.height as f64 / scale;

    const CURSOR_PADDING: f64 = 20.0;
    const OUTER_MARGIN: f64 = 8.0;

    // Horizontal placement: place on the side with more space
    let monitor_center_x = screen_pos.x + screen_size.width / 2.0;
    let mut x = if mouse_x > monitor_center_x {
        mouse_x - win_width - CURSOR_PADDING
    } else {
        mouse_x + CURSOR_PADDING
    };

    // Vertical placement: place above or below cursor
    let monitor_center_y = screen_pos.y + screen_size.height / 2.0;
    let mut y = if mouse_y > monitor_center_y {
        mouse_y - win_height - CURSOR_PADDING
    } else {
        mouse_y + CURSOR_PADDING
    };

    // Clamp Horizontal Edges
    if x + win_width > screen_pos.x + screen_size.width - OUTER_MARGIN {
        x = screen_pos.x + screen_size.width - win_width - OUTER_MARGIN;
    }
    if x < screen_pos.x + OUTER_MARGIN {
        x = screen_pos.x + OUTER_MARGIN;
    }

    // Clamp Vertical Edges
    if y + win_height > screen_pos.y + screen_size.height - OUTER_MARGIN {
        y = screen_pos.y + screen_size.height - win_height - OUTER_MARGIN;
    }
    if y < screen_pos.y + OUTER_MARGIN {
        y = screen_pos.y + OUTER_MARGIN;
    }

    win.set_position(Position::Logical(LogicalPosition::new(x, y)))?;
    Ok(())
}

/// Positions the notification panel in the top-right corner of the target monitor.
pub fn position_notification_panel(win: &WebviewWindow, monitor: &Monitor) {
    let scale = monitor.scale_factor();
    let screen_pos = monitor.position().to_logical::<f64>(scale);
    let screen_size = monitor.size().to_logical::<f64>(scale);

    const PANEL_WIDTH: f64 = 360.0;
    const EDGE_PADDING: f64 = 24.0;
    const TOP_PADDING: f64 = 40.0;

    let target_x = screen_pos.x + screen_size.width - PANEL_WIDTH - EDGE_PADDING;
    let target_y = screen_pos.y + TOP_PADDING;

    let _ = win.set_position(Position::Logical(LogicalPosition::new(target_x, target_y)));
}
