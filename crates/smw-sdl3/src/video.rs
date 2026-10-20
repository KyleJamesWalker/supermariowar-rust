//! The game window: a 640x480 streaming texture letterboxed into a resizable window, as smw-sdl2's.

use sdl3_sys::everything::*;
use smw_platform::{Frame, Overlay, Video, SCREEN_H, SCREEN_W};
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// The SDL window, its renderer and the screen texture; all null until `open`.
pub struct Sdl3Video {
    window: *mut SDL_Window,
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    /// The `Overlay::generation` that `overlay_textures` were made from.
    overlay_generation: Option<u64>,
    /// One per overlay image, null where SDL could not create it.
    overlay_textures: Vec<*mut SDL_Texture>,
    vsync: bool,
}

impl Sdl3Video {
    pub fn new() -> Self {
        Sdl3Video { window: null_mut(), renderer: null_mut(), texture: null_mut(), overlay_generation: None, overlay_textures: Vec::new(), vsync: false }
    }

    /// Turns the letterboxed 640x480 logical presentation on, or off to draw in window pixels.
    fn set_logical(&self, on: bool) {
        let (w, h, mode) = if on { (SCREEN_W as i32, SCREEN_H as i32, SDL_LOGICAL_PRESENTATION_LETTERBOX) } else { (0, 0, SDL_LOGICAL_PRESENTATION_DISABLED) };
        // SAFETY: SDL rejects a null renderer.
        unsafe { SDL_SetRenderLogicalPresentation(self.renderer, w, h, mode) };
    }

    /// Draws in window pixels, outside the 640x480 logical viewport. Safety: the renderer must be open.
    unsafe fn draw_overlay(&mut self, overlay: &Overlay) {
        if self.overlay_generation != Some(overlay.generation) {
            for tex in self.overlay_textures.drain(..) {
                SDL_DestroyTexture(tex);
            }
            for image in overlay.images.iter() {
                let tex = SDL_CreateTexture(self.renderer, SDL_PIXELFORMAT_ABGR8888, SDL_TEXTUREACCESS_STATIC, image.w as i32, image.h as i32);
                if !tex.is_null() {
                    // R, G, B, A bytes: ABGR8888 on a little-endian target.
                    SDL_UpdateTexture(tex, null(), image.rgba.as_ptr() as *const _, (image.w * 4) as i32);
                    SDL_SetTextureBlendMode(tex, SDL_BLENDMODE_BLEND);
                }
                self.overlay_textures.push(tex);
            }
            self.overlay_generation = Some(overlay.generation);
        }
        if self.overlay_textures.iter().any(|t| t.is_null()) {
            return;
        }
        self.set_logical(false);
        for q in &overlay.quads {
            let tex = self.overlay_textures[q.image as usize];
            SDL_SetTextureColorMod(tex, q.color[0], q.color[1], q.color[2]);
            SDL_SetTextureAlphaMod(tex, q.alpha);
            let dst = SDL_FRect { x: q.x as f32, y: q.y as f32, w: q.w as f32, h: q.h as f32 };
            SDL_RenderTexture(self.renderer, tex, null(), &dst);
        }
        self.set_logical(true);
    }
}

impl Default for Sdl3Video {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sdl3Video {
    fn drop(&mut self) {
        // SAFETY: each non-null pointer is destroyed once, textures before their renderer.
        unsafe {
            for tex in self.overlay_textures.drain(..) {
                SDL_DestroyTexture(tex);
            }
            if !self.texture.is_null() {
                SDL_DestroyTexture(self.texture);
            }
            if !self.renderer.is_null() {
                SDL_DestroyRenderer(self.renderer);
            }
            if !self.window.is_null() {
                SDL_DestroyWindow(self.window);
            }
        }
    }
}

impl Video for Sdl3Video {
    fn vsync(&self) -> bool {
        self.vsync
    }

    fn show_cursor(&mut self, on: bool) {
        // SAFETY: takes no pointers.
        unsafe {
            if on {
                SDL_ShowCursor();
            } else {
                SDL_HideCursor();
            }
        }
    }

    fn open(&mut self, fullscreen: bool) {
        // SAFETY: each create call panics on null, so the stored pointers are valid.
        unsafe {
            if !SDL_Init(SDL_INIT_VIDEO) {
                panic!("SDL error: {}", sdl_error());
            }
            let v = SDL_GetVersion();
            println!("[gfx] SDL {}.{}.{} loaded.", v / 1_000_000, v / 1000 % 1000, v % 1000);

            let mut flags = SDL_WINDOW_RESIZABLE;
            if fullscreen || cfg!(target_os = "android") {
                flags |= SDL_WINDOW_FULLSCREEN;
            }
            self.window = SDL_CreateWindow(c"smw".as_ptr(), SCREEN_W as i32, SCREEN_H as i32, flags);
            if self.window.is_null() {
                panic!("Couldn't create window: {}\n", sdl_error());
            }
            self.renderer = SDL_CreateRenderer(self.window, null());
            if self.renderer.is_null() {
                panic!("Couldn't create renderer: {}\n", sdl_error());
            }
            SDL_SetRenderDrawColor(self.renderer, 0, 0, 0, 255);
            self.vsync = smw_platform::want_vsync() && SDL_SetRenderVSync(self.renderer, 1);
            self.set_logical(true);
            let name = SDL_GetRendererName(self.renderer);
            println!("[gfx] Renderer: {}", if name.is_null() { String::new() } else { CStr::from_ptr(name).to_string_lossy().into_owned() });

            self.texture = SDL_CreateTexture(self.renderer, SDL_PIXELFORMAT_ARGB8888, SDL_TEXTUREACCESS_STREAMING, SCREEN_W as i32, SCREEN_H as i32);
            if self.texture.is_null() {
                panic!("Couldn't create video texture: {}\n", sdl_error());
            }
            SDL_SetTextureScaleMode(self.texture, SDL_SCALEMODE_NEAREST);
            // Text fields expect text input on desktops without asking for it.
            if !cfg!(any(target_os = "android", target_os = "ios")) {
                SDL_StartTextInput(self.window);
            }
        }
    }

    fn present(&mut self, frame: Frame<'_>, overlay: Option<&Overlay>) {
        // SAFETY: `Frame` holds SCREEN_W * SCREEN_H pixels, the texture's size; SDL rejects the nulls before `open`.
        unsafe {
            SDL_UpdateTexture(self.texture, null(), frame.pixels.as_ptr() as *const _, (SCREEN_W * 4) as i32);
            SDL_RenderClear(self.renderer);
            SDL_RenderTexture(self.renderer, self.texture, null(), null());
            if let Some(overlay) = overlay {
                self.draw_overlay(overlay);
            }
            SDL_RenderPresent(self.renderer);
        }
    }

    fn output_size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        // SAFETY: the out-pointers are live locals; SDL rejects a null renderer.
        unsafe { SDL_GetCurrentRenderOutputSize(self.renderer, &mut w, &mut h) };
        (w, h)
    }

    /// Android only: its content scale is relative to 160 DPI.
    fn display_dpi(&self) -> Option<f32> {
        if !cfg!(target_os = "android") {
            return None;
        }
        // SAFETY: SDL rejects a null window and an invalid display with 0.
        let scale = unsafe { SDL_GetDisplayContentScale(SDL_GetDisplayForWindow(self.window)) };
        (scale > 0.0).then_some(scale * 160.0)
    }

    fn set_fullscreen(&mut self, on: bool) {
        // Android stays immersive; leaving fullscreen there would bring back the system bars.
        if cfg!(target_os = "android") {
            return;
        }
        // SAFETY: SDL rejects a null window.
        if !unsafe { SDL_SetWindowFullscreen(self.window, on) } {
            eprintln!("[gfx] Couldn't toggle fullscreen mode: {}", sdl_error());
        }
    }

    fn set_title(&mut self, title: &str) {
        let t = CString::new(title).unwrap_or_default();
        // SAFETY: `t` is NUL-terminated and outlives the call; SDL rejects a null window.
        unsafe { SDL_SetWindowTitle(self.window, t.as_ptr()) };
    }

    fn show_error(&mut self, message: &str) {
        let msg = CString::new(message).unwrap_or_default();
        // SAFETY: both strings are NUL-terminated; a null parent window is allowed.
        unsafe { SDL_ShowSimpleMessageBox(SDL_MESSAGEBOX_ERROR, c"Error".as_ptr(), msg.as_ptr(), self.window) };
    }
}
