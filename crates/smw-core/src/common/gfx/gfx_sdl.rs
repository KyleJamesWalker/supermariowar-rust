//! Port of src/common/gfx/gfxSDL.cpp

use crate::common::path::convert_path;
use crate::globals::*;
use sdl2::sys::image::{IMG_Init, IMG_InitFlags_IMG_INIT_PNG, IMG_Linked_Version, IMG_Quit, IMG_SavePNG};
use sdl2::sys::*;
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

pub const GFX_BPP: i32 = 16;
pub const GFX_SCREEN_W: i32 = 640;
pub const GFX_SCREEN_H: i32 = 480;

const SDL_WINDOWPOS_CENTERED: i32 = SDL_WINDOWPOS_CENTERED_MASK as i32;

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

unsafe fn create_window(fullscreen: bool) -> *mut SDL_Window {
    let mut window_flags = SDL_WindowFlags::SDL_WINDOW_RESIZABLE as u32;
    if fullscreen || cfg!(target_os = "android") {
        window_flags |= SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
    }

    let window = SDL_CreateWindow(
        b"smw\0".as_ptr() as *const _,
        SDL_WINDOWPOS_CENTERED,
        SDL_WINDOWPOS_CENTERED,
        GFX_SCREEN_W,
        GFX_SCREEN_H,
        window_flags,
    );
    if window.is_null() {
        throw_error(format!("Couldn't create window: {}\n", sdl_error()));
    }

    window
}

fn find_preferred_renderer_index() -> i32 {
    -1
}

unsafe fn create_renderer(window: *mut SDL_Window) -> *mut SDL_Renderer {
    let rendering_flags =
        SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32 | SDL_RendererFlags::SDL_RENDERER_TARGETTEXTURE as u32;

    let renderer = SDL_CreateRenderer(window, find_preferred_renderer_index(), rendering_flags);
    if renderer.is_null() {
        throw_error(format!("Couldn't create renderer: {}\n", sdl_error()));
    }

    SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
    SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY.as_ptr() as *const _, b"nearest\0".as_ptr() as *const _);
    SDL_RenderSetLogicalSize(renderer, GFX_SCREEN_W, GFX_SCREEN_H);

    let mut renderer_info: SDL_RendererInfo = std::mem::zeroed();
    SDL_GetRendererInfo(renderer, &mut renderer_info);
    println!(
        "[gfx] Renderer: {}, {}",
        CStr::from_ptr(renderer_info.name).to_string_lossy(),
        if renderer_info.flags & SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32 != 0 { "accelerated" } else { "software" }
    );

    renderer
}

unsafe fn create_screen_surface() -> *mut SDL_Surface {
    let surface =
        SDL_CreateRGBSurface(0x0, GFX_SCREEN_W, GFX_SCREEN_H, 32, 0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000);
    if surface.is_null() {
        throw_error(format!("Couldn't create video buffer: {}\n", sdl_error()));
    }

    surface
}

unsafe fn create_screen_texture(renderer: *mut SDL_Renderer) -> *mut SDL_Texture {
    let texture = SDL_CreateTexture(
        renderer,
        SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32,
        SDL_TextureAccess::SDL_TEXTUREACCESS_STREAMING as i32,
        GFX_SCREEN_W,
        GFX_SCREEN_H,
    );
    if texture.is_null() {
        throw_error(format!("Couldn't create video texture: {}\n", sdl_error()));
    }

    texture
}

pub struct GraphicsSDL {
    // surface -> texture -> renderer -> window
    sdl_window: *mut SDL_Window,
    sdl_renderer: *mut SDL_Renderer,
    sdl_screen_surface: *mut SDL_Surface,
    sdl_screen_texture: *mut SDL_Texture,
    pub _alias: Aliased,
}

impl Default for GraphicsSDL {
    fn default() -> Self {
        GraphicsSDL { _alias: Aliased::new(),
            sdl_window: null_mut(),
            sdl_renderer: null_mut(),
            sdl_screen_surface: null_mut(),
            sdl_screen_texture: null_mut(),
        }
    }
}

impl Drop for GraphicsSDL {
    fn drop(&mut self) {
        unsafe {
            SDL_DestroyTexture(self.sdl_screen_texture);
            SDL_FreeSurface(self.sdl_screen_surface);
            SDL_DestroyRenderer(self.sdl_renderer);
            SDL_DestroyWindow(self.sdl_window);

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

            self.sdl_window = create_window(fullscreen);
            self.sdl_renderer = create_renderer(self.sdl_window);
            self.sdl_screen_surface = create_screen_surface();
            self.sdl_screen_texture = create_screen_texture(self.sdl_renderer);

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
        let msg = CString::new(message).unwrap_or_default();
        unsafe {
            SDL_ShowSimpleMessageBox(
                SDL_MessageBoxFlags::SDL_MESSAGEBOX_ERROR as u32,
                b"Error\0".as_ptr() as *const _,
                msg.as_ptr(),
                self.sdl_window,
            );
        }
    }

    pub fn set_title(&self, title: &str) {
        let t = CString::new(title).unwrap_or_default();
        unsafe { SDL_SetWindowTitle(self.sdl_window, t.as_ptr()) };
    }

    pub fn flip_screen(&self) {
        unsafe {
            SDL_UpdateTexture(
                self.sdl_screen_texture,
                null(),
                (*self.sdl_screen_surface).pixels,
                (*self.sdl_screen_surface).pitch,
            );
            SDL_RenderClear(self.sdl_renderer);
            SDL_RenderCopy(self.sdl_renderer, self.sdl_screen_texture, null(), null());
            #[cfg(not(target_os = "emscripten"))]
            crate::smw::touch::draw(self.sdl_renderer);
            SDL_RenderPresent(self.sdl_renderer);
        }
    }

    pub fn change_full_screen(&self, fullscreen: bool) {
        // Android stays immersive; leaving fullscreen there would bring back the system bars.
        if cfg!(target_os = "android") {
            return;
        }
        unsafe {
            let mut flags = SDL_GetWindowFlags(self.sdl_window);
            if fullscreen {
                flags |= SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
            } else {
                flags &= !(SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32);
            }

            if SDL_SetWindowFullscreen(self.sdl_window, flags) < 0 {
                eprintln!("[gfx] Couldn't toggle fullscreen mode: {}", sdl_error());
                return;
            }
        }
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
