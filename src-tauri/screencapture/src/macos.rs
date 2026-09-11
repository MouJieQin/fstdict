use std::process::Command;
use log::{error, info};

pub fn interactively_capture(png_path: &str) -> Result<Option<()>, String> {
    // '-i' triggers interactive selection mode (mouse crosshair)
    // '-r' drops screen shadows if capturing a single window frame
    let status = Command::new("screencapture")
        .args(&["-i", "-r", png_path])
        .status()
        .map_err(|e| format!("Failed to execute screencapture CLI: {}", e))?;

    if status.success() {
        info!("macOS interactive screenshot saved to {}", png_path);
        Ok(Some(()))
    } else {
        // If the user presses ESC, macOS returns a non-zero exit code
        info!("User canceled or closed the macOS screenshot utility.");
        Ok(None)
    }
}
