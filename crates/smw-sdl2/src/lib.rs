//! The SDL2 backend (docs/ARCHITECTURE_V2.md): smw-platform's traits over SDL2.

use smw_platform::{Clock, Services, Storage};
use std::path::PathBuf;

mod audio;
pub mod events;
pub mod input;
mod video;
pub use audio::Sdl2Audio;
pub use video::Sdl2Video;

pub struct Sdl2Clock;

impl Clock for Sdl2Clock {
    /// `SDL_GetTicks`, which wraps at 2^32 ms; the game's frame timing relies on that wraparound.
    fn now_ms(&self) -> u64 {
        unsafe { sdl2::sys::SDL_GetTicks() as u64 }
    }

    fn sleep_ms(&self, ms: u32) {
        unsafe { sdl2::sys::SDL_Delay(ms) }
    }
}

pub struct Sdl2Storage;

impl Storage for Sdl2Storage {
    fn settings_dir(&self) -> PathBuf {
        PathBuf::from(home_directory())
    }

    fn root_dir(&self) -> PathBuf {
        #[cfg(not(windows))]
        if let Ok(p) = sdl2::filesystem::base_path() {
            return PathBuf::from(p);
        }
        PathBuf::from("./")
    }
}

#[cfg(target_os = "android")]
fn home_directory() -> String {
    android_storage_path() + "/"
}

#[cfg(not(any(windows, target_os = "android")))]
fn home_directory() -> String {
    let mut result = String::from("/Library/Preferences/.smw/");
    if let Ok(folder) = std::env::var("HOME") {
        result = folder + &result;
    }
    result
}

/// `SHGetFolderPathA(CSIDL_PROFILE)` is the profile directory that `USERPROFILE` names.
#[cfg(windows)]
fn home_directory() -> String {
    let mut result = String::from(".smw/");
    if let Ok(folder) = std::env::var("USERPROFILE") {
        result = folder + "/" + &result;
    }
    result
}

/// The app's external files directory, as upstream's `$EXTERNAL_STORAGE/supermariowar`; internal storage when it
/// is unavailable. MainActivity extracts the APK's data/ there.
#[cfg(target_os = "android")]
pub fn android_storage_path() -> String {
    use std::ffi::{c_char, CStr};
    extern "C" {
        fn SDL_AndroidGetExternalStoragePath() -> *const c_char;
        fn SDL_AndroidGetInternalStoragePath() -> *const c_char;
    }
    unsafe {
        let mut path = SDL_AndroidGetExternalStoragePath();
        if path.is_null() {
            path = SDL_AndroidGetInternalStoragePath();
        }
        if path.is_null() {
            return ".".to_string();
        }
        CStr::from_ptr(path).to_string_lossy().into_owned()
    }
}

pub fn services() -> Services {
    Services { clock: Box::new(Sdl2Clock), storage: Box::new(Sdl2Storage), video: Box::new(Sdl2Video::new()), audio: Box::new(Sdl2Audio::new()) }
}
