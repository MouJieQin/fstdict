use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fstdict_common::window::state::{create_debounced_saver, WindowState};

use log::{info, warn};
use tauri::{
    App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_decorum::WebviewWindowExt;
use tauri_plugin_store::StoreExt;

/// True once the app is exiting (Cmd+Q / last window closed / ExitRequested).
/// Set in lib.rs's RunEvent handler. When it is true, window close events
/// are treated as part of app shutdown and the window's tab persistence is
/// KEPT (so the window re-opens with its tabs on the next launch).
pub static APP_EXITING: AtomicBool = AtomicBool::new(false);

/// Default size used for a freshly created (non-restored) dictionary window.
const DEFAULT_WINDOW_WIDTH: f64 = 900.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 640.0;

/// Delay before the window-state tracker starts saving (milliseconds).
const TRACKER_ARM_DELAY_MS: u64 = 500;

/// Per-label window-state file (separate from the frontend's
/// `windows/dict-tabs.json`; this one holds ONLY position/size/maximized).
fn state_file_for(label: &str) -> String {
    format!("{label}-window-state.json")
}

/// Applies the shared platform styling to a dictionary window builder.
/// `WebviewWindowBuilder` carries the runtime and manager as generics
/// (`<'a, R: Runtime, M: Manager<R>>`), so this helper is generic too.
fn style_dict_window<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    mut builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder = builder
        .inner_size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)
        .min_inner_size(400.0, 300.0)
        .disable_drag_drop_handler()
        .accept_first_mouse(true);

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .accept_first_mouse(true)
            .zoom_hotkeys_enabled(true)
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        builder = builder.transparent(true);
    }

    builder
}

/// Applies the shared platform effects AFTER the window exists.
fn style_dict_window_post(win: &WebviewWindow) -> Result<(), tauri::Error> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        win.create_overlay_titlebar()?;
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        win.make_transparent()?;
        win.set_traffic_lights_inset(12.0, 25.0)?;
        apply_vibrancy(
            win,
            NSVisualEffectMaterial::Sidebar,
            Some(NSVisualEffectState::Active),
            None,
        )
        .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
    }
    Ok(())
}

/// Overlays a saved `WindowState` onto a builder: size, then position with
/// an off-screen guard (falls back to centering). Used ONLY on the startup
/// restore path so runtime-created windows keep their default geometry.
fn apply_state_to_builder<'a, R: tauri::Runtime, M: tauri::Manager<R>>(
    mut builder: WebviewWindowBuilder<'a, R, M>,
    app: &AppHandle,
    state: &WindowState,
) -> WebviewWindowBuilder<'a, R, M> {
    builder = builder.inner_size(state.width, state.height);
    if let (Some(x), Some(y)) = (state.x, state.y) {
        if WindowState::is_position_visible(app, x, y, state.width, state.height) {
            builder = builder.position(x, y);
            info!("Restoring window position to ({x}, {y})");
        } else {
            warn!("Saved position ({x}, {y}) is off-screen. Centering instead.");
            builder = builder.center();
        }
    } else {
        builder = builder.center();
    }
    builder
}

/// Debounced saver: writes position/size/maximized to `{label}-window-state.json`
/// after the window has settled (suppressed during the first 500ms).
fn attach_window_state_save(win: &WebviewWindow, app: &AppHandle, label: &str) {
    let app_handle = app.clone();
    let config_name: Arc<str> = Arc::from(state_file_for(label));
    let is_ready = Arc::new(AtomicBool::new(false));
    let task_counter = Arc::new(Mutex::new(0u64));
    let save_trigger = create_debounced_saver(
        win.clone(),
        app_handle.clone(),
        Arc::clone(&is_ready),
        Arc::clone(&task_counter),
        Arc::clone(&config_name),
    );

    win.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => save_trigger(),
        _ => {}
    });

    let ready_flag = Arc::clone(&is_ready);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(TRACKER_ARM_DELAY_MS)).await;
        ready_flag.store(true, Ordering::Relaxed);
    });
}

/// Creates the primary application window ("main"). Saved position / size /
/// maximized are restored at STARTUP; every subsequent move/resize is
/// persisted by attach_window_state_save.
pub fn setup_main_window(app: &mut App) -> Result<(), tauri::Error> {
    let main_url = WebviewUrl::App("#/?env=main".into());
    let state = WindowState::load(app.handle(), &state_file_for("main"));
    let builder = apply_state_to_builder(
        style_dict_window(WebviewWindowBuilder::new(app, "main", main_url).title("FstDict")),
        app.handle(),
        &state,
    );
    let main_win = builder.build()?;
    style_dict_window_post(&main_win)?;
    if state.maximized {
        let _ = main_win.maximize();
    }
    attach_window_state_save(&main_win, app.handle(), "main");
    attach_close_cleanup(&main_win, app.handle());
    Ok(())
}

/// Creates a dictionary window with the given label and optional deep links.
///
/// `session_id` / `keyword` are appended to the URL
/// (`#/?env=main&session=..&keyword=..`); the new window's DictPage seeds
/// its first tab from them when nothing was persisted for that label.
///
/// `restore_state` selects the geometry source:
///   - true  (startup path: primary window / restore_persisted_main_windows)
///           -> saved position/size/maximized are restored;
///   - false (runtime path: "Open New Window" context-menu action)
///           -> the window uses the DEFAULT size and centers itself, i.e.
///           it never inherits geometry left over from an older window that
///           happened to reuse the same label.
/// Either way, further moves/resizes are saved for the NEXT startup restore.
pub fn open_main_window(
    app: &AppHandle,
    label: &str,
    session_id: Option<i64>,
    keyword: Option<String>,
    restore_state: bool,
) -> Result<WebviewWindow, String> {
    let mut url = "#/?env=main".to_string();
    if let Some(id) = session_id {
        url.push_str(&format!("&session={id}"));
    }
    if let Some(kw) = keyword {
        let kw = kw.trim();
        if !kw.is_empty() {
            url.push_str(&format!("&keyword={}", urlencoding::encode(kw)));
        }
    }

    let base = style_dict_window(
        WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into())).title("FstDict"),
    );
    let state = WindowState::load(app, &state_file_for(label));
    let builder = if restore_state {
        apply_state_to_builder(base, app, &state)
    } else {
        base.center()
    };

    let win = builder.build().map_err(|e| e.to_string())?;
    style_dict_window_post(&win).map_err(|e| e.to_string())?;
    if restore_state && state.maximized {
        let _ = win.maximize();
    }
    attach_window_state_save(&win, app, label);
    attach_close_cleanup(&win, app);
    let _ = win.set_focus();
    Ok(win)
}

/// Next free label: `main-2`, `main-3`, ... (skips labels in use).
pub fn next_main_window_label(app: &AppHandle) -> String {
    let mut n = 2u32;
    loop {
        let label = format!("main-{n}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
        n += 1;
    }
}

/// Reads `windows/dict-tabs.json` and re-opens every `main*` window (other
/// than the primary "main", which setup_main_window already created) that
/// still has persisted tabs. Windows closed manually had their tabs removed
/// (see attach_close_cleanup), so they are naturally skipped here. These
/// windows restore their saved geometry (startup path -> restore_state=true).
pub fn restore_persisted_main_windows(app: &mut App) -> Result<(), tauri::Error> {
    let Ok(store) = app.store("windows/dict-tabs.json") else {
        warn!(
            "restore_persisted_main_windows: failed to open tab store: \"windows/dict-tabs.json\""
        );
        return setup_main_window(app);
    };

    let mut restored = 0usize;
    for (label, state) in store.entries() {
        if !label.starts_with("main") || label == "main" {
            continue;
        }
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        let has_tabs = state
            .get("tabs")
            .and_then(|t| t.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if !has_tabs {
            continue;
        }
        info!("Restoring persisted main window \"{label}\"");
        if let Err(e) = open_main_window(app.handle(), &label, None, None, true) {
            log::error!("Failed to restore window \"{label}\": {e}");
            continue;
        }
        restored += 1;
    }
    info!("Restored {restored} persisted main window(s)");
    if restored == 0 {
        return setup_main_window(app);
    }
    Ok(())
}

/// Removes one window's tab data from `windows/dict-tabs.json` using the
/// store plugin (same file / same API family the frontend writes with -
/// equivalent to `(await Store.load("windows/dict-tabs.json")).delete(label)`).
fn clear_window_tabs(app: &AppHandle, label: &str) {
    let Ok(store) = app.store("windows/dict-tabs.json") else {
        warn!("clear_window_tabs: failed to open tab store for \"{label}\"");
        return;
    };
    if store.delete(label) {
        let _ = store.save();
    }
}

/// Watches a main window's close so that a MANUAL close (user clicking the
/// close button while other main windows are alive) clears that window's
/// persisted tabs - the window will not re-open on the next launch. The
/// LAST surviving main window keeps its tabs and re-opens on next launch.
///
/// The cleanup runs in `Destroyed` (NOT `CloseRequested`): the window is
/// fully destroyed by then, which is both the right time to mutate its
/// persisted tabs AND avoids the pitfall of emitting an event to a webview
/// that is already being torn down (the closing window cannot receive it).
/// `WindowEvent::Destroyed` fires after the window left webview_windows(),
/// so the alive-main count below excludes the closing window itself.
///
/// App shutdown (Cmd+Q / exit) is distinguished via APP_EXITING, which is
/// set in lib.rs's RunEvent handler before the windows are torn down.
fn attach_close_cleanup(win: &WebviewWindow, app: &AppHandle) {
    let app = app.clone();
    let label = win.label().to_string();
    win.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            if APP_EXITING.load(Ordering::Relaxed) {
                // Part of app shutdown: keep the persisted tabs.
                return;
            }
            // `Destroyed` fires AFTER the window left webview_windows(), so
            // the count below excludes the closing window itself.
            let alive_main = app
                .webview_windows()
                .iter()
                .filter(|(l, _)| l.starts_with("main"))
                .count();
            if alive_main >= 1 {
                clear_window_tabs(&app, &label);
                info!("Main window \"{label}\" closed by user; tabs cleared");
            } else {
                // Last main window: keep tabs so the next launch re-opens it.
                warn!("Last main window \"{label}\" closed; tabs preserved for next launch");
            }
        }
    });
}
