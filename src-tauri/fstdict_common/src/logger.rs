use chrono::Local;
use colored::*;
use env_logger::{Builder, Env, WriteStyle};
use log::{Level, LevelFilter, SetLoggerError};
use once_cell::sync::Lazy;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Holds cached log file handle + current log date for daily rotation
struct LogFileState {
    log_dir: PathBuf,
    file_prefix: String,
    current_date: String,
    handle: Option<fs::File>,
}

static LOG_FILE_STATE: Lazy<Mutex<LogFileState>> = Lazy::new(|| {
    Mutex::new(LogFileState {
        log_dir: PathBuf::new(),
        file_prefix: String::new(),
        current_date: String::new(),
        handle: None,
    })
});

/// Initialize logging: colored console + daily rotated file output with cached file handle.
/// Preserves original log format, thread id, source location, and old log pruning.
pub fn init_logging(log_dir: &Path, file_prefix: String) -> io::Result<()> {
    // Force enable ANSI color for stdout, bypass isatty check for tauri dev pipe output
    // colored::control::set_override(true);

    let env = Env::default().filter_or("RUST_LOG", "info");
    let mut builder = Builder::from_env(env);

    // Force env_logger to keep ANSI codes even when piped
    builder.write_style(WriteStyle::Always);

    // Silence noisy third-party crates
    builder
        .filter_module("enigo", LevelFilter::Warn)
        .filter_module("reqwest", LevelFilter::Warn)
        .filter_module("hyper", LevelFilter::Warn)
        .filter_module("hyper_util", LevelFilter::Warn)
        .filter_module("tauri_plugin_updater", LevelFilter::Warn);

    fs::create_dir_all(log_dir)?;
    println!("Log directory: {:?}", log_dir);

    // Clean up expired logs before starting
    prune_old_logs(log_dir, &file_prefix, 3);

    // Initialize global log file state
    {
        let mut state = LOG_FILE_STATE.lock().unwrap();
        state.log_dir = log_dir.to_path_buf();
        state.file_prefix = file_prefix;
        // Trigger opening of today's log file on first write
        state.current_date = String::new();
        state.handle = None;
    }

    builder.format(move |buf, record| {
        let now = Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let date_str = now.format("%Y-%m-%d").to_string();
        let level = record.level();
        let level_str = format!("{:>5}", level.as_str());

        let thread_id = format!("{:?}", std::thread::current().id());
        let thread_str = thread_id.replace("ThreadId(", "").replace(')', "");
        let thread_short = format!("{:>2}", thread_str);

        let file_loc = match (record.file(), record.line()) {
            (Some(file), Some(line)) => {
                let filename = file.rsplit('/').next().unwrap_or(file);
                format!("{}:{}", filename, line)
            }
            _ => "-".to_string(),
        };

        // Colored console output
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

        // Plain file output (no ANSI colors), use cached handle with daily rotation
        if let Ok(mut state) = LOG_FILE_STATE.lock() {
            // Reopen file if date changed or handle is missing
            if state.current_date != date_str || state.handle.is_none() {
                let log_path = state
                    .log_dir
                    .join(format!("{}-{}.log", state.file_prefix, date_str));

                let new_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .write(true)
                    .open(&log_path);

                match new_file {
                    Ok(file) => {
                        state.handle = Some(file);
                        state.current_date = date_str;
                    }
                    Err(e) => {
                        eprintln!("Failed to open daily log file {:?}: {}", log_path, e);
                        state.handle = None;
                    }
                }
            }

            // Write log line if handle exists
            if let Some(ref mut file) = state.handle {
                let file_line = format!(
                    "{} [{}] [thread {}] [{}] {}\n",
                    time_str,
                    level_str,
                    thread_short,
                    file_loc,
                    record.args()
                );
                let _ = file.write_all(file_line.as_bytes());
                // Flush: tradeoff, remove flush for higher throughput; keep for realtime log
                let _ = file.flush();
            }
        }

        Ok(())
    });

    // Convert SetLoggerError into io::Error
    builder.try_init().map_err(|e: SetLoggerError| {
        io::Error::new(io::ErrorKind::Other, format!("Failed to set logger: {}", e))
    })?;

    Ok(())
}

/// Get or create the daily log file handle (Deprecated, replaced by global cached state)
#[allow(dead_code)]
fn daily_log_file(_log_dir: &Path, _prefix: &str) -> io::Result<fs::File> {
    unreachable!("daily_log_file is replaced by cached LogFileState");
}

/// Scans directory and automatically deletes logs older than max_days
fn prune_old_logs(log_dir: &Path, prefix: &str, max_days: u64) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    let seconds_threshold = max_days * 24 * 60 * 60;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
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
