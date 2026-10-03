pub mod main_window;
pub mod manage;
pub mod permission_window;
pub mod setting;
#[cfg(any(feature = "dev-non-macos", not(target_os = "macos")))]
pub mod setup;
pub mod updater_window;
pub mod vibrant_window;
