//! Native CoreAudio alert-volume save/restore via direct FFI (coreaudio-sys).
//! Replaces AppleScript: `alert volume of (get volume settings)`.

use coreaudio_sys::{
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectSetPropertyData,
};
use std::error::Error;
use std::mem::size_of;
use std::ptr::null;

// FourCharCode selectors — hardcoded because coreaudio-sys does not
// re-export every CoreAudio constant under a stable module path.
const SELECTOR_DEFAULT_SYSTEM_OUTPUT: u32 = 0x736F7574; // 'sout'
const SELECTOR_ALERT_VOLUME: u32 = 0x616C766C; // 'alvl'
const SYSTEM_OBJECT: AudioObjectID = 1; // kAudioObjectSystemObject

/// Read a scalar AudioObject property.
unsafe fn get_scalar<T>(object_id: AudioObjectID, selector: u32) -> Result<T, Box<dyn Error>> {
    let address = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: 0, // kAudioObjectPropertyScopeGlobal
        mElement: 0,
    };
    let mut data_size = size_of::<T>() as u32;
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    let status = AudioObjectGetPropertyData(
        object_id,
        &address,
        0,
        null(),
        &mut data_size,
        value.as_mut_ptr() as *mut _,
    );
    if status != 0 {
        return Err(
            format!("AudioObjectGetPropertyData(0x{selector:08x}) OSStatus={status}").into(),
        );
    }
    Ok(value.assume_init())
}

/// Write a scalar AudioObject property.
unsafe fn set_scalar<T>(
    object_id: AudioObjectID,
    selector: u32,
    value: &T,
) -> Result<(), Box<dyn Error>> {
    let address = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: 0,
        mElement: 0,
    };
    let data_size = size_of::<T>() as u32;
    let status = AudioObjectSetPropertyData(
        object_id,
        &address,
        0,
        null(),
        data_size,
        value as *const T as *const _,
    );
    if status != 0 {
        return Err(
            format!("AudioObjectSetPropertyData(0x{selector:08x}) OSStatus={status}").into(),
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct AlertVolumeSnapshot {
    device: AudioObjectID,
    volume: f32,
}

impl AlertVolumeSnapshot {
    /// Capture current system alert volume.
    pub fn capture() -> Result<Self, Box<dyn Error>> {
        unsafe {
            let device: AudioObjectID = get_scalar(SYSTEM_OBJECT, SELECTOR_DEFAULT_SYSTEM_OUTPUT)?;
            let volume: f32 = get_scalar(device, SELECTOR_ALERT_VOLUME)?;
            Ok(Self { device, volume })
        }
    }

    /// Mute alert volume (suppress Cmd+C beep in some apps).
    pub fn mute(&self) -> Result<(), Box<dyn Error>> {
        unsafe { set_scalar(self.device, SELECTOR_ALERT_VOLUME, &0.0f32) }
    }

    /// Restore alert volume to the captured value.
    pub fn restore(&self) -> Result<(), Box<dyn Error>> {
        unsafe { set_scalar(self.device, SELECTOR_ALERT_VOLUME, &self.volume) }
    }
}
