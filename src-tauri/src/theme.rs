use tauri::{AppHandle, Theme};

pub fn set_app_theme(app: &AppHandle, theme: &str){
    let val = match theme {
        "Light" => Some(Theme::Light),
        "Dark" => Some(Theme::Dark),
        _ => None,
    };
    app.set_theme(val);
}
