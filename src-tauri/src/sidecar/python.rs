use std::process::Child;
#[cfg(not(dev))]
use std::process::{Command, Stdio};

use log::info;
use tauri::App;

#[cfg(not(dev))]
use super::common::find_sidecar_path;

/// Starts the Python backend sidecar process.
///
/// Returns `Ok(None)` in dev mode or if the binary cannot be found.
///

#[cfg(dev)]
pub fn start_python_sidecar(_app: &App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    info!("Dev mode — skipping Python sidecar (run backend manually)");
    return Ok(None);
}

#[cfg(not(dev))]
pub fn start_python_sidecar(app: &App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let binary = match find_sidecar_path(app, "fstdict-server") {
        Some(path) => path,
        None => {
            log::warn!("Python sidecar 'fstdict-server' not found — skipping");
            return Ok(None);
        }
    };

    info!("Starting Python server from: {:?}", binary);
    let mut cmd = Command::new(&binary);

    // Bypass PyInstaller console detection hang on GUI apps
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    // Windows: hide transient console window
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // Unix: spawn in new process group for clean tree termination
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Python server: {}", e))?;

    info!("Python server started (PID: {})", child.id());
    Ok(Some(child))
}
