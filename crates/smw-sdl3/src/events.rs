//! The SDL3 event queue and keyboard, in SDL2's numbering (input.rs).

use crate::input::{from_sdl, key_from_sdl2, key_to_sdl2, scancode_to_sdl2, to_sdl, TextArena};
use crate::pads::PadState;
use sdl3_sys::everything::*;
use smw_platform::{Events, Input, Keyboard};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;

pub struct Sdl3Events {
    pads: Rc<RefCell<PadState>>,
    texts: TextArena,
}

impl Sdl3Events {
    /// Translates pad events through `pads`, which `Sdl3Pads` shares.
    pub fn new(pads: Rc<RefCell<PadState>>) -> Self {
        Sdl3Events { pads, texts: TextArena::new() }
    }

    /// The event as the game reads it, `None` when translation drops it.
    fn read(&mut self, mut e: SDL_Event) -> Option<Input> {
        unsafe {
            let t = e.r#type;
            if t == SDL_EVENT_MOUSE_MOTION.0 || t == SDL_EVENT_MOUSE_BUTTON_DOWN.0 || t == SDL_EVENT_MOUSE_BUTTON_UP.0 || t == SDL_EVENT_MOUSE_WHEEL.0 {
                // SDL2's renderer moved mouse events into its logical 640x480 on its own; SDL3 leaves that to the app.
                let renderer = SDL_GetRenderer(SDL_GetWindowFromID(e.motion.windowID));
                if !renderer.is_null() {
                    SDL_ConvertEventToRenderCoordinates(renderer, &mut e);
                }
            }
        }
        self.pads.borrow_mut().translate(from_sdl(&e))
    }
}

impl Events for Sdl3Events {
    fn poll(&mut self) -> Option<Input> {
        loop {
            let mut e = SDL_Event { r#type: 0 };
            if !unsafe { SDL_PollEvent(&mut e) } {
                return None;
            }
            if let Some(input) = self.read(e) {
                return Some(input);
            }
        }
    }

    fn wait(&mut self) -> Input {
        loop {
            let mut e = SDL_Event { r#type: 0 };
            if unsafe { SDL_WaitEvent(&mut e) } {
                if let Some(input) = self.read(e) {
                    return input;
                }
            }
        }
    }

    fn push(&mut self, input: &Input) {
        let mut e = to_sdl(input, &mut self.texts);
        unsafe { SDL_PushEvent(&mut e) };
    }

    fn pump(&mut self) {
        unsafe { SDL_PumpEvents() }
    }
}

/// SDL3's keyboard and mouse state, with keys and scancodes in SDL2's values.
pub struct Sdl3Keyboard;

impl Keyboard for Sdl3Keyboard {
    fn scancode_from_key(&self, key: i32) -> i32 {
        let scancode = unsafe { SDL_GetScancodeFromKey(SDL_Keycode(key_from_sdl2(key)), std::ptr::null_mut()) };
        scancode_to_sdl2(scancode.0)
    }

    /// A single character names its own key, as in SDL2; SDL3 would map `!` to the `1` key.
    fn key_from_name(&self, name: &str) -> i32 {
        let mut chars = name.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            return c.to_ascii_lowercase() as i32;
        }
        let Ok(c) = CString::new(name) else { return 0 };
        key_to_sdl2(unsafe { SDL_GetKeyFromName(c.as_ptr()) }.0)
    }

    fn key_name(&self, key: i32) -> String {
        let name = unsafe { SDL_GetKeyName(SDL_Keycode(key_from_sdl2(key))) };
        if name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
        }
    }

    fn state(&self) -> &[u8] {
        unsafe {
            let mut n = 0;
            let p = SDL_GetKeyboardState(&mut n);
            std::slice::from_raw_parts(p as *const u8, n.max(0) as usize)
        }
    }

    fn mouse_buttons(&self) -> u32 {
        unsafe { SDL_GetMouseState(std::ptr::null_mut(), std::ptr::null_mut()) }.0
    }

    fn has_focus(&self) -> bool {
        unsafe { !SDL_GetKeyboardFocus().is_null() }
    }
}
