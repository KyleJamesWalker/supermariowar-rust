//! Port of src/common/gfx/gfxSDL.cpp

use crate::common::path::convert_path;
use crate::globals::*;
use sdl2::sys::image::{IMG_Init, IMG_InitFlags_IMG_INIT_PNG, IMG_Linked_Version, IMG_Quit, IMG_SavePNG};
use sdl2::sys::*;
use std::ffi::{CStr, CString};
use crate::services::services;
use smw_platform::Frame;
use std::ptr::null_mut;

pub const GFX_BPP: i32 = 16;
pub const GFX_SCREEN_W: i32 = 640;
pub const GFX_SCREEN_H: i32 = 480;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn throw_error(msg: String) -> ! {
    panic!("{}", msg)
}

unsafe fn init_sdl() {
    if SDL_Init(SDL_INIT_VIDEO) < 0 {
        throw_error(format!("SDL error: {}", sdl_error()));
    }

    let mut sdl_version = SDL_version { major: 0, minor: 0, patch: 0 };
    SDL_GetVersion(&mut sdl_version);
    println!("[gfx] SDL {}.{}.{} loaded.", sdl_version.major, sdl_version.minor, sdl_version.patch);

    let IMG_FLAGS = IMG_InitFlags_IMG_INIT_PNG as i32;
    if (IMG_Init(IMG_FLAGS) & IMG_FLAGS) != IMG_FLAGS {
        throw_error(format!("SDL_image error: {}\n", CStr::from_ptr(SDL_GetError()).to_string_lossy()));
    }

    let img_version = &*IMG_Linked_Version();
    println!("[gfx] SDL_image {}.{}.{} loaded.", img_version.major, img_version.minor, img_version.patch);
}

unsafe fn quit_sdl() {
    IMG_Quit();
    SDL_Quit();
}

unsafe fn create_screen_surface() -> *mut SDL_Surface {
    let surface =
        SDL_CreateRGBSurface(0x0, GFX_SCREEN_W, GFX_SCREEN_H, 32, 0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000);
    if surface.is_null() {
        throw_error(format!("Couldn't create video buffer: {}\n", sdl_error()));
    }

    surface
}

/// The screen surface the game draws on. The window, renderer and texture that show it are the video service's
/// (smw-sdl2).
pub struct GraphicsSDL {
    sdl_screen_surface: *mut SDL_Surface,
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
            SDL_FreeSurface(self.sdl_screen_surface);

            quit_sdl();
        }
    }
}

impl GraphicsSDL {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, fullscreen: bool) -> bool {
        unsafe {
            init_sdl();

            services().video.open(fullscreen);
            #[cfg(not(target_os = "emscripten"))]
            smw_sdl2::set_overlay(crate::smw::touch::draw);
            self.sdl_screen_surface = create_screen_surface();

            println!(
                "[gfx] Game window initialized ({}x{}, {}bpp)",
                (*self.sdl_screen_surface).w,
                (*self.sdl_screen_surface).h,
                (*(*self.sdl_screen_surface).format).BitsPerPixel
            );

            screen = self.sdl_screen_surface;
        }
        true
    }

    pub fn show_error_box(&self, message: &str) {
        services().video.show_error(message);
    }

    pub fn set_title(&self, title: &str) {
        services().video.set_title(title);
    }

    pub fn flip_screen(&self) {
        let pixels = unsafe {
            let s = &*self.sdl_screen_surface;
            debug_assert_eq!(s.pitch, GFX_SCREEN_W * 4);
            std::slice::from_raw_parts(s.pixels as *const u32, (GFX_SCREEN_W * GFX_SCREEN_H) as usize)
        };
        services().video.present(Frame { pixels });
    }

    pub fn change_full_screen(&self, fullscreen: bool) {
        services().video.set_fullscreen(fullscreen);
    }

    pub fn take_screenshot(&self) {
        // NOTE: %F and %T don't work on Windows
        let path_format = "screenshots/%Y-%m-%d_%H%M%S.png";
        let path = convert_path(&strftime_local(path_format));

        let cpath = CString::new(path.as_bytes()).unwrap();
        unsafe {
            if IMG_SavePNG(self.sdl_screen_surface, cpath.as_ptr()) != 0 {
                eprintln!("[gfx] Couldn't write the screenshot to file: {}", sdl_error());
                return;
            }
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
