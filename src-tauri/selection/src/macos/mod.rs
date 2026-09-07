//! Three-tier fallback selected-text extraction.
//!
//! Tier 1: AXSelectedText attribute       — <1ms, zero side-effects
//! Tier 2: AXCopy action on focused elem  — 5–30ms, targeted, no keyboard event
//! Tier 3: Global CGEvent Cmd+C            — 10–60ms, ultimate fallback
//!
//! Tiers 2 & 3 share: NSPasteboard changeCount detection,
//! full multi-type clipboard backup/restore, CoreAudio alert volume mute.

mod alert_volume;
mod ax;
mod keyboard;
mod pasteboard;

use alert_volume::AlertVolumeSnapshot;
use core_foundation::base::CFRelease;
use log::{error, info};
use pasteboard::{current_change_count, PasteboardSnapshot};
use std::time::{Duration, Instant};

/// Poll interval for clipboard changeCount detection.
const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Maximum wait for clipboard to change after Cmd+C (replaces `delay 0.1`).
const COPY_TIMEOUT: Duration = Duration::from_millis(200);

/// Public entry — same signature as the original implementation.
pub fn get_text() -> String {
    // ── Tier 1: direct AX attribute read ──────────────────────────────
    match ax::get_selected_text_by_ax() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => info!("Tier 1 (AXSelectedText) returned empty"),
        Err(err) => info!("Tier 1 (AXSelectedText) failed: {err}"),
    }

    // ── Tier 2: AXCopy action on focused element ──────────────────────
    match get_text_by_ax_copy() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => info!("Tier 2 (AXCopy action) returned empty"),
        Err(err) => info!("Tier 2 (AXCopy action) failed: {err}"),
    }

    // ── Tier 3: global CGEvent fallback ────────────────────────────────
    match get_text_by_global_copy() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => info!("Tier 3 (global CGEvent) returned empty"),
        Err(err) => error!("Tier 3 (global CGEvent) failed: {err}"),
    }

    String::new()
}

/// Tier 2: perform AXCopy action on the focused AXUIElement,
/// then detect success via NSPasteboard changeCount.
fn get_text_by_ax_copy() -> Result<String, Box<dyn std::error::Error>> {
    let element = unsafe { ax::get_focused_element_raw() }
        .ok_or("No focused AXUIElement for AXCopy action")?;

    // AXUIElementRef is a raw pointer (Copy), so moving it into the closure
    // still lets us CFRelease it afterwards.
    let result = copy_with_change_count_detection(move || {
        if unsafe { ax::perform_ax_copy(element) } {
            Ok(())
        } else {
            Err("AXCopy action not supported by this element".into())
        }
    });

    unsafe { CFRelease(element as *const _) };
    result
}

/// Tier 3: global CGEvent Cmd+C.
fn get_text_by_global_copy() -> Result<String, Box<dyn std::error::Error>> {
    copy_with_change_count_detection(|| keyboard::send_cmd_c_global())
}

/// Shared core for Tiers 2 & 3 — 100% behavior-aligned with the original
/// AppleScript, but fully in-process:
///
/// 1. Snapshot full pasteboard (all items/types) + changeCount
/// 2. Snapshot & mute alert volume
/// 3. Fire copy via the provided closure
/// 4. Poll for changeCount increment (not text comparison)
/// 5. Read text on success
/// 6. Restore full pasteboard + alert volume
fn copy_with_change_count_detection<F>(fire_cmd_c: F) -> Result<String, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<(), Box<dyn std::error::Error>>,
{
    // 1. Snapshot clipboard (full multi-type items + changeCount)
    let snapshot = PasteboardSnapshot::capture();
    let before_count = snapshot.change_count();

    // 2. Snapshot & mute alert volume (best-effort; skip on audio failure)
    let alert_vol = AlertVolumeSnapshot::capture().ok();
    if let Some(ref vol) = alert_vol {
        let _ = vol.mute();
    }

    // 3. Fire copy via provided method (AXCopy action or global CGEvent)
    if let Err(e) = fire_cmd_c() {
        // Restore on failure before returning
        if let Some(ref vol) = alert_vol {
            let _ = vol.restore();
        }
        snapshot.restore();
        return Err(e);
    }

    // 4. Poll for changeCount increment — replaces AppleScript's fixed `delay 0.1`.
    //    Using changeCount (not text equality) correctly handles the edge case
    //    where the newly copied text is identical to the previous clipboard content.
    let start = Instant::now();
    let changed = loop {
        if current_change_count() != before_count {
            break true;
        }
        if start.elapsed() >= COPY_TIMEOUT {
            break false;
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    // 5. Read result
    let result = if changed {
        pasteboard::read_text()
    } else {
        info!("changeCount unchanged within {COPY_TIMEOUT:?}; no selection");
        String::new()
    };

    // 6. Restore full clipboard (preserves RTF/HTML/files/images — not just text)
    snapshot.restore();

    // 7. Restore alert volume
    if let Some(ref vol) = alert_vol {
        let _ = vol.restore();
    }

    Ok(result)
}
