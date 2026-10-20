//! The game window: a 640x480 streaming texture scaled into a resizable window.

use sdl2::sys::*;
use smw_platform::{Frame, Overlay, Video, SCREEN_H, SCREEN_W};
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

const SDL_WINDOWPOS_CENTERED: i32 = SDL_WINDOWPOS_CENTERED_MASK as i32;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// How `Video::open` fails: a panic carrying SDL's message.
fn throw_error(msg: String) -> ! {
    panic!("{}", msg)
}

/// Always fullscreen on Android.
unsafe fn create_window(fullscreen: bool, extra_flags: u32) -> *mut SDL_Window {
    let mut window_flags = SDL_WindowFlags::SDL_WINDOW_RESIZABLE as u32 | extra_flags;
    if fullscreen || cfg!(target_os = "android") {
        window_flags |= SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
    }

    // SMW_WINDOW_SCALE opens the window at a multiple of 640x480.
    let scale = std::env::var("SMW_WINDOW_SCALE").ok().and_then(|s| s.parse::<i32>().ok()).filter(|s| *s > 0).unwrap_or(1);
    // SMW_WINDOW_DISPLAY opens it centered on that display (0 is the primary one).
    let display = std::env::var("SMW_WINDOW_DISPLAY").ok().and_then(|d| d.parse::<i32>().ok()).filter(|d| (0..SDL_GetNumVideoDisplays()).contains(d));
    let pos = display.map_or(SDL_WINDOWPOS_CENTERED, |d| SDL_WINDOWPOS_CENTERED | d);
    let window = SDL_CreateWindow(
        b"smw\0".as_ptr() as *const _,
        pos,
        pos,
        SCREEN_W as i32 * scale,
        SCREEN_H as i32 * scale,
        window_flags,
    );
    if window.is_null() {
        throw_error(format!("Couldn't create window: {}\n", sdl_error()));
    }

    window
}

/// Rescales a mouse button event to the logical viewport, which SDL's renderer only does after event filters run.
pub unsafe fn button_to_logical(b: &mut SDL_MouseButtonEvent) {
    let renderer = SDL_GetRenderer(SDL_GetWindowFromID(b.windowID));
    if renderer.is_null() {
        return;
    }
    let (mut x, mut y) = (0.0f32, 0.0f32);
    SDL_RenderWindowToLogical(renderer, b.x, b.y, &mut x, &mut y);
    b.x = x.floor() as i32;
    b.y = y.floor() as i32;
}

/// -1: SDL picks the first driver that supports the flags.
fn find_preferred_renderer_index() -> i32 {
    -1
}

/// The renderer, and whether it presents on vsync (`smw_platform::want_vsync`).
unsafe fn create_renderer(window: *mut SDL_Window) -> (*mut SDL_Renderer, bool) {
    let mut rendering_flags =
        SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32 | SDL_RendererFlags::SDL_RENDERER_TARGETTEXTURE as u32;
    if smw_platform::want_vsync() {
        rendering_flags |= SDL_RendererFlags::SDL_RENDERER_PRESENTVSYNC as u32;
    }

    // Must be added before the renderer's own watch, which rescales and clamps finger events.
    static RAW_FINGER_WATCH: std::sync::Once = std::sync::Once::new();
    RAW_FINGER_WATCH.call_once(|| SDL_AddEventWatch(Some(crate::input::keep_raw_finger), std::ptr::null_mut()));
    let renderer = SDL_CreateRenderer(window, find_preferred_renderer_index(), rendering_flags);
    if renderer.is_null() {
        throw_error(format!("Couldn't create renderer: {}\n", sdl_error()));
    }

    SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
    SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY.as_ptr() as *const _, b"nearest\0".as_ptr() as *const _);
    SDL_RenderSetLogicalSize(renderer, SCREEN_W as i32, SCREEN_H as i32);

    let mut renderer_info: SDL_RendererInfo = std::mem::zeroed();
    SDL_GetRendererInfo(renderer, &mut renderer_info);
    let vsync = renderer_info.flags & SDL_RendererFlags::SDL_RENDERER_PRESENTVSYNC as u32 != 0;
    println!(
        "[gfx] Renderer: {}, {}{}",
        CStr::from_ptr(renderer_info.name).to_string_lossy(),
        if renderer_info.flags & SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32 != 0 { "accelerated" } else { "software" },
        if vsync { ", vsync" } else { "" }
    );

    (renderer, vsync)
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

/// The SDL window, its renderer and the screen texture; all null until `open`.
pub struct Sdl2Video {
    window: *mut SDL_Window,
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    /// The `Overlay::generation` that `overlay_textures` were made from.
    overlay_generation: Option<u64>,
    /// One per overlay image, null where SDL could not create it.
    overlay_textures: Vec<*mut SDL_Texture>,
    vsync: bool,
    integer_scale: bool,
    #[cfg(feature = "shaders")]
    shader: Option<crate::shader::ShaderPresenter>,
    #[cfg(feature = "gl")]
    lite: Option<crate::crt_lite::CrtLite>,
    /// The preset `set_shader` picked before `open`.
    #[cfg(feature = "gl")]
    pending_shader: Option<std::path::PathBuf>,
}

impl Sdl2Video {
    pub fn new() -> Self {
        Sdl2Video {
            window: null_mut(),
            renderer: null_mut(),
            texture: null_mut(),
            overlay_generation: Option::None,
            overlay_textures: Vec::new(),
            vsync: false,
            integer_scale: false,
            #[cfg(feature = "shaders")]
            shader: Option::None,
            #[cfg(feature = "gl")]
            lite: Option::None,
            #[cfg(feature = "gl")]
            pending_shader: Option::None,
        }
    }
}

impl Default for Sdl2Video {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sdl2Video {
    fn drop(&mut self) {
        // SAFETY: `destroy` frees each object once and leaves nulls behind.
        unsafe { self.destroy() };
    }
}

impl Sdl2Video {
    /// Makes the window and its presenter: the built-in one for a `builtin:<name>` preset, the librashader one for
    /// a `.slangp` file, else the renderer. Err says why the preset did not load; the window then uses the renderer.
    unsafe fn create(&mut self, fullscreen: bool, preset: Option<&std::path::Path>) -> Result<(), String> {
        let mut result = Ok(());
        #[cfg(feature = "gl")]
        if let Some(name) = preset.and_then(|p| p.to_str()).and_then(|p| p.strip_prefix("builtin:")) {
            crate::crt_lite::request_context();
            let flags = SDL_WindowFlags::SDL_WINDOW_OPENGL as u32 | SDL_WindowFlags::SDL_WINDOW_ALLOW_HIGHDPI as u32;
            self.window = create_window(fullscreen, flags);
            match crate::crt_lite::CrtLite::new(self.window, name) {
                Ok(lite) => {
                    self.vsync = lite.vsync();
                    self.lite = Some(lite);
                    self.apply_integer_scale();
                    return Ok(());
                }
                Err(e) => {
                    SDL_DestroyWindow(self.window);
                    SDL_GL_ResetAttributes();
                    return self.create(fullscreen, Option::None).and(Err(e));
                }
            }
        }
        #[cfg(feature = "shaders")]
        if let Some(preset) = preset {
            crate::shader::request_context();
            self.window = create_window(fullscreen, crate::shader::window_flags());
            match crate::shader::ShaderPresenter::new(self.window, preset) {
                Ok(shader) => {
                    self.vsync = shader.vsync();
                    self.shader = Some(shader);
                    self.apply_integer_scale();
                    return Ok(());
                }
                Err(e) => {
                    SDL_DestroyWindow(self.window);
                    SDL_GL_ResetAttributes();
                    result = Err(e);
                }
            }
        }
        #[cfg(not(feature = "shaders"))]
        if preset.is_some() {
            result = Err("this build runs only the built-in shaders".to_string());
        }
        self.window = create_window(fullscreen, 0);
        (self.renderer, self.vsync) = create_renderer(self.window);
        self.texture = create_screen_texture(self.renderer);
        self.apply_integer_scale();
        result
    }

    /// Frees the window and everything drawn into it. Safety: no other pointer to them is in use.
    unsafe fn destroy(&mut self) {
        #[cfg(feature = "shaders")]
        {
            self.shader = Option::None;
        }
        #[cfg(feature = "gl")]
        {
            self.lite = Option::None;
        }
        for tex in self.overlay_textures.drain(..) {
            SDL_DestroyTexture(tex);
        }
        self.overlay_generation = Option::None;
        SDL_DestroyTexture(self.texture);
        SDL_DestroyRenderer(self.renderer);
        SDL_DestroyWindow(self.window);
        (self.texture, self.renderer, self.window) = (null_mut(), null_mut(), null_mut());
    }

    unsafe fn apply_integer_scale(&mut self) {
        #[cfg(feature = "gl")]
        crate::gl_common::set_integer_scale(self.integer_scale);
        if !self.renderer.is_null() {
            SDL_RenderSetIntegerScale(self.renderer, if self.integer_scale { SDL_bool::SDL_TRUE } else { SDL_bool::SDL_FALSE });
        }
    }

    /// Draws in window pixels, outside the 640x480 logical viewport. Safety: the renderer must be open.
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
    fn vsync(&self) -> bool {
        self.vsync
    }

    fn show_cursor(&mut self, on: bool) {
        // SAFETY: takes no pointers.
        unsafe { SDL_ShowCursor(if on { SDL_ENABLE } else { SDL_DISABLE } as i32) };
    }

    fn open(&mut self, fullscreen: bool) {
        // SAFETY: each create function panics on null, so the stored pointers are valid.
        unsafe {
            if SDL_Init(SDL_INIT_VIDEO) < 0 {
                panic!("SDL error: {}", std::ffi::CStr::from_ptr(SDL_GetError()).to_string_lossy());
            }
            let mut v = SDL_version { major: 0, minor: 0, patch: 0 };
            SDL_GetVersion(&mut v);
            println!("[gfx] SDL {}.{}.{} loaded.", v.major, v.minor, v.patch);
            #[cfg(feature = "gl")]
            let preset = std::env::var_os("SMW_SHADER").map(std::path::PathBuf::from).or(self.pending_shader.take());
            #[cfg(not(feature = "gl"))]
            let preset: Option<std::path::PathBuf> = Option::None;
            if let Err(e) = self.create(fullscreen, preset.as_deref()) {
                eprintln!("[gfx] Shader preset unavailable, using the renderer: {e}");
            }
        }
    }

    fn supports_shaders(&self) -> bool {
        cfg!(feature = "gl")
    }

    fn builtin_shaders(&self) -> Vec<String> {
        #[cfg(all(feature = "gl", not(feature = "shaders")))]
        return crate::crt_lite::BUILTIN.iter().map(|s| s.to_string()).collect();
        #[cfg(any(not(feature = "gl"), feature = "shaders"))]
        Vec::new()
    }

    fn set_shader(&mut self, preset: Option<&std::path::Path>) -> Result<(), String> {
        if self.window.is_null() {
            #[cfg(feature = "gl")]
            {
                self.pending_shader = preset.map(std::path::Path::to_path_buf);
                return Ok(());
            }
            #[cfg(not(feature = "gl"))]
            return if preset.is_some() { Err("built without shader support".to_string()) } else { Ok(()) };
        }
        // SAFETY: the window is open, so `destroy` frees live objects and `create` replaces them.
        unsafe {
            let fullscreen = SDL_GetWindowFlags(self.window) & SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32 != 0;
            self.destroy();
            self.create(fullscreen, preset)
        }
    }

    fn set_integer_scale(&mut self, on: bool) {
        self.integer_scale = on;
        // SAFETY: takes the renderer only when it exists.
        unsafe { self.apply_integer_scale() };
    }

    fn present(&mut self, frame: Frame<'_>, overlay: Option<&Overlay>) {
        // SAFETY: `Frame` holds SCREEN_W * SCREEN_H pixels, the texture's size; SDL rejects the nulls before `open`.
        unsafe {
            #[cfg(feature = "shaders")]
            if let Some(shader) = self.shader.as_mut() {
                shader.present(self.window, frame, overlay);
                return;
            }
            #[cfg(feature = "gl")]
            if let Some(lite) = self.lite.as_mut() {
                lite.present(self.window, frame, overlay);
                return;
            }
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
        #[cfg(feature = "gl")]
        if self.renderer.is_null() && !self.window.is_null() {
            // SAFETY: without a renderer, a GL presenter owns the open window.
            return unsafe { crate::gl_common::drawable_size(self.window) };
        }
        let (mut w, mut h) = (0, 0);
        // SAFETY: the out-pointers are live locals; SDL rejects a null renderer.
        unsafe { SDL_GetRendererOutputSize(self.renderer, &mut w, &mut h) };
        (w, h)
    }

    fn display_dpi(&self) -> Option<f32> {
        let mut dpi = 0.0;
        // SAFETY: SDL accepts null for the DPIs it should not write.
        (unsafe { SDL_GetDisplayDPI(0, &mut dpi, null_mut(), null_mut()) } == 0 && dpi > 0.0).then_some(dpi)
    }

    fn set_fullscreen(&mut self, on: bool) {
        // Android stays immersive; leaving fullscreen there would bring back the system bars.
        if cfg!(target_os = "android") {
            return;
        }
        // SAFETY: SDL rejects a null window.
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
        // SAFETY: `t` is NUL-terminated and outlives the call; SDL rejects a null window.
        unsafe { SDL_SetWindowTitle(self.window, t.as_ptr()) };
    }

    fn show_error(&mut self, message: &str) {
        let msg = CString::new(message).unwrap_or_default();
        // SAFETY: both strings are NUL-terminated; a null parent window is allowed.
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
