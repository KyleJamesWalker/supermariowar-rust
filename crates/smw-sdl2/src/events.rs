//! The SDL2 event queue. The core reads and feeds input only through these until phase 3's InputEvent replaces
//! SDL_Event at this boundary (docs/ARCHITECTURE_V2.md, Normalized input).

use sdl2::sys::{SDL_Event, SDL_EventFilter, SDL_bool};
use std::ffi::c_void;

/// `SDL_PollEvent`.
pub unsafe fn poll(event: *mut SDL_Event) -> i32 {
    sdl2::sys::SDL_PollEvent(event)
}

/// `SDL_PollEvent`, normalized.
/// The SDL event queue as the game's input.
pub struct Sdl2Events;

impl smw_platform::Events for Sdl2Events {
    fn poll(&mut self) -> Option<smw_platform::Input> {
        poll_input()
    }
}

/// SDL's keyboard and mouse state.
pub struct Sdl2Keyboard;

impl smw_platform::Keyboard for Sdl2Keyboard {
    fn scancode_from_key(&self, key: i32) -> i32 {
        unsafe { sdl2::sys::SDL_GetScancodeFromKey(key) as i32 }
    }
    fn key_from_name(&self, name: &str) -> i32 {
        let Ok(c) = std::ffi::CString::new(name) else { return 0 };
        unsafe { sdl2::sys::SDL_GetKeyFromName(c.as_ptr()) }
    }
    fn key_name(&self, key: i32) -> String {
        unsafe { std::ffi::CStr::from_ptr(sdl2::sys::SDL_GetKeyName(key)).to_string_lossy().into_owned() }
    }
    fn state(&self) -> &[u8] {
        unsafe {
            let mut n = 0;
            let p = sdl2::sys::SDL_GetKeyboardState(&mut n);
            std::slice::from_raw_parts(p, n as usize)
        }
    }
    fn mouse_buttons(&self) -> u32 {
        unsafe { sdl2::sys::SDL_GetMouseState(std::ptr::null_mut(), std::ptr::null_mut()) }
    }
    fn has_focus(&self) -> bool {
        unsafe { !sdl2::sys::SDL_GetKeyboardFocus().is_null() }
    }
}

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
