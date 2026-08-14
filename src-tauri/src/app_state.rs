use std::process::Child;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::mpsc;

/// State wrapper for the Python backend sidecar process handle.
#[derive(Default)]
pub struct PythonServer(pub Mutex<Option<Child>>);

/// State wrapper for the floating helper process handle (macOS only).
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct HelperProcess(pub Mutex<Option<Child>>);

/// State wrapper for the CGEvent server sidecar process handle (macOS only).
#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct CGEventHelperProcess(pub Mutex<Option<Child>>);

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
