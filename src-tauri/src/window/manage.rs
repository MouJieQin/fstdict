use super::vibrant_window::show_vibrant_window;
use tauri::AppHandle;

pub fn show_manage_window(app: &AppHandle) -> Result<(), tauri::Error> {
    show_vibrant_window(app, "manage", "Manage")
}
