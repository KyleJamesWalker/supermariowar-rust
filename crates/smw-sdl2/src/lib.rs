//! The SDL2 backend (docs/design/ARCHITECTURE_V2.md): smw-platform's traits over SDL2.

use smw_platform::{Clock, Services, Storage};
use std::path::PathBuf;

mod audio;
pub mod durations;
pub mod events;
pub mod input;
pub mod keymap;
#[cfg(not(target_os = "emscripten"))]
mod pad;
mod pads;
#[cfg(feature = "shaders")]
mod shader;
mod video;
pub use audio::Sdl2Audio;
pub use pads::Sdl2Pads;
pub use video::{button_to_logical, Sdl2Video};

/// SDL's millisecond ticks and performance counter.
pub struct Sdl2Clock;

impl Clock for Sdl2Clock {
    /// `SDL_GetTicks`, which wraps at 2^32 ms; the game's frame timing relies on that wraparound.
    fn now_ms(&self) -> u64 {
        // SAFETY: SDL's timer functions take no pointers and work before `SDL_Init`.
        unsafe { sdl2::sys::SDL_GetTicks() as u64 }
    }

    fn now_us(&self) -> u64 {
        // SAFETY: as above.
        unsafe { (sdl2::sys::SDL_GetPerformanceCounter() as u128 * 1_000_000 / sdl2::sys::SDL_GetPerformanceFrequency().max(1) as u128) as u64 }
    }

    fn sleep_ms(&self, ms: u32) {
        // SAFETY: as above.
        unsafe { sdl2::sys::SDL_Delay(ms) }
    }
}

/// Settings in the user's profile directory, data next to the executable.
pub struct Sdl2Storage;

impl Storage for Sdl2Storage {
    fn settings_dir(&self) -> PathBuf {
        PathBuf::from(home_directory())
    }

    /// SDL's base path; the working directory on Windows or when SDL cannot tell.
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
        result = folder + result.as_str();
    }
    result
}

/// `%USERPROFILE%/.smw/`.
#[cfg(windows)]
fn home_directory() -> String {
    let mut result = String::from(".smw/");
    if let Ok(folder) = std::env::var("USERPROFILE") {
        result = folder + "/" + result.as_str();
    }
    result
}

/// The app's external files directory, or internal storage when it is unavailable. MainActivity extracts the
/// APK's data/ there.
#[cfg(target_os = "android")]
pub fn android_storage_path() -> String {
    use std::ffi::{c_char, CStr};
    extern "C" {
        fn SDL_AndroidGetExternalStoragePath() -> *const c_char;
        fn SDL_AndroidGetInternalStoragePath() -> *const c_char;
    }
    // SAFETY: SDL returns null or a NUL-terminated path it owns until it quits.
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

/// The SDL2 backend, for `smw::services::set_backend`.
pub fn services() -> Services {
    Services {
        clock: Box::new(Sdl2Clock),
        storage: Box::new(Sdl2Storage),
        video: Box::new(Sdl2Video::new()),
        audio: Box::new(Sdl2Audio::new()),
        events: Box::new(events::Sdl2Events),
        keyboard: Box::new(events::Sdl2Keyboard),
        pads: Box::new(Sdl2Pads::new()),
    }
}
