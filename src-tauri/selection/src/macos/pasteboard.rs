use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSData, NSString};
use std::sync::Mutex;

/// Serialize all NSPasteboard mutations. Concurrent clear/write from
/// multiple tokio workers can throw NSInternalInconsistencyException.
static PASTEBOARD_LOCK: Mutex<()> = Mutex::new(());

/// UTI for plain text (equivalent to AppKit's NSPasteboardTypeString).
const TEXT_UTI: &str = "public.utf8-plain-text";

/// Text-only pasteboard snapshot + changeCount.
/// Matches the original AppleScript behavior (`set savedClipboard to the clipboard`)
/// and avoids NSData/bytes API incompatibilities across objc2 versions.
pub struct PasteboardSnapshot {
    text: Option<String>,
    change_count: isize,
}

impl PasteboardSnapshot {
    pub fn capture() -> Self {
        let _guard = PASTEBOARD_LOCK.lock().unwrap();
        let pb = NSPasteboard::generalPasteboard();
        let text = pb
            .stringForType(&NSString::from_str(TEXT_UTI))
            .map(|s| s.to_string());
        let change_count = pb.changeCount();
        Self { text, change_count }
    }

    #[inline]
    pub fn change_count(&self) -> isize {
        self.change_count
    }

    /// Restore plain-text content. Non-text clipboard content is not
    /// preserved — same behavior as the original AppleScript fallback.
    pub fn restore(&self) {
        let _guard = PASTEBOARD_LOCK.lock().unwrap();
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        if let Some(ref text) = self.text {
            let data = NSData::from_vec(text.as_bytes().to_vec());
            pb.setData_forType(Some(&data), &NSString::from_str(TEXT_UTI));
        }
    }
}

/// Read plain-text content (serialized via PASTEBOARD_LOCK).
pub fn read_text() -> String {
    let _guard = PASTEBOARD_LOCK.lock().unwrap();
    let pb = NSPasteboard::generalPasteboard();
    match pb.stringForType(&NSString::from_str(TEXT_UTI)) {
        Some(s) => s.to_string(),
        None => String::new(),
    }
}

/// Current changeCount (integer read; safe without lock).
#[inline]
pub fn current_change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}
