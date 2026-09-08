use std::process::Child;
#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
use std::sync::atomic::{AtomicBool, Ordering};
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

// Global sidecar handle registry for signal handler access
// Shares the same Arc<Mutex<>> instances with Tauri state (single source of truth)
pub static GLOBAL_PYTHON_SERVER: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();
#[cfg(target_os = "macos")]
pub static GLOBAL_HELPER_PROCESS: OnceLock<Arc<Mutex<Option<Child>>>> = OnceLock::new();

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

/// Pin state for the selection search panel.
#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub struct HelperSelectionWindowPinState {
    pub is_pinned: AtomicBool,
}

/// Pin state for the main helper panel.
#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub struct HelperMainWindowPinState {
    pub is_pinned: AtomicBool,
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
impl HelperSelectionWindowPinState {
    pub fn new() -> Self {
        Self {
            is_pinned: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.is_pinned.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn set_pinned(&self, pinned: bool) {
        self.is_pinned.store(pinned, Ordering::SeqCst);
    }
}

#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
impl HelperMainWindowPinState {
    pub fn new() -> Self {
        Self {
            is_pinned: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.is_pinned.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn set_pinned(&self, pinned: bool) {
        self.is_pinned.store(pinned, Ordering::SeqCst);
    }
}
