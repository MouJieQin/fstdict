pub use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewWindow};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowState {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1440.0,
            height: 900.0,
            maximized: false,
        }
    }
}

impl WindowState {
    fn config_path(app: &AppHandle, filename: &str) -> PathBuf {
        app.path()
            .app_data_dir()
            .expect("get app data dir failed")
            .join("Storage/config")
            .join(filename)
    }

    pub fn load(app: &AppHandle, filename: &str) -> Self {
        let path = Self::config_path(app, filename);
        if !path.exists() {
            return Self::default();
        }
        std::fs::read_to_string(&path)
            .map_err(|_| ())
            .and_then(|text| serde_json::from_str(&text).map_err(|_| ()))
            .unwrap_or_else(|_| Self::default())
    }

    pub fn save(&self, app: &AppHandle, filename: &str) {
        let path = Self::config_path(app, filename);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn from_window(win: &WebviewWindow) -> Self {
        let maximized = win.is_maximized().unwrap_or(false);
        let scale_factor = win.scale_factor().unwrap_or(1.0);

        let (width, height) = if maximized {
            (Self::default().width, Self::default().height)
        } else if let Ok(physical_size) = win.inner_size() {
            let logical_size = physical_size.to_logical::<f64>(scale_factor);
            (logical_size.width, logical_size.height)
        } else {
            (Self::default().width, Self::default().height)
        };

        let (x, y) = if let Ok(physical_pos) = win.outer_position() {
            let logical_pos = physical_pos.to_logical::<f64>(scale_factor);
            (Some(logical_pos.x), Some(logical_pos.y))
        } else {
            (None, None)
        };

        Self {
            x,
            y,
            width,
            height,
            maximized,
        }
    }

    pub fn is_position_visible(app: &AppHandle, x: f64, y: f64, _w: f64, _h: f64) -> bool {
        let monitors = app.available_monitors().unwrap_or_default();
        if monitors.is_empty() {
            return true;
        }

        for monitor in monitors {
            let scale_factor = monitor.scale_factor();
            let size = monitor.size().to_logical::<f64>(scale_factor);
            let pos = monitor.position().to_logical::<f64>(scale_factor);

            let m_x = pos.x;
            let m_y = pos.y;
            let m_w = size.width;
            let m_h = size.height;

            if x >= m_x && x <= (m_x + m_w) && y >= m_y && y <= (m_y + m_h) {
                return true;
            }
        }
        false
    }
}
