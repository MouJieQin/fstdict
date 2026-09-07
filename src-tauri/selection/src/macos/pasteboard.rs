//! Native NSPasteboard wrapper: precise changeCount + full multi-type backup/restore.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString};

/// UTI for plain text — equivalent to AppKit's NSPasteboardTypeString.
fn string_type() -> Retained<NSString> {
    NSString::from_str("public.utf8-plain-text")
}

/// Immutable snapshot: all pasteboard items (every UTI type) + changeCount.
pub struct PasteboardSnapshot {
    items: Retained<NSArray<NSPasteboardItem>>,
    change_count: isize,
}

impl PasteboardSnapshot {
    /// Capture the current general pasteboard state.
    pub fn capture() -> Self {
        let pb = NSPasteboard::generalPasteboard();
        let items = pb.pasteboardItems().unwrap_or_else(NSArray::new);
        let change_count = pb.changeCount();
        Self {
            items,
            change_count,
        }
    }

    #[inline]
    pub fn change_count(&self) -> isize {
        self.change_count
    }

    /// Restore the general pasteboard to this snapshot's full contents.
    /// Preserves ALL UTI types (RTF, HTML, file URLs, images, …) —
    /// unlike a text-only backup/restore.
    pub fn restore(&self) {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();

        // NSArray<T> is #[repr(transparent)] around the same NSMutableArray
        // object pointer regardless of T. NSPasteboardItem conforms to
        // NSPasteboardWriting, and Objective-C generics are erased at runtime,
        // so this transmute is sound.
        let writing_items: Retained<NSArray<ProtocolObject<dyn NSPasteboardWriting>>> =
            unsafe { std::mem::transmute(self.items.clone()) };
        let _ = pb.writeObjects(&writing_items);
    }
}

/// Read plain-text content from the general pasteboard.
#[inline]
pub fn read_text() -> String {
    let pb = NSPasteboard::generalPasteboard();
    match pb.stringForType(&string_type()) {
        Some(s) => s.to_string(),
        None => String::new(),
    }
}

/// Current changeCount of the general pasteboard.
#[inline]
pub fn current_change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}
