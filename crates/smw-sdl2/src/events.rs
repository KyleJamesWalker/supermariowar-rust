//! The SDL2 event queue. The core reads and feeds input only through these until phase 3's InputEvent replaces
//! SDL_Event at this boundary (docs/ARCHITECTURE_V2.md, Normalized input).

use sdl2::sys::{SDL_Event, SDL_EventFilter, SDL_bool};
use std::ffi::c_void;

/// `SDL_PollEvent`.
pub unsafe fn poll(event: *mut SDL_Event) -> i32 {
    sdl2::sys::SDL_PollEvent(event)
}

/// `SDL_PollEvent` that leaves `*event` as is on an empty queue, as sdl2-compat does. SDL2 writes its poll
/// sentinel there, and the editors read `event` after their poll loops.
pub unsafe fn poll_keep_last(event: *mut SDL_Event) -> i32 {
    let mut polled: SDL_Event = std::mem::zeroed();
    let result = sdl2::sys::SDL_PollEvent(&mut polled);
    if result != 0 {
        *event = polled;
    }
    result
}

/// `SDL_PollEvent`, normalized.
pub fn poll_input() -> Option<smw_platform::Input> {
    unsafe {
        let mut event: SDL_Event = std::mem::zeroed();
        (sdl2::sys::SDL_PollEvent(&mut event) != 0).then(|| crate::input::from_sdl(&event))
    }
}

/// `SDL_WaitEvent`.
pub unsafe fn wait(event: *mut SDL_Event) -> i32 {
    sdl2::sys::SDL_WaitEvent(event)
}

/// `SDL_PushEvent`.
pub unsafe fn push(event: *mut SDL_Event) -> i32 {
    sdl2::sys::SDL_PushEvent(event)
}

/// `SDL_PumpEvents`.
pub fn pump() {
    unsafe { sdl2::sys::SDL_PumpEvents() }
}

/// `SDL_SetEventFilter`.
pub unsafe fn set_filter(filter: SDL_EventFilter, userdata: *mut c_void) {
    sdl2::sys::SDL_SetEventFilter(filter, userdata)
}

/// `SDL_GetEventFilter`.
pub unsafe fn get_filter(filter: *mut SDL_EventFilter, userdata: *mut *mut c_void) -> SDL_bool {
    sdl2::sys::SDL_GetEventFilter(filter, userdata)
}
