// ============================================================
// context_menu.rs
// ------------------------------------------------------------------
// Native right-click context menu for the dictionary windows
// (cross-platform: Windows / macOS / Linux via the Tauri menu crate).
//
// Flow:
//   1. The webview's right-click handler calls the `show_context_menu`
//      Tauri command with the cursor position (physical pixels), the
//      currently selected text (if any), the active UI locale and the
//      list of backend sessions.
//   2. Rust builds a native `Menu` - including per-session Submenus -
//      remembers which window opened it, and pops it up.
//   3. On item click, Tauri dispatches an app-level `MenuEvent`; the
//      `menu_event_handler` maps the item id back to an action string
//      (session submenu items carry the session id) and emits it to the
//      window that opened the menu as a "ctx-menu-action" event.
//   4. The frontend (DictPage) turns the action into a tab or window
//      operation.
//
// Menu layout (labels come from the SHARED frontend i18n JSONs - Rust
// embeds all four locale files at compile time, the frontend passes the
// active locale code on every invocation; see the I18N section below):
//
//   [text selected]   [no selection]
//     Look Up "<p>"      -
//     New Tab            New Tab
//     New Tab (Session)▸ New Tab (Session)▸
//     New Tab (Session, Look Up "<p>")▸   -
//     ----------         ----------
//     New Window         New Window
//     New Window Look Up "<p>"  -
//     New Window (Session)▸ New Window (Session)▸
//     New Window (Session, Look Up "<p>")▸   -
//     ----------
//     Copy (native)
//
// "Look Up" items and Copy only appear when the user has a selection.
// ============================================================

use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use tauri::menu::{ContextMenu, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Window};

// ------------------------------------------------------------------
// I18N bridge: the native menu reuses the Vue app's locale JSON files.
// All four files are embedded at compile time; `show_context_menu`
// receives the frontend's active locale and we look the label up in the
// matching JSON (fallback: English). The frontend JSON is the single
// source of truth - translating here would drift from the webview.
// ------------------------------------------------------------------

const EN_JSON: &str = include_str!("../../../src/locales/en.json");
const ZH_JSON: &str = include_str!("../../../src/locales/zh.json");
const JA_JSON: &str = include_str!("../../../src/locales/ja.json");
const KO_JSON: &str = include_str!("../../../src/locales/ko.json");

static EN: Lazy<Value> = Lazy::new(|| serde_json::from_str(EN_JSON).unwrap_or(Value::Null));
static ZH: Lazy<Value> = Lazy::new(|| serde_json::from_str(ZH_JSON).unwrap_or(Value::Null));
static JA: Lazy<Value> = Lazy::new(|| serde_json::from_str(JA_JSON).unwrap_or(Value::Null));
static KO: Lazy<Value> = Lazy::new(|| serde_json::from_str(KO_JSON).unwrap_or(Value::Null));

/// Translate a `contextMenu.<key>` string for the given locale.
fn t(locale: &str, key: &str) -> String {
    let data = match locale {
        "zh" => &*ZH,
        "ja" => &*JA,
        "ko" => &*KO,
        _ => &*EN,
    };
    data.pointer(&format!("/contextMenu/{key}"))
        .and_then(Value::as_str)
        .unwrap_or(key)
        .to_string()
}

/// Translate a label and substitute the `{preview}` placeholder.
fn t_preview(locale: &str, key: &str, preview: &str) -> String {
    t(locale, key).replace("{preview}", preview)
}

// ------------------------------------------------------------------
// Menu item ids (matched inside `menu_event_handler`).
// ------------------------------------------------------------------

const ID_NEW_TAB: &str = "ctx-new-tab";
const ID_LOOKUP_SELECTION: &str = "ctx-lookup-selection";
/// Suffix-prefixed ids: `ctx-new-tab-session:<sessionId>`.
const ID_NEW_TAB_SESSION: &str = "ctx-new-tab-session:";
const ID_NEW_TAB_SESSION_LOOKUP: &str = "ctx-new-tab-session-lookup:";
const ID_NEW_WINDOW: &str = "ctx-new-window";
const ID_NEW_WINDOW_LOOKUP: &str = "ctx-new-window-lookup";
/// Suffix-prefixed ids: `ctx-new-window-session:<sessionId>`.
const ID_NEW_WINDOW_SESSION: &str = "ctx-new-window-session:";
const ID_NEW_WINDOW_SESSION_LOOKUP: &str = "ctx-new-window-session-lookup:";
/// Move the active tab into a brand-new window (shown only with 2+ tabs).
const ID_MOVE_TAB_TO_NEW_WINDOW: &str = "ctx-move-tab-to-new-window";

/// Tauri event emitted to the webview that opened the menu.
pub const CTX_MENU_ACTION_EVENT: &str = "ctx-menu-action";

/// One backend session entry. The frontend sends the list it received via
/// the `sessions_name_id` WebSocket message.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionItem {
    pub id: i64,
    pub name: String,
}

/// Label of the window that most recently opened the context menu.
///
/// `MenuEvent` carries only the item id (not the source window), so we
/// remember the window here in order to route the click back to the
/// correct webview (main / helper-main / helper-selection each host
/// their own DictPage and must act independently).
#[derive(Default)]
pub struct ContextMenuWindow(pub Mutex<Option<String>>);

/// App-level `MenuEvent` handler: forward item clicks to the right window.
pub fn menu_event_handler() -> impl Fn(&AppHandle, tauri::menu::MenuEvent) + Send + Sync + 'static {
    |app, event| {
        let id = event.id().as_ref();
        // NOTE: the more specific (suffix-prefixed) ids must be tested
        // BEFORE the plain `ctx-new-window` prefix, otherwise a session id
        // like "ctx-new-window-session:3" would be mistaken for a window.
        let action = if id == ID_NEW_TAB {
            Some("new-tab".to_string())
        } else if id == ID_LOOKUP_SELECTION {
            Some("lookup-selection".to_string())
        } else if let Some(rest) = id.strip_prefix(ID_NEW_TAB_SESSION_LOOKUP) {
            Some(format!("new-tab-session-lookup:{rest}"))
        } else if let Some(rest) = id.strip_prefix(ID_NEW_TAB_SESSION) {
            Some(format!("new-tab-session:{rest}"))
        } else if let Some(rest) = id.strip_prefix(ID_NEW_WINDOW_SESSION_LOOKUP) {
            Some(format!("new-window-session-lookup:{rest}"))
        } else if let Some(rest) = id.strip_prefix(ID_NEW_WINDOW_SESSION) {
            Some(format!("new-window-session:{rest}"))
        } else if id == ID_NEW_WINDOW_LOOKUP {
            Some("new-window-lookup".to_string())
        } else if id == ID_NEW_WINDOW {
            Some("new-window".to_string())
        } else if id == ID_MOVE_TAB_TO_NEW_WINDOW {
            Some("move-tab-to-new-window".to_string())
        } else {
            None
        };
        let Some(action) = action else { return };

        let Some(label) = app.state::<ContextMenuWindow>().0.lock().unwrap().clone() else {
            return;
        };
        if let Some(win) = app.get_webview_window(&label) {
            // NOTE: use emit_to (NOT emit) - `emit` is a GLOBAL broadcast and
            // would be delivered to EVERY window listening on this event
            // (every DictPage window registers a listener), so a single
            // "Open New Window" action would fire in every main window and
            // duplicate the window on each launch. emit_to targets ONLY the
            // window that opened the context menu.
            let _ = win.emit_to(&label, CTX_MENU_ACTION_EVENT, action);
        }
    }
}

/// Build a submenu whose items are the backend sessions.
/// Item id = `{id_prefix}{sessionId}`, label = the session's name.
/// `preview` (the selected-text preview) is substituted into the title when
/// Some - used by the "to Look Up" submenus (their title carries `{preview}`).
fn build_session_submenu(
    app: &AppHandle,
    locale: &str,
    label_key: &str,
    id_prefix: &str,
    sessions: &[SessionItem],
    preview: Option<&str>,
) -> Result<Submenu<tauri::Wry>, String> {
    let mut items: Vec<MenuItem<tauri::Wry>> = Vec::with_capacity(sessions.len());
    for s in sessions {
        let label = if s.name.trim().is_empty() {
            format!("Session {}", s.id)
        } else {
            s.name.clone()
        };
        items.push(
            MenuItem::with_id(
                app,
                format!("{id_prefix}{}", s.id),
                label,
                true,
                None::<&str>,
            )
            .map_err(|e| e.to_string())?,
        );
    }
    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items
        .iter()
        .map(|i| i as &dyn IsMenuItem<tauri::Wry>)
        .collect();
    let title = match preview {
        Some(p) => t_preview(locale, label_key, p),
        None => t(locale, label_key),
    };
    Submenu::with_items(app, title, true, &refs).map_err(|e| e.to_string())
}

/// Tauri command: pop up the native context menu at (x, y) physical pixels.
///
/// The command receives the invoking window as `tauri::Window` (the
/// `ContextMenu` trait methods take a `Window` BY VALUE, not a reference).
/// `selected_text` decides which items the menu contains; `locale` selects
/// the embedded i18n JSON; `sessions` feeds the per-session submenus.
#[tauri::command]
pub fn show_context_menu(
    window: Window,
    x: f64,
    y: f64,
    selected_text: Option<String>,
    locale: String,
    sessions: Vec<SessionItem>,
    tab_count: usize,
) -> Result<(), String> {
    let app = window.app_handle().clone();
    let text = selected_text.unwrap_or_default().trim().to_string();
    let has_selection = !text.is_empty();

    // Truncated preview shown inside the "to Look Up" labels.
    let preview: String = {
        let mut chars = text.chars();
        let head: String = chars.by_ref().take(18).collect();
        if chars.next().is_some() {
            format!("{head}…")
        } else {
            head
        }
    };

    let menu = Menu::new(&app).map_err(|e| e.to_string())?;

    // --- Look up the selection in a new tab (selection only) ---
    if has_selection {
        menu.append(
            &MenuItem::with_id(
                &app,
                ID_LOOKUP_SELECTION,
                t_preview(&locale, "openNewTabLookup", &preview),
                true,
                Some("CmdOrCtrl+Shift+Enter"),
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }

    // --- Plain "Open New Tab" (clone the active session) ---
    menu.append(
        &MenuItem::with_id(
            &app,
            ID_NEW_TAB,
            t(&locale, "openNewTab"),
            true,
            Some("CmdOrCtrl+N"),
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // --- "Open New Tab in the Session" submenu (per-session items) ---
    if !sessions.is_empty() {
        let sub = build_session_submenu(
            &app,
            &locale,
            "openNewTabInSession",
            ID_NEW_TAB_SESSION,
            &sessions,
            None,
        )?;
        menu.append(&sub).map_err(|e| e.to_string())?;

        if has_selection {
            let sub = build_session_submenu(
                &app,
                &locale,
                "openNewTabInSessionLookup",
                ID_NEW_TAB_SESSION_LOOKUP,
                &sessions,
                Some(&preview),
            )?;
            menu.append(&sub).map_err(|e| e.to_string())?;
        }
    }

    menu.append(&PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // --- "Open New Window" group ---
    menu.append(
        &MenuItem::with_id(
            &app,
            ID_NEW_WINDOW,
            t(&locale, "openNewWindow"),
            true,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    if has_selection {
        menu.append(
            &MenuItem::with_id(
                &app,
                ID_NEW_WINDOW_LOOKUP,
                t_preview(&locale, "openNewWindowLookup", &preview),
                true,
                None::<&str>,
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }

    if !sessions.is_empty() {
        let sub = build_session_submenu(
            &app,
            &locale,
            "openNewWindowInSession",
            ID_NEW_WINDOW_SESSION,
            &sessions,
            None,
        )?;
        menu.append(&sub).map_err(|e| e.to_string())?;

        if has_selection {
            let sub = build_session_submenu(
                &app,
                &locale,
                "openNewWindowInSessionLookup",
                ID_NEW_WINDOW_SESSION_LOOKUP,
                &sessions,
                Some(&preview),
            )?;
            menu.append(&sub).map_err(|e| e.to_string())?;
        }
    }

    // --- "Move Tab to New Window" (only when the window has 2+ tabs) ---
    if tab_count > 1 {
        menu.append(
            &MenuItem::with_id(
                &app,
                ID_MOVE_TAB_TO_NEW_WINDOW,
                t(&locale, "moveTabToNewWindow"),
                true,
                None::<&str>,
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }

    // --- Native Copy (selection only) ---
    if has_selection {
        let copy_label = t(&locale, "copy");
        menu.append(&PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        menu.append(
            &PredefinedMenuItem::copy(&app, Some(copy_label.as_str()))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }

    // Remember the source window so the click can be routed back to it.
    *app.state::<ContextMenuWindow>().0.lock().unwrap() = Some(window.label().to_string());

    // `popup` (unlike `popup_at`) takes no coordinates and pops at the OS
    // cursor - which is exactly where the right-click happened. The exact
    // x/y arguments are kept for future `popup_at` use if we ever need to
    // open the menu away from the cursor.
    let _ = (x, y);
    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
}
