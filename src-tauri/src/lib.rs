// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use chrono::Local;
use colored::*;
use env_logger::{Builder, Env};
use log::{debug, error, info, warn, Level, LevelFilter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{App, AppHandle, Manager, RunEvent};

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
#[cfg(target_os = "macos")]
struct HelperProcess(Mutex<Option<Child>>);

#[cfg(target_os = "macos")]
/// Locate fstdict-helper binary, support dev & release bundle
fn find_helper_binary() -> Option<PathBuf> {
    // Release bundle: Contents/MacOS/fstdict-helper
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate = exe_dir.join("fstdict-helper");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Dev mode: src-tauri/target/debug/fstdict-helper
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

#[cfg(target_os = "macos")]
fn start_helper() -> Result<Option<Child>, Box<dyn std::error::Error>> {
    // No arguments needed here anymore
    let binary = match find_helper_binary() {
        Some(path) => path,
        None => {
            error!("fstdict-helper binary not found — skip launch");
            return Ok(None);
        }
    };
    info!("Starting fstdict-helper from: {:?}", binary);

    let mut cmd = Command::new(&binary);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Spawn fstdict-helper failed: {}", e))?;
    info!("fstdict-helper started, PID: {}", child.id());
    Ok(Some(child))
}

#[cfg(target_os = "macos")]
fn stop_helper(process: &mut Option<Child>) {
    if let Some(mut proc) = process.take() {
        let pid = proc.id();
        info!("Stopping fstdict-helper, process group PID: {}", pid);
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        match proc.try_wait() {
            Ok(Some(_)) => {}
            _ => unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            },
        }
        let _ = proc.wait();
        info!("fstdict-helper stopped");
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn launch_helper(app_handle: tauri::AppHandle) -> Result<String, String> {
    // 1. Verify accessibility trust status first
    if !macos_accessibility_client::accessibility::application_is_trusted() {
        return Err("Accessibility permission is missing. Cannot spawn helper.".to_string());
    }

    // 2. Lock global managed process state to prevent duplicate spawning loops
    let state = app_handle.state::<HelperProcess>();
    let mut lock = state.0.lock().unwrap();
    if lock.is_some() {
        return Ok("Helper is already running.".to_string());
    }

    // 3. Fire the launch sequence cleanly
    match start_helper() {
        Ok(Some(child)) => {
            *lock = Some(child);
            Ok("Helper started successfully.".to_string())
        }
        Ok(None) => Err("Helper binary could not be located on disk.".to_string()),
        Err(e) => Err(format!("Failed to spawn process: {}", e)),
    }
}

#[cfg(target_os = "macos")]
struct CGEventHelperProcess(Mutex<Option<Child>>);

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

// CHANGED: Accept &AppHandle so this can run inside an invoke command handler context
fn find_sidecar_path_by_app_handle(app: &AppHandle, base_name: &str) -> Option<PathBuf> {
    let filename = sidecar_filename(base_name);

    if let Ok(resource_dir) = app.path().resource_dir() {
        let p = resource_dir
            .join("sidecars")
            .join(base_name)
            .join(&filename);
        if p.exists() {
            return Some(p);
        }
    }

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

#[cfg(target_os = "macos")]
fn start_cgevent_sidecar(app: &AppHandle) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    // Pass the AppHandle reference to your file locator
    let binary = match find_sidecar_path_by_app_handle(app, "fstdict_cgevent_server") {
        Some(path) => path,
        None => {
            log::warn!("Python sidecar 'fstdict_cgevent_server' not found — skipping");
            return Ok(None);
        }
    };

    log::info!("Starting fstdict cgevent server from: {:?}", binary);
    let mut cmd = Command::new(&binary);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    use std::os::unix::process::CommandExt;
    cmd.process_group(0);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn fstdict cgevent server: {}", e))?;

    log::info!(
        "fstdict cgevent server server started (PID: {})",
        child.id()
    );
    Ok(Some(child))
}

#[cfg(target_os = "macos")]
fn stop_cgevent_sidecar(process: &mut Option<Child>) {
    if let Some(mut proc) = process.take() {
        let pid = proc.id();
        log::info!(
            "Shutting fstdict cgevnt server (process group PID: {})",
            pid
        );
        // Send SIGTERM to the entire process group (negative PID)
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
        // Give it 200ms to exit gracefully, then force kill
        std::thread::sleep(std::time::Duration::from_millis(200));
        // Check if it's still alive before kill (optional but cleaner)
        match proc.try_wait() {
            Ok(Some(_)) => {} // Already dead
            _ => unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            },
        }
        // Avoid blocking the UI thread for too long
        let _ = proc.wait();
        log::info!("fstdict cgevnt server stopped");
    }
}

#[cfg(not(dev))]
fn start_python_sidecar(app: &App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    let binary = match find_sidecar_path(app, "fstdict-server") {
        Some(path) => path,
        None => {
            log::warn!("Python sidecar 'fstdict-server' not found — skipping");
            return Ok(None);
        }
    };

    log::info!("Starting Python server from: {:?}", binary);
    let mut cmd = Command::new(&binary);

    // ─────────────────────────────────────────────────────────────────
    // OPTIMIZATION 1: Fix Launch Lag (macOS & Windows)
    // ─────────────────────────────────────────────────────────────────
    // PyInstaller's bootloader tries to detect a parent console. If connected
    // to a GUI app, it hangs for seconds trying to initialize stdout/stderr.
    // Piping to null bypasses this check instantly.
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    // ─────────────────────────────────────────────────────────────────
    // OPTIMIZATION 2: Windows Invisible Launch
    // ─────────────────────────────────────────────────────────────────
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000): Prevents a transient console window
        // from appearing even for a split second.
        cmd.creation_flags(0x08000000);
    }

    // ─────────────────────────────────────────────────────────────────
    // SAFETY: Unix Process Groups
    // ─────────────────────────────────────────────────────────────────
    #[cfg(unix)]
    {
        // Create a new process group so we can kill the whole tree (bootloader + python)
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Python server: {}", e))?;

    log::info!("Python server started (PID: {})", child.id());
    Ok(Some(child))
}

/// Dev mode stub — sidecar is not bundled, skip startup entirely.
#[cfg(dev)]
fn start_python_sidecar(_app: &App) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    info!("Dev mode — skipping Python sidecar (run backend manually)");
    Ok(None)
}

fn stop_python_sidecar(process: &mut Option<Child>) {
    if let Some(mut proc) = process.take() {
        let pid = proc.id();
        log::info!("Shutting down Python server (process group PID: {})", pid);

        #[cfg(unix)]
        {
            // Send SIGTERM to the entire process group (negative PID)
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            }
            // Give it 200ms to exit gracefully, then force kill
            std::thread::sleep(std::time::Duration::from_millis(200));
            // Check if it's still alive before kill (optional but cleaner)
            match proc.try_wait() {
                Ok(Some(_)) => {} // Already dead
                _ => unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                },
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // FIX: Add creation_flags(0x08000000) to hide the "Black Terminal Window"
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x08000000)
                .status();
        }

        // Avoid blocking the UI thread for too long
        let _ = proc.wait();
        log::info!("Python server stopped");
    }
}

#[cfg(target_os = "macos")]
use macos_accessibility_client::accessibility::{
    application_is_trusted, application_is_trusted_with_prompt,
};

// Only compiled and registered when building for macOS
#[cfg(target_os = "macos")]
#[tauri::command]
fn check_accessibility() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn request_accessibility() -> bool {
    use macos_accessibility_client::accessibility::application_is_trusted_with_prompt;
    let is_trusted = application_is_trusted_with_prompt();
    if !is_trusted {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
    is_trusted
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn launch_cgevent_server(app_handle: AppHandle) -> Result<String, String> {
    // 1. Double check permission constraints before spawning the sidecar loop
    if !macos_accessibility_client::accessibility::application_is_trusted() {
        return Err("Accessibility permission is missing. Cannot spawn sidecar.".to_string());
    }

    // 2. Lock managed state to prevent duplicate runtime spawns
    let state = app_handle.state::<CGEventHelperProcess>();
    let mut lock = state.0.lock().unwrap();
    if lock.is_some() {
        return Ok("CgEvent sidecar is already running.".to_string());
    }

    // 3. Fire the launch protocol using the shared app handle wrapper context
    match start_cgevent_sidecar(&app_handle) {
        Ok(Some(child)) => {
            *lock = Some(child);
            Ok("CgEvent sidecar started successfully.".to_string())
        }
        Ok(None) => Err("Sidecar binary file path matching lookup failed.".to_string()),
        Err(e) => Err(format!("Process spawn runtime exception: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .manage(PythonServer(Mutex::new(None)));

    // ======================【修正】只在 macOS 上注册 HelperProcess 状态托管 ======================
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .invoke_handler(tauri::generate_handler![
                check_accessibility,
                request_accessibility,
                launch_helper,
                launch_cgevent_server
            ])
            .manage(CGEventHelperProcess(Mutex::new(None)))
            .manage(HelperProcess(Mutex::new(None)));
    }

    let app = builder
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

            // ====================== 启动 Helper (macOS) =====================
            #[cfg(target_os = "macos")]
            {
                if macos_accessibility_client::accessibility::application_is_trusted() {
                    match start_cgevent_sidecar(app.handle()) {
                        Ok(Some(child)) => {
                            *app.state::<CGEventHelperProcess>().0.lock().unwrap() = Some(child);
                        }
                        Ok(None) => warn!("Cgevent server sidecar binary missing at startup"),
                        Err(e) => {
                            error!("Failed to start cgevent server: {}", e);
                            return Err(e);
                        }
                    }

                    match start_helper() {
                        Ok(Some(child)) => {
                            *app.state::<HelperProcess>().0.lock().unwrap() = Some(child);
                        }
                        Ok(None) => warn!("Helper binary missing at startup"),
                        Err(e) => error!("Start helper at startup failed: {}", e),
                    }
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application");

    app.run(|app_handle, event| {
        match event {
            // FIX: Handle BOTH ExitRequested and Exit.
            // ExitRequested catches standard closes.
            // Exit catches Cmd+Q on macOS and other forced terminations.
            RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                // Fix lifetime by locking directly on the state extraction in one clean step
                if let Ok(mut proc_guard) = app_handle.state::<PythonServer>().0.lock() {
                    stop_python_sidecar(&mut proc_guard);
                }

                // ======================【修正】退出时关闭 Helper 同样使用平台宏保护 ======================
                #[cfg(target_os = "macos")]
                {
                    if let Ok(mut helper_guard) = app_handle.state::<HelperProcess>().0.lock() {
                        stop_helper(&mut helper_guard);
                    }
                    if let Ok(mut cgevent_server_guard) =
                        app_handle.state::<CGEventHelperProcess>().0.lock()
                    {
                        stop_cgevent_sidecar(&mut cgevent_server_guard);
                    }
                }
            }
            _ => {}
        }
    });
}
