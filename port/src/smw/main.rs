//! Port of src/smw/main.cpp

use sdl2::sys::SDL_Surface;
use std::ptr::null_mut;

pub static mut screen: *mut SDL_Surface = null_mut();
pub static mut blitdest: *mut SDL_Surface = null_mut();

pub static mut x_shake: i16 = 0;
pub static mut y_shake: i16 = 0;

pub fn main() {
    crate::globals::init_globals();
}
