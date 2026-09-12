use std::env;
use std::process::Command;

pub fn interactively_capture(png_path: &str) -> Result<Option<()>, String> {
    // Detect if the user is running X11 or Wayland
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "x11".to_string());

    if session_type.to_lowercase() == "wayland" {
        // Wayland Standard: Use grim + slurp
        // Requires 'grim' and 'slurp' installed on the host system
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("slurp | grim -g - {}", png_path))
            .status()
            .map_err(|e| format!("Failed to run Wayland capture: {}", e))?;

        if output.success() {
            return Ok(Some(()));
        }
    } else {
        // X11 Legacy Standard: Use maim or scrot
        // Requires 'maim' or 'scrot' installed on the host system
        let status = Command::new("maim")
            .args(&["-s", png_path]) // '-s' enables interactive selection mode
            .status()
            .map_err(|e| format!("Failed to run X11 capture (maim): {}", e))?;

        if status.success() {
            return Ok(Some(()));
        }
    }

    Ok(None)
}
