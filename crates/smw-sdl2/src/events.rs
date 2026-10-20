//! The SDL2 event queue and keyboard state as smw-platform's `Events` and `Keyboard`, and thin wrappers over the
//! raw SDL event calls.

use sdl2::sys::{SDL_Event, SDL_EventFilter, SDL_bool};
use std::ffi::c_void;

/// `SDL_PollEvent`.
pub unsafe fn poll(event: *mut SDL_Event) -> i32 {
    sdl2::sys::SDL_PollEvent(event)
}

/// The SDL event queue as the game's input.
pub struct Sdl2Events;

impl smw_platform::Events for Sdl2Events {
    fn poll(&mut self) -> Option<smw_platform::Input> {
        poll_input()
    }
    fn wait(&mut self) -> smw_platform::Input {
        // SAFETY: an all-zero `SDL_Event` is valid, and SDL fills it before `from_sdl` reads it.
        unsafe {
            let mut event: SDL_Event = std::mem::zeroed();
            wait(&mut event);
            crate::input::from_sdl(&event)
        }
    }
    fn push(&mut self, input: &smw_platform::Input) {
        let mut event = crate::input::to_sdl(input);
        // SAFETY: SDL copies the event before returning.
        unsafe { push(&mut event) };
    }
    fn pump(&mut self) {
        pump();
    }
}

/// SDL's keyboard and mouse state.
pub struct Sdl2Keyboard;

impl smw_platform::Keyboard for Sdl2Keyboard {
    fn scancode_from_key(&self, key: i32) -> i32 {
        // SAFETY: a pure table lookup; unknown keys give 0.
        unsafe { sdl2::sys::SDL_GetScancodeFromKey(key) as i32 }
    }
    fn key_from_name(&self, name: &str) -> i32 {
        let Ok(c) = std::ffi::CString::new(name) else { return 0 };
        // SAFETY: `c` is NUL-terminated and outlives the call.
        unsafe { sdl2::sys::SDL_GetKeyFromName(c.as_ptr()) }
    }
    fn key_name(&self, key: i32) -> String {
        // SAFETY: SDL always returns a NUL-terminated string, empty for an unknown key.
        unsafe { std::ffi::CStr::from_ptr(sdl2::sys::SDL_GetKeyName(key)).to_string_lossy().into_owned() }
    }
    fn state(&self) -> &[u8] {
        // SAFETY: SDL's state array has `n` entries and lives until SDL quits; it changes only when events are pumped.
        unsafe {
            let mut n = 0;
            let p = sdl2::sys::SDL_GetKeyboardState(&mut n);
            std::slice::from_raw_parts(p, n as usize)
        }
    }
    fn mouse_buttons(&self) -> u32 {
        // SAFETY: SDL accepts null for the positions it should not write.
        unsafe { sdl2::sys::SDL_GetMouseState(std::ptr::null_mut(), std::ptr::null_mut()) }
    }
    fn has_focus(&self) -> bool {
        // SAFETY: takes no arguments; the window pointer is only compared with null.
        unsafe { !sdl2::sys::SDL_GetKeyboardFocus().is_null() }
    }
}

/// `SDL_PollEvent`, normalized.
pub fn poll_input() -> Option<smw_platform::Input> {
    // SAFETY: an all-zero `SDL_Event` is valid, and `from_sdl` reads it only when SDL filled it.
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
    // SAFETY: takes no arguments; SDL requires the thread that set up video.
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
