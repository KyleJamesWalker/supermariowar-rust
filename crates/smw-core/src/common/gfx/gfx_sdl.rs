//! The game's screen surface and the window operations it forwards to the video service.

use crate::common::gfx::soft_surface::{alloc_surface, free_surface, Surface};
use crate::game::Game;
use crate::common::path::convert_path;
use crate::globals::*;
use crate::services::services;
use smw_platform::Frame;
use std::ffi::CString;
use std::ptr::null_mut;

pub const GFX_BPP: i32 = 16;
/// Screen size in pixels.
pub const GFX_SCREEN_W: i32 = 640;
pub const GFX_SCREEN_H: i32 = 480;

fn create_screen_surface() -> *mut Surface {
    alloc_surface(Surface::new(GFX_SCREEN_W, GFX_SCREEN_H))
}

/// The screen surface the game draws on. The window, renderer and texture that show it are the video service's
/// (smw-sdl2).
pub struct GraphicsSDL {
    sdl_screen_surface: *mut Surface,
    pub _alias: Aliased,
}

impl Default for GraphicsSDL {
    fn default() -> Self {
        GraphicsSDL { _alias: Aliased::new(), sdl_screen_surface: null_mut() }
    }
}

impl Drop for GraphicsSDL {
    fn drop(&mut self) {
        unsafe {
            free_surface(self.sdl_screen_surface);
        }
    }
}

impl GraphicsSDL {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the window and allocates the screen surface, which also becomes `g.screen`. Always returns `true`.
    pub fn init(&mut self, g: &mut Game, fullscreen: bool) -> bool {
        let mut gp = g.ptr();
        unsafe {
            services(gp.get()).video.open(fullscreen);
            self.sdl_screen_surface = create_screen_surface();

            println!(
                "[gfx] Game window initialized ({}x{}, {}bpp)",
                (*self.sdl_screen_surface).w,
                (*self.sdl_screen_surface).h,
                32
            );

            g.screen = self.sdl_screen_surface;
        }
        true
    }

    pub fn show_error_box(&self, g: &mut Game, message: &str) {
        let mut gp = g.ptr();
        services(gp.get()).video.show_error(message);
    }

    pub fn set_title(&self, g: &mut Game, title: &str) {
        let mut gp = g.ptr();
        services(gp.get()).video.set_title(title);
    }

    /// Presents the screen surface, with the touch controls overlay on platforms that have one.
    pub fn flip_screen(&self, g: &mut Game) {
        let mut gp = g.ptr();
        let screen = unsafe { &*self.sdl_screen_surface };
        let pixels = &screen.pixels[..];
        #[cfg(not(target_os = "emscripten"))]
        let overlay = crate::smw::touch::overlay(g);
        #[cfg(target_os = "emscripten")]
        let overlay: Option<smw_platform::Overlay> = Option::None;
        services(gp.get()).video.present(Frame { pixels }, overlay.as_ref());
    }

    pub fn change_full_screen(&self, g: &mut Game, fullscreen: bool) {
        let mut gp = g.ptr();
        services(gp.get()).video.set_fullscreen(fullscreen);
    }

    /// Saves the screen as a timestamped PNG under `screenshots/`; failures are logged, not returned.
    pub fn take_screenshot(&self, g: &mut Game) {
        // NOTE: %F and %T don't work on Windows
        let path_format = "screenshots/%Y-%m-%d_%H%M%S.png";
        let path = convert_path(g, &strftime_local(path_format));

        if let Err(e) = unsafe { (*self.sdl_screen_surface).save_png(std::path::Path::new(&path)) } {
            eprintln!("[gfx] Couldn't write the screenshot to file: {}", e);
            return;
        }

        println!("[gfx] Screenshot saved to file: {}", path);
    }
}

fn strftime_local(format: &str) -> String {
    extern "C" {
        #[cfg_attr(windows, link_name = "_time64")]
        fn time(t: *mut i64) -> i64;
        #[cfg_attr(windows, link_name = "_localtime64")]
        fn localtime(t: *const i64) -> *mut libc_tm;
        fn strftime(s: *mut u8, max: usize, format: *const u8, tm: *const libc_tm) -> usize;
    }
    #[repr(C)]
    struct libc_tm {
        _opaque: [u8; 0],
    }
    let fmt = CString::new(format).unwrap();
    let mut buf = [0u8; 256];
    unsafe {
        let now = time(null_mut());
        let tm = localtime(&now);
        let n = strftime(buf.as_mut_ptr(), buf.len(), fmt.as_ptr() as *const u8, tm);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}
