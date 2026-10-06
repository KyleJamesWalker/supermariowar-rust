//! The SDL2 backend (docs/ARCHITECTURE_V2.md): smw-platform's traits over SDL2.

use smw_platform::{Clock, Services};

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

pub fn services() -> Services {
    Services { clock: Box::new(Sdl2Clock) }
}
