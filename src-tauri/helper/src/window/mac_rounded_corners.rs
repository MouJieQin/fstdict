//! Modern borderless window styling for macOS.
//!
//! Uses objc2 (the modern Objective-C runtime bindings) instead of the
//! deprecated `cocoa` + `objc` crates, eliminating the
//! `unexpected_cfgs: cargo-clippy` warnings.

use tauri::{AppHandle, Runtime, WebviewWindow};

#[cfg(target_os = "macos")]
use std::ffi::CStr;

#[cfg(target_os = "macos")]
use objc2::{class, msg_send, runtime::AnyObject};

// NSWindowStyleMask bit values (AppKit).
#[cfg(target_os = "macos")]
const NS_WINDOW_STYLE_MASK_TITLED: u64 = 1 << 0;
#[cfg(target_os = "macos")]
const NS_WINDOW_STYLE_MASK_CLOSABLE: u64 = 1 << 1;
#[cfg(target_os = "macos")]
const NS_WINDOW_STYLE_MASK_MINIATURIZABLE: u64 = 1 << 2;
#[cfg(target_os = "macos")]
const NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW: u64 = 1 << 15;

/// Recursively searches a view hierarchy for a WKWebView by checking each
/// view's class name. Returns null if none is found.
///
/// Tauri v2 does not expose `ns_view()` on PlatformWebview, so we locate
/// the web view by walking the window's content view subtree.
#[cfg(target_os = "macos")]
unsafe fn find_wkwebview(view: *mut AnyObject) -> *mut AnyObject {
    if view.is_null() {
        return std::ptr::null_mut();
    }

    let subviews: *mut AnyObject = msg_send![view, subviews];
    if subviews.is_null() {
        return std::ptr::null_mut();
    }

    let count: usize = msg_send![subviews, count];
    for i in 0..count {
        let subview: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
        if subview.is_null() {
            continue;
        }

        // Inspect the class name for "WKWebView".
        let class_name: *mut AnyObject = msg_send![subview, className];
        if !class_name.is_null() {
            let utf8: *const i8 = msg_send![class_name, UTF8String];
            if !utf8.is_null() {
                if let Ok(s) = CStr::from_ptr(utf8).to_str() {
                    if s.contains("WKWebView") {
                        return subview;
                    }
                }
            }
        }

        // Recurse into this subview's hierarchy.
        let found = find_wkwebview(subview);
        if !found.is_null() {
            return found;
        }
    }

    std::ptr::null_mut()
}

/// Recursively sets the background color of a view and all its subviews
/// to transparent. This eliminates the white flash that appears during
/// live resize when any view in the hierarchy paints an opaque background.
#[cfg(target_os = "macos")]
unsafe fn clear_background_recursive(view: *mut AnyObject, clear_color: *mut AnyObject) {
    if view.is_null() {
        return;
    }

    let _: () = msg_send![view, setBackgroundColor: clear_color];

    let subviews: *mut AnyObject = msg_send![view, subviews];
    if subviews.is_null() {
        return;
    }

    let count: usize = msg_send![subviews, count];
    for i in 0..count {
        let subview: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
        clear_background_recursive(subview, clear_color);
    }
}

/// Applies a modern borderless window style: rounded corners, native shadow,
/// no title bar, and no traffic light buttons. Edge-drag resizing is
/// preserved from Tauri's default configuration.
///
/// # Performance design
/// Corner clipping is applied to the WKWebView's own layer (located by
/// recursively scanning the view hierarchy). The window's content view
/// remains a plain transparent container with no masking pass. This avoids
/// the double off-screen rendering that causes laggy resizing on borderless
/// windows.
///
/// If the WKWebView cannot be located, the code falls back to clipping the
/// content view's layer.
///
/// All background colors in the entire view hierarchy are set to transparent
/// to eliminate the white flash during live resize.
///
/// # Arguments
/// * `window` - The Tauri webview window to style.
/// * `corner_radius` - Corner radius in points. Defaults to 12.0.
#[tauri::command]
pub fn enable_modern_window_style<R: Runtime>(
    _app: AppHandle<R>,
    window: WebviewWindow<R>,
    corner_radius: Option<f64>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let radius = corner_radius.unwrap_or(12.0);

        window
            .with_webview(move |webview| {
                #[cfg(target_os = "macos")]
                unsafe {
                    let win = webview.ns_window() as *mut AnyObject;

                    // --- Window style: borderless but resizable ---
                    // Read Tauri's existing mask and strip only the title-bar
                    // and control bits. NSResizableWindowMask (bit 3) is
                    // preserved so edge-drag resizing continues to work.
                    let style_mask: u64 = msg_send![win, styleMask];
                    let style_mask = (style_mask
                        & !NS_WINDOW_STYLE_MASK_TITLED
                        & !NS_WINDOW_STYLE_MASK_CLOSABLE
                        & !NS_WINDOW_STYLE_MASK_MINIATURIZABLE)
                        | NS_WINDOW_STYLE_MASK_FULL_SIZE_CONTENT_VIEW;
                    let _: () = msg_send![win, setStyleMask: style_mask];

                    // --- Shadow and window transparency ---
                    let _: () = msg_send![win, setHasShadow: true];
                    let _: () = msg_send![win, setOpaque: false];

                    let clear_color: *mut AnyObject = msg_send![class!(NSColor), clearColor];
                    let _: () = msg_send![win, setBackgroundColor: clear_color];

                    // --- Content view: transparent container, NO clipping ---
                    let content_view: *mut AnyObject = msg_send![win, contentView];
                    let _: () = msg_send![content_view, setWantsLayer: true];
                    let _: () = msg_send![content_view, setBackgroundColor: clear_color];

                    // --- Locate WKWebView and apply corner clipping ---
                    let web_view = find_wkwebview(content_view);

                    if !web_view.is_null() {
                        // Preferred path: clip the WKWebView itself.
                        // Only one layer performs masking, keeping resize smooth.
                        let _: () = msg_send![web_view, setWantsLayer: true];
                        let _: () = msg_send![web_view, setBackgroundColor: clear_color];

                        let web_layer: *mut AnyObject = msg_send![web_view, layer];
                        if !web_layer.is_null() {
                            let cg_color: *mut AnyObject = msg_send![clear_color, CGColor];
                            let _: () = msg_send![web_layer, setBackgroundColor: cg_color];
                            let _: () = msg_send![web_layer, setCornerRadius: radius];
                            let _: () = msg_send![web_layer, setMasksToBounds: true];
                        }
                    } else {
                        // Fallback: clip the content view's layer.
                        let cv_layer: *mut AnyObject = msg_send![content_view, layer];
                        if !cv_layer.is_null() {
                            let _: () = msg_send![cv_layer, setCornerRadius: radius];
                            let _: () = msg_send![cv_layer, setMasksToBounds: true];
                        }
                    }

                    // --- Clear ALL backgrounds in the view hierarchy ---
                    // Ensures no subview (including WKWebView internal views
                    // like WKContentView) paints an opaque white background
                    // that would flash during live resize.
                    clear_background_recursive(content_view, clear_color);

                    // Recompute the window shadow so it follows the rounded
                    // corners instead of the original rectangular frame.
                    let _: () = msg_send![win, invalidateShadow];
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
