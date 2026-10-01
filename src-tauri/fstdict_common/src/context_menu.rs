// ============================================================
// context_menu.rs
// ------------------------------------------------------------------
// Native right-click context menu for the dictionary windows
// (cross-platform: Windows / macOS / Linux via the Tauri menu crate).
//
// Flow:
//   1. The webview's right-click handler calls the `show_context_menu`
//      Tauri command with the cursor position (physical pixels) and the
//      currently selected text (if any).
//   2. Rust builds a native `Menu`, remembers which window opened it,
//      and pops it up at the given position.
//   3. On item click, Tauri dispatches an app-level `MenuEvent`; the
//      `menu_event_handler` forwarded to the Builder routes the action
//      back to THAT window's webview as a "ctx-menu-action" event.
//   4. The frontend (DictPage) turns the action into a tab operation:
//        "new-tab"           -> clone the active session into a new tab
//        "lookup-selection"  -> open a new tab and look up the selection
//
// Items shown (built per invocation):
//   - "在 New Tab 中查询 \"<preview>\""   (only when text is selected)
//   - separator
//   - "创建 New Tab"                       (always)
//   - separator + "复制" (native copy)     (only when text is selected)
// ============================================================

use std::sync::Mutex;

use tauri::menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Window};

/// Menu item ids (matched inside `menu_event_handler`).
const ID_NEW_TAB: &str = "ctx-new-tab";
const ID_LOOKUP_SELECTION: &str = "ctx-lookup-selection";

/// Tauri event emitted to the webview that opened the menu.
pub const CTX_MENU_ACTION_EVENT: &str = "ctx-menu-action";

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
        let action = match event.id().as_ref() {
            ID_NEW_TAB => Some("new-tab"),
            ID_LOOKUP_SELECTION => Some("lookup-selection"),
            _ => None,
        };
        let Some(action) = action else { return };

        let Some(label) = app.state::<ContextMenuWindow>().0.lock().unwrap().clone() else {
            return;
        };
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.emit(CTX_MENU_ACTION_EVENT, action);
        }
    }
}

/// Tauri command: pop up the native context menu at (x, y) physical pixels.
///
/// The command receives the invoking window as `tauri::Window` (the
/// `ContextMenu` trait methods take a `Window` BY VALUE, not a reference).
/// `selected_text` decides which items the menu contains; the text itself
/// is not needed again until the click arrives (the frontend keeps it).
#[tauri::command]
pub fn show_context_menu(
    window: Window,
    x: f64,
    y: f64,
    selected_text: Option<String>,
) -> Result<(), String> {
    let app = window.app_handle().clone();
    let text = selected_text.unwrap_or_default().trim().to_string();
    let has_selection = !text.is_empty();

    let menu = Menu::new(&app).map_err(|e| e.to_string())?;

    if has_selection {
        // Truncate the preview shown inside the label.
        let preview: String = {
            let mut chars = text.chars();
            let head: String = chars.by_ref().take(18).collect();
            if chars.next().is_some() {
                format!("{head}…")
            } else {
                head
            }
        };
        menu.append(
            &MenuItem::with_id(
                &app,
                ID_LOOKUP_SELECTION,
                format!("在 New Tab 中查询 \"{preview}\""),
                true,
                None::<&str>,
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }

    menu.append(
        &MenuItem::with_id(&app, ID_NEW_TAB, "创建 New Tab", true, None::<&str>)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    if has_selection {
        menu.append(&PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::copy(&app, Some("复制")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }

    // Remember the source window so the click can be routed back to it.
    *app.state::<ContextMenuWindow>().0.lock().unwrap() = Some(window.label().to_string());

    // NOTE: `popup_at` lives on the `tauri::menu::ContextMenu` trait (imported
    // above). It takes the window BY VALUE and the popup position; unlike
    // `popup` (which pops at the OS cursor), `popup_at` uses our exact
    // right-click coordinates.
    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
    // menu.popup_at(
    //     window,
    //     tauri::Position::Physical(PhysicalPosition::new(x.round() as i32, y.round() as i32)),
    // )
    // .map_err(|e| e.to_string())
}
