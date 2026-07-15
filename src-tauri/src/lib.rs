// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use chrono::Local;
use colored::*;
use env_logger::{Builder, Env};
use log::{debug, error, info, warn, Level, LevelFilter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;
use tauri::{App, Manager, RunEvent};

#[cfg(unix)]

/// Initialize logging: colored console + daily rotated file output.
/// Must be called after the Tauri app is created so we can use app_log_dir().
pub fn init_logging(log_dir: &std::path::Path) {
    let env = Env::default().filter_or("RUST_LOG", "info");
    let mut builder = Builder::from_env(env);

    // Silence noisy third-party crates
    builder
        .filter_module("reqwest", LevelFilter::Warn)
        .filter_module("hyper", LevelFilter::Warn)
        .filter_module("hyper_util", LevelFilter::Warn)
        .filter_module("tauri_plugin_updater", LevelFilter::Warn);

    let _ = fs::create_dir_all(log_dir);
    println!("Log directory: {:?}", log_dir);

    // Custom formatter matching Python server style
    let log_dir = log_dir.to_path_buf();
    builder.format(move |buf, record| {
        let now = Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let level = record.level();
        let level_str = format!("{:>8}", level.as_str());

        let thread_id = format!("{:?}", std::thread::current().id());
        let thread_short = thread_id.replace("ThreadId(", "").replace(')', "");

        let file_loc = match (record.file(), record.line()) {
            (Some(file), Some(line)) => {
                let filename = file.rsplit('/').next().unwrap_or(file);
                format!("{}:{}", filename, line)
            }
            _ => "-".to_string(),
        };

        // ── Colored console output ──
        let colored_level = match level {
            Level::Error => level_str.red().bold(),
            Level::Warn => level_str.yellow().bold(),
            Level::Info => level_str.green(),
            Level::Debug => level_str.cyan(),
            Level::Trace => level_str.white().dimmed(),
        };

        let console_line = format!(
            "{} [{}] [thread {}] [{}] {}",
            time_str.dimmed(),
            colored_level,
            thread_short,
            file_loc.purple(),
            record.args()
        );
        let _ = writeln!(buf, "{}", console_line);

        // ── Plain file output (no ANSI codes, daily rotation) ──
        if let Ok(mut file) = daily_log_file(&log_dir) {
            let file_line = format!(
                "{} [{}] [thread {}] [{}] {}",
                time_str,
                level_str,
                thread_short,
                file_loc,
                record.args()
            );
            let _ = writeln!(file, "{}", file_line);
            let _ = file.flush();
        }

        Ok(())
    });

    let _ = builder.try_init();
}

/// Get or create the daily log file handle
fn daily_log_file(log_dir: &PathBuf) -> std::io::Result<fs::File> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let log_file = log_dir.join(format!("fstdict-app-{}.log", today));

    OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(log_file)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Application state holding the Python sidecar process handle
struct PythonServer(Mutex<Option<Child>>);

/// Build platform-specific sidecar binary base name.
fn sidecar_filename(base_name: &str) -> String {
    format!("{}{}", base_name, std::env::consts::EXE_SUFFIX)
}

/// Locate the sidecar binary by trying multiple paths.
/// Supports both onefile (single binary) and onedir (directory) layouts.
fn find_sidecar_path(app: &App, base_name: &str) -> Option<std::path::PathBuf> {
    let filename = sidecar_filename(base_name);

    // Candidate 1: resource_dir/sidecars/<name>/<binary> (onedir, .app bundle)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let p = resource_dir
            .join("sidecars")
            .join(base_name)
            .join(&filename);
        if p.exists() {
            return Some(p);
        }
    }

    // Candidate 2: same directory as executable (onefile mode, bare binary)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let p = exe_dir.join(&filename);
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Start the Python sidecar server (release build only).
/// Optimized to minimize OS process-spawning overhead.
#[cfg(not(dev))]
fn start_python_sidecar(app: &tauri::App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let binary = match find_sidecar_path(app, "fstdict-server") {
        Some(path) => path,
        None => {
            warn!("Python sidecar 'fstdict-server' not found — skipping");
            return Ok(None);
        }
    };

    info!("Starting Python server from: {:?}", binary);

    use std::process::Command;
    let mut cmd = Command::new(&binary);

    // ─────────────────────────────────────────────────────────────────
    // WINDOWS OPTIMIZATIONS
    // ─────────────────────────────────────────────────────────────────
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 0x08000000 = CREATE_NO_WINDOW. Disables console subsystem allocation.
        // Prevents execution speed drops caused by implicit console management.
        cmd.creation_flags(0x08000000);
    }

    // ─────────────────────────────────────────────────────────────────
    // UNIX OPTIMIZATIONS (macOS / Linux)
    // ─────────────────────────────────────────────────────────────────
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // 1. Spawn in a new process group to kill the whole tree at once.
        cmd.process_group(0);

        // 2. Performance Fix: Decouple child standard streams from the Tauri GUI parent.
        // If Rust hangs waiting on PyInstaller's initial standard outputs, it delays startup.
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Python server: {}", e))?;

    info!("Python server started (PID: {})", child.id());
    Ok(Some(child))
}

/// Dev mode stub — sidecar is not bundled, skip startup entirely.
#[cfg(dev)]
fn start_python_sidecar(_app: &App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    info!("Dev mode — skipping Python sidecar (run backend manually)");
    Ok(None)
}

/// Gracefully terminate the entire Python sidecar process group.
/// Kills both the PyInstaller bootloader and the actual Python child process.
fn stop_python_sidecar(process: &mut Option<Child>) {
    if let Some(mut proc) = process.take() {
        let pid = proc.id();
        info!("Shutting down Python server (process group PID: {})", pid);

        #[cfg(unix)]
        {
            // Send SIGTERM to the entire process group (negative PID)
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            }
            // Give it a moment to exit gracefully, then force kill if needed
            std::thread::sleep(std::time::Duration::from_millis(500));
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }

        // #[cfg(not(unix))]
        // {
        //     let _ = proc.kill();
        // }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt; // Required for creation_flags
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .creation_flags(0x08000000) // 0x08000000 = CREATE_NO_WINDOW
                .status();
        }

        let _ = proc.wait();
        info!("Python server stopped");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .manage(PythonServer(Mutex::new(None)))
        .setup(|app: &mut App| {
            // Initialize logging using Tauri's standard app_log_dir
            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("./logs"));
            init_logging(&log_dir);

            // Ensure app data directory exists
            let app_data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_dir)?;
            info!("App data directory: {:?}", app_data_dir);

            // Start Python sidecar (skipped automatically in dev mode)
            match start_python_sidecar(app) {
                Ok(Some(child)) => {
                    *app.state::<PythonServer>().0.lock().unwrap() = Some(child);
                }
                Ok(None) => {
                    debug!("Python sidecar not started (dev mode or not found)");
                }
                Err(e) => {
                    error!("Failed to start Python server: {}", e);
                    return Err(e);
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application");

    app.run(|app_handle, event: RunEvent| {
        if let RunEvent::ExitRequested { .. } = event {
            let state = app_handle.state::<PythonServer>();
            let mut proc_guard = state.0.lock().unwrap();
            stop_python_sidecar(&mut proc_guard);
        }
    });
}
