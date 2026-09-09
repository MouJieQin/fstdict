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
use log::{debug, error};
use pasteboard::{current_change_count, PasteboardSnapshot};
use std::time::{Duration, Instant};

/// Poll interval for clipboard changeCount detection.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
// Tier-specific timeouts
const TIER2_AXCOPY_TIMEOUT: Duration = Duration::from_millis(30); // AXCopy unreliable; fail fast
const TIER3_CGEVENT_TIMEOUT: Duration = Duration::from_millis(500); // WKWebView needs ~100-300ms
const POST_CHANGE_DELAY: Duration = Duration::from_millis(50); // Wait for promised data fulfillment

pub fn get_text() -> String {
    // Tier 1: AXSelectedText (works for native text fields, not WKWebView)
    match ax::get_selected_text_by_ax() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => debug!("Tier 1 (AXSelectedText) returned empty"),
        Err(err) => debug!("Tier 1 (AXSelectedText) failed: {err}"),
    }

    // Tier 2: AXCopy action — 100ms timeout.
    // WKWebView returns AXError=0 for AXCopy but does NOTHING (changeCount
    // never changes). A short timeout lets us degrade quickly instead of
    // blocking for seconds.
    match get_text_by_ax_copy() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => debug!("Tier 2 (AXCopy action) returned empty"),
        Err(err) => debug!("Tier 2 (AXCopy action) failed: {err}"),
    }

    // Tier 3: global CGEvent Cmd+C — 500ms timeout + post-change delay
    match get_text_by_global_copy() {
        Ok(text) if !text.is_empty() => return text,
        Ok(_) => debug!("Tier 3 (global CGEvent) returned empty"),
        Err(err) => error!("Tier 3 (global CGEvent) failed: {err}"),
    }

    String::new()
}

fn get_text_by_ax_copy() -> Result<String, Box<dyn std::error::Error>> {
    let element = unsafe { ax::get_focused_element_raw() }
        .ok_or("No focused AXUIElement for AXCopy action")?;

    let result = copy_with_change_count_detection(
        move || {
            if unsafe { ax::perform_ax_copy(element) } {
                Ok(())
            } else {
                Err("AXCopy action not supported by this element".into())
            }
        },
        TIER2_AXCOPY_TIMEOUT,
    );

    unsafe { CFRelease(element as *const _) };
    result
}

fn get_text_by_global_copy() -> Result<String, Box<dyn std::error::Error>> {
    copy_with_change_count_detection(|| keyboard::send_cmd_c_global(), TIER3_CGEVENT_TIMEOUT)
}

/// Shared core with per-call timeout and post-change delay.
/// AppleScript, but fully in-process:
///
/// 1. Snapshot full pasteboard (all items/types) + changeCount
/// 2. Snapshot & mute alert volume
/// 3. Fire copy via the provided closure
/// 4. Poll for changeCount increment (not text comparison)
/// 5. Read text on success
/// 6. Restore full pasteboard + alert volume
fn copy_with_change_count_detection<F>(
    fire_cmd_c: F,
    timeout: Duration,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<(), Box<dyn std::error::Error>>,
{
    let snapshot = PasteboardSnapshot::capture();
    let before_count = snapshot.change_count();

    let alert_vol = AlertVolumeSnapshot::capture().ok();
    if let Some(ref vol) = alert_vol {
        let _ = vol.mute();
    }

    if let Err(e) = fire_cmd_c() {
        if let Some(ref vol) = alert_vol {
            let _ = vol.restore();
        }
        snapshot.restore();
        return Err(e);
    }

    // Poll for changeCount
    let start = Instant::now();
    let changed = loop {
        if current_change_count() != before_count {
            break true;
        }
        if start.elapsed() >= timeout {
            break false;
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    let result = if changed {
        // WKWebView registers promised pasteboard data: changeCount increments
        // immediately, but the actual string is fulfilled asynchronously by the
        // WebContent process. Wait before reading so the promise is fulfilled.
        std::thread::sleep(POST_CHANGE_DELAY);
        pasteboard::read_text()
    } else {
        debug!("changeCount unchanged within {timeout:?}; no selection");
        String::new()
    };

    snapshot.restore();

    if let Some(ref vol) = alert_vol {
        let _ = vol.restore();
    }

    Ok(result)
}
