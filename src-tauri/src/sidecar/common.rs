use std::path::PathBuf;
use std::process::Child;
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::time::Duration;

use log::info;
use tauri::{Manager, Runtime};

/// Returns the platform-specific filename for a sidecar binary.
#[inline]
pub fn sidecar_filename(base_name: &str) -> String {
    format!("{}{}", base_name, std::env::consts::EXE_SUFFIX)
}

/// Locates a sidecar binary using standard Tauri resource paths.
///
/// Works with both `App` and `AppHandle` via the `Manager` trait.
/// Checks resource directory first (bundled release), then executable directory.
pub fn find_sidecar_path<R: Runtime, M: Manager<R>>(
    manager: &M,
    base_name: &str,
) -> Option<PathBuf> {
    let filename = sidecar_filename(base_name);

    // Candidate 1: bundled resource directory (release .app / installer)
    if let Ok(resource_dir) = manager.path().resource_dir() {
        let path = resource_dir
            .join("sidecars")
            .join(base_name)
            .join(&filename);
        if path.exists() {
            return Some(path);
        }
    }

    // Candidate 2: same directory as main executable (onefile mode)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let path = exe_dir.join(&filename);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Gracefully terminates a child process and its entire process group.
///
/// On Unix: sends SIGTERM, waits 200ms, then SIGKILL if still alive.
/// On Windows: uses taskkill with /F /T flags to kill the process tree.
pub fn terminate_child_process(child: &mut Option<Child>, name: &str) {
    let Some(mut proc) = child.take() else { return };

    let pid = proc.id();
    info!("Terminating {} (PID: {})", name, pid);

    #[cfg(unix)]
    {
        // Send SIGTERM to the entire process group (negative PID)
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }

        std::thread::sleep(Duration::from_millis(200));

        // Force kill if process is still running
        if proc.try_wait().ok().flatten().is_none() {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW flag prevents console flash
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    // Reap the exit status to avoid zombie processes
    let _ = proc.wait();
    info!("{} terminated successfully", name);
}
