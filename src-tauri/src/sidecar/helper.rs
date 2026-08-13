#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use log::{error, info};

/// Locates the fstdict-helper binary in both dev and release environments.
///
/// Release: `Contents/MacOS/fstdict-helper` inside the app bundle.
/// Dev: `target/debug/fstdict-helper` relative to the cargo manifest.
pub fn find_helper_binary() -> Option<PathBuf> {
    // Release bundle path
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate = exe_dir.join("fstdict-helper");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Development build path
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_bin = manifest_dir
        .join("target")
        .join("debug")
        .join("fstdict-helper");
    if dev_bin.exists() {
        return Some(dev_bin);
    }

    None
}

/// Starts the floating helper application process.
pub fn start_helper() -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let binary = match find_helper_binary() {
        Some(path) => path,
        None => {
            error!("fstdict-helper binary not found — skipping launch");
            return Ok(None);
        }
    };

    info!("Starting fstdict-helper from: {:?}", binary);
    let mut cmd = Command::new(&binary);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    use std::os::unix::process::CommandExt;
    cmd.process_group(0);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Spawn fstdict-helper failed: {}", e))?;

    info!("fstdict-helper started, PID: {}", child.id());
    Ok(Some(child))
}
