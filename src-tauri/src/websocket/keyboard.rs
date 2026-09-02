use enigo::{Direction, Enigo, Keyboard, Settings};
use std::{thread, time};

pub fn simulate_key_press(key_code_str: String) {
    thread::spawn(move || {
        // Essential micro-delay allowing OS window management to attach text fields
        thread::sleep(time::Duration::from_millis(15));

        let mut enigo = Enigo::new(&Settings::default()).unwrap();

        // Resolve key_code_str (e.g., "KeyM", "KeyA") to native hardware IDs
        let native_code: Option<u16> = match key_code_str.as_str() {
            // macOS Scan codes vs Windows Virtual Key (VK) codes mapping
            "KeyA" => Some(if cfg!(target_os = "macos") { 0 } else { 0x41 }),
            "KeyB" => Some(if cfg!(target_os = "macos") { 11 } else { 0x42 }),
            "KeyC" => Some(if cfg!(target_os = "macos") { 8 } else { 0x43 }),
            "KeyD" => Some(if cfg!(target_os = "macos") { 2 } else { 0x44 }),
            "KeyE" => Some(if cfg!(target_os = "macos") { 14 } else { 0x45 }),
            "KeyF" => Some(if cfg!(target_os = "macos") { 3 } else { 0x46 }),
            "KeyG" => Some(if cfg!(target_os = "macos") { 5 } else { 0x47 }),
            "KeyH" => Some(if cfg!(target_os = "macos") { 4 } else { 0x48 }),
            "KeyI" => Some(if cfg!(target_os = "macos") { 34 } else { 0x49 }),
            "KeyJ" => Some(if cfg!(target_os = "macos") { 38 } else { 0x4A }),
            "KeyK" => Some(if cfg!(target_os = "macos") { 40 } else { 0x4B }),
            "KeyL" => Some(if cfg!(target_os = "macos") { 37 } else { 0x4C }),
            "KeyM" => Some(if cfg!(target_os = "macos") { 46 } else { 0x4D }),
            "KeyN" => Some(if cfg!(target_os = "macos") { 45 } else { 0x4E }),
            "KeyO" => Some(if cfg!(target_os = "macos") { 31 } else { 0x4F }),
            "KeyP" => Some(if cfg!(target_os = "macos") { 35 } else { 0x50 }),
            "KeyQ" => Some(if cfg!(target_os = "macos") { 12 } else { 0x51 }),
            "KeyR" => Some(if cfg!(target_os = "macos") { 15 } else { 0x52 }),
            "KeyS" => Some(if cfg!(target_os = "macos") { 1 } else { 0x53 }),
            "KeyT" => Some(if cfg!(target_os = "macos") { 17 } else { 0x54 }),
            "KeyU" => Some(if cfg!(target_os = "macos") { 32 } else { 0x55 }),
            "KeyV" => Some(if cfg!(target_os = "macos") { 9 } else { 0x56 }),
            "KeyW" => Some(if cfg!(target_os = "macos") { 13 } else { 0x57 }),
            "KeyX" => Some(if cfg!(target_os = "macos") { 7 } else { 0x58 }),
            "KeyY" => Some(if cfg!(target_os = "macos") { 16 } else { 0x59 }),
            "KeyZ" => Some(if cfg!(target_os = "macos") { 6 } else { 0x5A }),
            _ => None,
        };

        if let Some(scancode) = native_code {
            let _ = enigo.raw(scancode, Direction::Press);
            thread::sleep(time::Duration::from_millis(25));
            let _ = enigo.raw(scancode, Direction::Release);
        }
    });
}
