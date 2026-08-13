use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tokio::sync::mpsc;

/// Pin state and WebSocket sender for the selection search panel.
pub struct SelectionWindowPinState {
    pub is_pinned: AtomicBool,
    pub ws_sender: mpsc::Sender<String>,
}

/// Pin state and WebSocket sender for the main helper panel.
pub struct MainWindowPinState {
    pub is_pinned: AtomicBool,
    pub ws_sender: mpsc::Sender<String>,
}

impl SelectionWindowPinState {
    pub fn new(sender: mpsc::Sender<String>) -> Self {
        Self {
            is_pinned: AtomicBool::new(false),
            ws_sender: sender,
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

impl MainWindowPinState {
    pub fn new(sender: mpsc::Sender<String>) -> Self {
        Self {
            is_pinned: AtomicBool::new(false),
            ws_sender: sender,
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

