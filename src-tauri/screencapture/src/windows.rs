use image::{ImageBuffer, ImageFormat, RgbaImage};
use interactive_screenshot::{start_capture, CaptureOutcome};
use log::{error, info};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::Path;

pub fn interactively_capture(png_path: &str) -> Result<Option<()>, String> {
    match start_capture() {
        Ok(CaptureOutcome::Captured {
            rgba,
            width,
            height,
        }) => {
            // 1. Wrap raw bytes into a managed image buffer wrapper
            let img_buffer: RgbaImage = ImageBuffer::from_raw(width, height, rgba)
                .ok_or_else(|| "Failed to create image buffer from raw RGBA data".to_string())?;

            // 2. Encode to PNG bytes in memory first
            let mut png_bytes = Vec::new();
            img_buffer
                .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
                .map_err(|e| format!("Failed to encode PNG bytes: {}", e))?;

            // 3. Forcibly overwrite the file on disk by truncating the lock target
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true) // Destroys previous content if file exists
                .open(Path::new(png_path))
                .map_err(|e| {
                    format!(
                        "Failed to open file (might be locked by another process): {}",
                        e
                    )
                })?;

            file.write_all(&png_bytes)
                .map_err(|e| format!("Failed to write PNG data to disk: {}", e))?;

            info!("Screenshot forced and saved successfully to {}", png_path);
            Ok(Some(()))
        }
        Ok(CaptureOutcome::Cancelled) => {
            info!("User cancelled the screenshot sequence.");
            Ok(None)
        }
        Err(e) => {
            error!("Screenshot overlay error: {:?}", e);
            Err(format!("Screenshot overlay error: {:?}", e))
        }
    }
}
