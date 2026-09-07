//! Global CGEvent Cmd+C (Tier 3 — ultimate fallback).
//! Replaces AppleScript: `tell application "System Events" to keystroke "c" using {command down}`.

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::error::Error;

// Virtual key code for 'c' (<HIToolbox/Events.h>).
// core-graphics 0.25 defines CGKeyCode as u16.
const VKEY_C: u16 = 0x08;

/// Send Cmd+C as a global HID event.
pub fn send_cmd_c_global() -> Result<(), Box<dyn Error>> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|e| format!("CGEventSource init failed: {e:?}"))?;

    // Key-down with Command flag set in a single event — no separate
    // modifier down/up needed.
    let down = CGEvent::new_keyboard_event(source.clone(), VKEY_C, true)
        .map_err(|e| format!("key-down event failed: {e:?}"))?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);

    let up = CGEvent::new_keyboard_event(source, VKEY_C, false)
        .map_err(|e| format!("key-up event failed: {e:?}"))?;
    up.post(CGEventTapLocation::HID);

    Ok(())
}
