#![cfg(target_os = "macos")]

use std::process::{Child, Command, Stdio};

use log::info;
use tauri::AppHandle;

use super::common::find_sidecar_path;

/// Starts the CGEvent monitoring sidecar process.
pub fn start_cgevent_sidecar(app: &AppHandle) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let binary = match find_sidecar_path(app, "fstdict_cgevent_server") {
        Some(path) => path,
        None => {
            log::warn!("CGEvent server sidecar not found — skipping");
            return Ok(None);
        }
    };

    info!("Starting CGEvent server from: {:?}", binary);
    let mut cmd = Command::new(&binary);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    use std::os::unix::process::CommandExt;
    cmd.process_group(0);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn CGEvent server: {}", e))?;

    info!("CGEvent server started (PID: {})", child.id());
    Ok(Some(child))
}
