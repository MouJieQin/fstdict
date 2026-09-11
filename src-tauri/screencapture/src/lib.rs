// Project: screencapture
// File: lib.rs
// Created Date: 2026-09-11
// Author: MouJieQin

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use crate::linux::interactively_capture as _interactively_capture;
#[cfg(target_os = "macos")]
use crate::macos::interactively_capture as _interactively_capture;
#[cfg(target_os = "windows")]
use crate::windows::interactively_capture as _interactively_capture;

/// Get the text selected by the cursor
///
/// Return empty string if no text is selected or error occurred
/// # Example
///
/// ```
/// use screencapture::interactively_capture;
///
/// interactively_capture("./screenshot.png").unwrap();
/// ```

pub fn interactively_capture(png_path: &str) -> Result<Option<()>, String> {
    _interactively_capture(png_path)
}

#[cfg(test)]
mod tests {
    use crate::interactively_capture;
    #[test]
    fn interactively_capture_test() {
        interactively_capture("./screenshot.png").unwrap();
    }
}
