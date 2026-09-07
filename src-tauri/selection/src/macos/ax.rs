//! Tier 1: AXSelectedText via accessibility-ng (existing working code).
//! Tier 2 helpers: focused element resolution + AXCopy action via raw FFI.

use accessibility_ng::{AXAttribute, AXUIElement};
use accessibility_sys_ng::{
    kAXFocusedUIElementAttribute, kAXSelectedTextAttribute, AXUIElementCopyAttributeValue,
    AXUIElementCreateSystemWide, AXUIElementPerformAction, AXUIElementRef,
};
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use std::error::Error;

// kAXCopyAction is not exported by accessibility-sys-ng; its literal value.
const K_AX_COPY_ACTION: &str = "AXCopy";

/// Tier 1: read AXSelectedText directly — unchanged from your working code.
pub fn get_selected_text_by_ax() -> Result<String, Box<dyn Error>> {
    let system_element = AXUIElement::system_wide();
    let Some(selected_element) = system_element
        .attribute(&AXAttribute::new(&CFString::from_static_string(
            kAXFocusedUIElementAttribute,
        )))
        .map(|element| element.downcast_into::<AXUIElement>())
        .ok()
        .flatten()
    else {
        return Err(io_err("No focused element"));
    };
    let Some(selected_text) = selected_element
        .attribute(&AXAttribute::new(&CFString::from_static_string(
            kAXSelectedTextAttribute,
        )))
        .map(|text| text.downcast_into::<CFString>())
        .ok()
        .flatten()
    else {
        return Err(io_err("No selected text"));
    };
    Ok(selected_text.to_string())
}

/// Resolve the currently focused AXUIElement as a raw FFI reference.
/// The caller owns the returned reference and must CFRelease it.
///
/// # Safety
/// Caller must ensure CFRelease is called on the returned pointer.
pub unsafe fn get_focused_element_raw() -> Option<AXUIElementRef> {
    let system = AXUIElementCreateSystemWide();
    if system.is_null() {
        return None;
    }
    let attr = CFString::from_static_string(kAXFocusedUIElementAttribute);
    let mut value: CFTypeRef = std::ptr::null();
    let result = AXUIElementCopyAttributeValue(system, attr.as_concrete_TypeRef(), &mut value);
    CFRelease(system as *const _);

    if result != 0 || value.is_null() {
        None
    } else {
        Some(value as AXUIElementRef)
    }
}

/// Perform the AXCopy action on a raw element reference.
/// Returns true on success (AXError == 0).
///
/// # Safety
/// `element` must be a valid AXUIElementRef.
pub unsafe fn perform_ax_copy(element: AXUIElementRef) -> bool {
    let action = CFString::from_static_string(K_AX_COPY_ACTION);
    AXUIElementPerformAction(element, action.as_concrete_TypeRef()) == 0
}

#[inline]
fn io_err(msg: &str) -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        msg.to_string(),
    ))
}
