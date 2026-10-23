use anyhow::anyhow;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_decorum::WebviewWindowExt;
#[cfg(target_os = "windows")]
use window_vibrancy::apply_blur;
#[cfg(target_os = "macos")]
use window_vibrancy::{
    apply_vibrancy, clear_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
};

pub fn show_vibrant_window(app: &AppHandle, label: &str, title: &str) -> Result<(), tauri::Error> {
    // Fast path: panel already exists
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        return Ok(());
    }
    let app_clone = app.clone();
    let label_clone: String = label.to_string();
    let title_clone: String = title.to_string();
    let _ = app.run_on_main_thread(move || {
        let _ = create_vibrant_window(&app_clone, &label_clone, &title_clone);
    });
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn restyle_vibrant_window(
    win: &WebviewWindow,
    material: u64,
    state: u64,
) -> Result<(), tauri::Error> {
    let _ = clear_vibrancy(&win);

    win.make_transparent()?;

    let mat = material_from_u64(material)?;
    let st = state_from_u64(state)?;

    let _ = apply_vibrancy(&win, mat, Some(st), None)
        .map_err(|e| format!("!apply_vibrancy failed: {e}"));

    Ok(())
}

/// Creates the setting window with saved state restoration.
fn create_vibrant_window(app: &AppHandle, label: &str, title: &str) -> Result<(), tauri::Error> {
    // Window position / size are persisted by the window-state plugin.
    let main_url = WebviewUrl::App(format!("#/{}", label).into());

    let mut builder = WebviewWindowBuilder::new(app, label, main_url)
        .title(title)
        .inner_size(900.0, 640.0)
        .min_inner_size(400.0, 300.0)
        .transparent(true)
        .accept_first_mouse(true);

    // Platform-specific window builder configuration
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .accept_first_mouse(true)
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);
    }

    let main_win = builder.center().build()?;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        main_win.create_overlay_titlebar()?;
    }

    #[cfg(target_os = "windows")]
    apply_blur(&main_win, Some((18, 18, 18, 125)))
        .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

    #[cfg(target_os = "macos")]
    {
        // main_win.set_traffic_lights_inset(25.0, 30.0)?;
        main_win.make_transparent()?;
        apply_vibrancy(
            &main_win,
            NSVisualEffectMaterial::Sidebar,
            // NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            // Some(NSVisualEffectState::FollowsWindowActiveState),
            None,
        )
        .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn material_from_u64(v: u64) -> Result<NSVisualEffectMaterial, tauri::Error> {
    let mat = match v {
        3 => NSVisualEffectMaterial::Titlebar,
        4 => NSVisualEffectMaterial::Selection,
        5 => NSVisualEffectMaterial::Menu,
        6 => NSVisualEffectMaterial::Popover,
        7 => NSVisualEffectMaterial::Sidebar,
        10 => NSVisualEffectMaterial::HeaderView,
        11 => NSVisualEffectMaterial::Sheet,
        12 => NSVisualEffectMaterial::WindowBackground,
        13 => NSVisualEffectMaterial::HudWindow,
        15 => NSVisualEffectMaterial::FullScreenUI,
        17 => NSVisualEffectMaterial::Tooltip,
        18 => NSVisualEffectMaterial::ContentBackground,
        21 => NSVisualEffectMaterial::UnderWindowBackground,
        22 => NSVisualEffectMaterial::UnderPageBackground,
        _ => NSVisualEffectMaterial::Sidebar,
    };
    Ok(mat)
}

#[cfg(target_os = "macos")]
fn state_from_u64(v: u64) -> Result<NSVisualEffectState, tauri::Error> {
    let st = match v {
        0 => NSVisualEffectState::FollowsWindowActiveState,
        1 => NSVisualEffectState::Active,
        2 => NSVisualEffectState::Inactive,
        _ => NSVisualEffectState::FollowsWindowActiveState,
    };
    Ok(st)
}
