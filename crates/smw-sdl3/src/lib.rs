//! The SDL3 backend (docs/design/ARCHITECTURE_V2.md): smw-platform's traits over SDL3, with keys, scancodes and pad events
//! in SDL2's numbering, which settings files and replays store.

use sdl3_sys::everything::*;
use smw_platform::{Clock, Services, Storage};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

mod audio;
pub mod events;
pub mod input;
pub mod pads;
mod video;
pub use audio::{decode_wav, Sdl3Audio};
pub use events::{Sdl3Events, Sdl3Keyboard};
pub use pads::{PadState, Sdl3Pads};
pub use video::Sdl3Video;

/// SDL's millisecond and nanosecond ticks.
pub struct Sdl3Clock;

impl Clock for Sdl3Clock {
    fn now_ms(&self) -> u64 {
        // SAFETY: SDL's timer functions take no pointers and work before `SDL_Init`.
        unsafe { SDL_GetTicks() }
    }

    fn now_us(&self) -> u64 {
        // SAFETY: as above.
        unsafe { SDL_GetTicksNS() / 1000 }
    }

    fn sleep_ms(&self, ms: u32) {
        // SAFETY: as above.
        unsafe { SDL_Delay(ms) }
    }
}

/// Settings in the user's profile directory, data next to the executable.
pub struct Sdl3Storage;

impl Storage for Sdl3Storage {
    fn settings_dir(&self) -> PathBuf {
        PathBuf::from(home_directory())
    }

    fn root_dir(&self) -> PathBuf {
        // SAFETY: SDL returns null or a NUL-terminated path it owns until it quits.
        let p = unsafe { SDL_GetBasePath() };
        if p.is_null() {
            PathBuf::from("./")
        } else {
            PathBuf::from(unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
        }
    }
}

/// `$HOME/Library/Preferences/.smw/`.
#[cfg(not(windows))]
fn home_directory() -> String {
    let mut result = String::from("/Library/Preferences/.smw/");
    if let Ok(folder) = std::env::var("HOME") {
        result = folder + &result;
    }
    result
}

/// `%USERPROFILE%/.smw/`.
#[cfg(windows)]
fn home_directory() -> String {
    let mut result = String::from(".smw/");
    if let Ok(folder) = std::env::var("USERPROFILE") {
        result = folder + "/" + &result;
    }
    result
}

/// The SDL3 backend, for `smw::services::set_backend`. Sets SDL's keycode hint, so call it before SDL starts.
pub fn services() -> Services {
    // Key events carry the unshifted key, as bindings store them.
    // SAFETY: both strings are NUL-terminated literals.
    unsafe { SDL_SetHint(SDL_HINT_KEYCODE_OPTIONS, c"unmodified".as_ptr()) };
    let pads = Rc::new(RefCell::new(PadState::default()));
    Services {
        clock: Box::new(Sdl3Clock),
        storage: Box::new(Sdl3Storage),
        video: Box::new(Sdl3Video::new()),
        audio: Box::new(Sdl3Audio::new()),
        events: Box::new(Sdl3Events::new(pads.clone())),
        keyboard: Box::new(Sdl3Keyboard),
        pads: Box::new(Sdl3Pads::new(pads)),
    }
}
