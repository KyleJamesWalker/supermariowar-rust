//! The game window: a 640x480 streaming texture scaled into a resizable window (from the C++ gfxSDL.cpp).

use sdl2::sys::*;
use smw_platform::{Frame, Overlay, Video, SCREEN_H, SCREEN_W};
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

const SDL_WINDOWPOS_CENTERED: i32 = SDL_WINDOWPOS_CENTERED_MASK as i32;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn throw_error(msg: String) -> ! {
    panic!("{}", msg)
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
        SCREEN_W as i32,
        SCREEN_H as i32,
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
    SDL_RenderSetLogicalSize(renderer, SCREEN_W as i32, SCREEN_H as i32);

    let mut renderer_info: SDL_RendererInfo = std::mem::zeroed();
    SDL_GetRendererInfo(renderer, &mut renderer_info);
    println!(
        "[gfx] Renderer: {}, {}",
        CStr::from_ptr(renderer_info.name).to_string_lossy(),
        if renderer_info.flags & SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32 != 0 { "accelerated" } else { "software" }
    );

    renderer
}

unsafe fn create_screen_texture(renderer: *mut SDL_Renderer) -> *mut SDL_Texture {
    let texture = SDL_CreateTexture(
        renderer,
        SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32,
        SDL_TextureAccess::SDL_TEXTUREACCESS_STREAMING as i32,
        SCREEN_W as i32,
        SCREEN_H as i32,
    );
    if texture.is_null() {
        throw_error(format!("Couldn't create video texture: {}\n", sdl_error()));
    }

    texture
}

pub struct Sdl2Video {
    window: *mut SDL_Window,
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    overlay_generation: Option<u64>,
    overlay_textures: Vec<*mut SDL_Texture>,
}

impl Sdl2Video {
    pub fn new() -> Self {
        Sdl2Video { window: null_mut(), renderer: null_mut(), texture: null_mut(), overlay_generation: Option::None, overlay_textures: Vec::new() }
    }
}

impl Default for Sdl2Video {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sdl2Video {
    fn drop(&mut self) {
        unsafe {
            for tex in self.overlay_textures.drain(..) {
                SDL_DestroyTexture(tex);
            }
            SDL_DestroyTexture(self.texture);
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.window);
        }
    }
}

impl Sdl2Video {
    /// Draws in window pixels, outside the 640x480 logical viewport.
    unsafe fn draw_overlay(&mut self, overlay: &Overlay) {
        if self.overlay_generation != Some(overlay.generation) {
            for tex in self.overlay_textures.drain(..) {
                SDL_DestroyTexture(tex);
            }
            for image in overlay.images.iter() {
                let tex = SDL_CreateTexture(
                    self.renderer,
                    SDL_PixelFormatEnum::SDL_PIXELFORMAT_ABGR8888 as u32,
                    SDL_TextureAccess::SDL_TEXTUREACCESS_STATIC as i32,
                    image.w as i32,
                    image.h as i32,
                );
                if !tex.is_null() {
                    // R, G, B, A bytes: ABGR8888 on a little-endian target.
                    SDL_UpdateTexture(tex, null(), image.rgba.as_ptr() as *const _, (image.w * 4) as i32);
                    SDL_SetTextureBlendMode(tex, SDL_BlendMode::SDL_BLENDMODE_BLEND);
                }
                self.overlay_textures.push(tex);
            }
            self.overlay_generation = Some(overlay.generation);
        }
        if self.overlay_textures.iter().any(|t| t.is_null()) {
            return;
        }
        SDL_RenderSetLogicalSize(self.renderer, 0, 0);
        for q in &overlay.quads {
            let tex = self.overlay_textures[q.image as usize];
            SDL_SetTextureColorMod(tex, q.color[0], q.color[1], q.color[2]);
            SDL_SetTextureAlphaMod(tex, q.alpha);
            let dst = SDL_Rect { x: q.x, y: q.y, w: q.w, h: q.h };
            SDL_RenderCopy(self.renderer, tex, null(), &dst);
        }
        SDL_RenderSetLogicalSize(self.renderer, SCREEN_W as i32, SCREEN_H as i32);
    }
}

impl Video for Sdl2Video {
    fn open(&mut self, fullscreen: bool) {
        unsafe {
            self.window = create_window(fullscreen);
            self.renderer = create_renderer(self.window);
            self.texture = create_screen_texture(self.renderer);
        }
    }

    fn present(&mut self, frame: Frame<'_>, overlay: Option<&Overlay>) {
        unsafe {
            SDL_UpdateTexture(self.texture, null(), frame.pixels.as_ptr() as *const _, (SCREEN_W * 4) as i32);
            SDL_RenderClear(self.renderer);
            SDL_RenderCopy(self.renderer, self.texture, null(), null());
            if let Some(overlay) = overlay {
                self.draw_overlay(overlay);
            }
            SDL_RenderPresent(self.renderer);
        }
    }

    fn output_size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { SDL_GetRendererOutputSize(self.renderer, &mut w, &mut h) };
        (w, h)
    }

    fn display_dpi(&self) -> Option<f32> {
        let mut dpi = 0.0;
        (unsafe { SDL_GetDisplayDPI(0, &mut dpi, null_mut(), null_mut()) } == 0 && dpi > 0.0).then_some(dpi)
    }

    fn set_fullscreen(&mut self, on: bool) {
        // Android stays immersive; leaving fullscreen there would bring back the system bars.
        if cfg!(target_os = "android") {
            return;
        }
        unsafe {
            let mut flags = SDL_GetWindowFlags(self.window);
            if on {
                flags |= SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
            } else {
                flags &= !(SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32);
            }

            if SDL_SetWindowFullscreen(self.window, flags) < 0 {
                eprintln!("[gfx] Couldn't toggle fullscreen mode: {}", sdl_error());
            }
        }
    }

    fn set_title(&mut self, title: &str) {
        let t = CString::new(title).unwrap_or_default();
        unsafe { SDL_SetWindowTitle(self.window, t.as_ptr()) };
    }

    fn show_error(&mut self, message: &str) {
        let msg = CString::new(message).unwrap_or_default();
        unsafe {
            SDL_ShowSimpleMessageBox(
                SDL_MessageBoxFlags::SDL_MESSAGEBOX_ERROR as u32,
                b"Error\0".as_ptr() as *const _,
                msg.as_ptr(),
                self.window,
            );
        }
    }
}
