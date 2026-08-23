use chrono::Local;
use colored::*;
use env_logger::{Builder, Env};
use log::{Level, LevelFilter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

/// Initialize logging: colored console + daily rotated file output.
/// FIX: Changed file_prefix parameter from `&str` to an owned `String`
pub fn init_logging(log_dir: &Path, file_prefix: String) {
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

    // Clone the prefix for the cleanup routine before moving it into the formatting loop
    prune_old_logs(log_dir, &file_prefix, 7);

    let log_dir_buf = log_dir.to_path_buf();

    // The 'move' keyword now safely moves ownership of the 'file_prefix' String into the closure
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
        // FIX: Pass the owned file_prefix context parameter to daily_log_file safely
        if let Ok(mut file) = daily_log_file(&log_dir_buf, &file_prefix) {
            let file_line = format!(
                "{} [{}] [thread {}] [{}] {}\n",
                time_str,
                level_str,
                thread_short,
                file_loc,
                record.args()
            );
            let _ = file.write_all(file_line.as_bytes());
            let _ = file.flush();
        }

        Ok(())
    });

    let _ = builder.try_init();
}

/// Get or create the daily log file handle
fn daily_log_file(log_dir: &Path, prefix: &str) -> std::io::Result<fs::File> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    // Uses the customized layout prefix naming scheme dynamically
    let log_file = log_dir.join(format!("{}-{}.log", prefix, today));

    OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(log_file)
}

/// Scans the directory and automatically deletes logs older than max_days
fn prune_old_logs(log_dir: &Path, prefix: &str, max_days: u64) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    let seconds_threshold = max_days * 24 * 60 * 60;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                // Safely maps target criteria based on the runtime initialization variable
                if filename.starts_with(prefix) && filename.ends_with(".log") {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = SystemTime::now().duration_since(modified) {
                                if duration.as_secs() > seconds_threshold {
                                    println!("Pruning expired log archive: {:?}", filename);
                                    let _ = fs::remove_file(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
