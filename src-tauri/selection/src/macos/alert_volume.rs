use coreaudio_sys::{
    kAudioServicesNoError, AudioServicesGetProperty, AudioServicesPropertyID,
    AudioServicesSetProperty,
};
use std::error::Error;
use std::mem::size_of;
use std::ptr::null;

// The undocumented selector for "System Alert Volume" (matches 'ssvl')
// This controls the "Alert volume" slider in System Settings > Sound.
const K_AUDIO_SERVICES_PROPERTY_SYSTEM_VOLUME: AudioServicesPropertyID = 0x7373766c;

#[derive(Clone, Copy)]
pub struct AlertVolumeSnapshot {
    volume: f32,
}

impl AlertVolumeSnapshot {
    /// Capture the current system alert volume.
    pub fn capture() -> Result<Self, Box<dyn Error>> {
        let mut size = size_of::<f32>() as u32;
        let mut volume: f32 = 0.0;

        let status = unsafe {
            AudioServicesGetProperty(
                K_AUDIO_SERVICES_PROPERTY_SYSTEM_VOLUME,
                0,
                null(),
                &mut size,
                &mut volume as *mut f32 as *mut _,
            )
        };

        if status != kAudioServicesNoError as i32 {
            return Err(format!("Failed to get alert volume. OSStatus: {}", status).into());
        }

        Ok(Self { volume })
    }

    /// Mute the alert volume (sets it to 0.0).
    pub fn mute(&self) -> Result<(), Box<dyn Error>> {
        self.set_volume(0.0)
    }

    /// Restore the alert volume to the captured value.
    pub fn restore(&self) -> Result<(), Box<dyn Error>> {
        self.set_volume(self.volume)
    }

    fn set_volume(&self, volume: f32) -> Result<(), Box<dyn Error>> {
        let size = size_of::<f32>() as u32;
        let status = unsafe {
            AudioServicesSetProperty(
                K_AUDIO_SERVICES_PROPERTY_SYSTEM_VOLUME,
                0,
                null(),
                size,
                &volume as *const f32 as *const _,
            )
        };

        if status != kAudioServicesNoError as i32 {
            return Err(format!("Failed to set alert volume. OSStatus: {}", status).into());
        }
        Ok(())
    }
}
