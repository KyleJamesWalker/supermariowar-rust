//! Port of src/common/util/SdlHelpers.cpp

use sdl2::sys::{
    SDL_DestroyRenderer, SDL_DestroyTexture, SDL_DestroyWindow, SDL_FreeSurface, SDL_Renderer,
    SDL_Surface, SDL_Texture, SDL_Window,
};
use std::ptr::null_mut;

macro_rules! sdl_unique_ptr {
    ($name:ident, $ty:ty, $deleter:ident) => {
        /// `std::unique_ptr<T, SdlDeleter>`.
        pub struct $name(*mut $ty);

        impl $name {
            pub fn new(ptr: *mut $ty) -> Self {
                $name(ptr)
            }
            pub const fn null() -> Self {
                $name(null_mut())
            }
            pub fn get(&self) -> *mut $ty {
                self.0
            }
            pub fn is_null(&self) -> bool {
                self.0.is_null()
            }
            pub fn release(&mut self) -> *mut $ty {
                std::mem::replace(&mut self.0, null_mut())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name::null()
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { $deleter(self.0) };
                }
            }
        }

        impl std::ops::Deref for $name {
            type Target = $ty;
            fn deref(&self) -> &$ty {
                assert!(!self.0.is_null(), "null SDL pointer dereference");
                unsafe { &*self.0 }
            }
        }
    };
}

sdl_unique_ptr!(SdlWindowPtr, SDL_Window, SDL_DestroyWindow);
sdl_unique_ptr!(SdlRendererPtr, SDL_Renderer, SDL_DestroyRenderer);
sdl_unique_ptr!(SdlTexturePtr, SDL_Texture, SDL_DestroyTexture);
sdl_unique_ptr!(SdlSurfacePtr, SDL_Surface, SDL_FreeSurface);

/// `SDL_PollEvent` that leaves `*event` as is on an empty queue, as sdl2-compat does. SDL2 writes its poll
/// sentinel there, and the editors read `event` after their poll loops.
pub unsafe fn poll_event(event: *mut sdl2::sys::SDL_Event) -> i32 {
    let mut polled: sdl2::sys::SDL_Event = std::mem::zeroed();
    let result = sdl2::sys::SDL_PollEvent(&mut polled);
    if result != 0 {
        *event = polled;
    }
    result
}
