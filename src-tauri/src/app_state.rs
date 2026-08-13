use std::process::Child;
use std::sync::Mutex;

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
