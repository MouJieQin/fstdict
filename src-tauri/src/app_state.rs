use std::process::Child;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tokio::sync::mpsc;

/// State wrapper for the Python backend sidecar process handle.
#[derive(Default)]
pub struct PythonServer(pub Arc<Mutex<Option<Child>>>);

/// State wrapper for the floating helper process handle (macOS only).
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct HelperProcess(pub Arc<Mutex<Option<Child>>>);

/// State wrapper for the CGEvent server sidecar process handle (macOS only).
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct CGEventHelperProcess(pub Arc<Mutex<Option<Child>>>);

// Global sidecar handle registry for signal handler access
// Shares the same Arc<Mutex<>> instances with Tauri state (single source of truth)
pub static GLOBAL_PYTHON_SERVER: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();
#[cfg(target_os = "macos")]
pub static GLOBAL_HELPER_PROCESS: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();
#[cfg(target_os = "macos")]
pub static GLOBAL_CGEVENT_SERVER: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();

/// Tracks timestamps for double-press detection (Cmd/Ctrl + C twice).
pub struct DoubleCopyTracker {
    pub last_pressed: Mutex<Option<Instant>>,
}

impl Default for DoubleCopyTracker {
    fn default() -> Self {
        Self {
            last_pressed: Mutex::new(None),
        }
    }
}

/// Pin state and WebSocket sender for the main helper panel.
pub struct MainWindowWsSender {
    pub ws_sender: mpsc::Sender<String>,
}

impl MainWindowWsSender {
    pub fn new(sender: mpsc::Sender<String>) -> Self {
        Self { ws_sender: sender }
    }
}
