//! Port of src/common/gfx/gfxSprite.cpp

use crate::common::gfx::color::{colors, RGB};
use crate::common::gfx::{gfx_adjusthiddenrects, rle_enabled};
use crate::common::util::sdl_helpers::SdlSurfacePtr;
use crate::globals::*;
use sdl2::sys::image::IMG_Load;
use sdl2::sys::{
    SDL_BlendMode, SDL_ConvertSurface, SDL_CreateRGBSurfaceWithFormat, SDL_GetError, SDL_MapRGB, SDL_Rect, SDL_SetColorKey,
    SDL_SetSurfaceAlphaMod, SDL_SetSurfaceBlendMode, SDL_SetSurfaceRLE, SDL_Surface, SDL_UpperBlit,
    SDL_UpperBlitScaled,
};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::ptr::null;

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ClipEdge {
    Top,
    Right,
    Bottom,
    Left,
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// Mirrors the C++ `throw std::format(...)`: loading failures are fatal unless a caller catches them.
fn throw(msg: String) -> ! {
    panic!("{}", msg)
}

fn try_load_image(path: &Path, optimize: bool, color_key: Option<RGB>, alpha: Option<u8>) -> Result<SdlSurfacePtr, String> {
    let path_str = path.to_string_lossy().into_owned();

    let mut out = String::from("loading sprite");
    if color_key.is_some() {
        out += " with key";
        if alpha.is_some() {
            out += "+alpha";
        }
        out += " ";
    }
    out += &path_str;
    out += " ...";
    print!("{}", out);
    let _ = std::io::stdout().flush();

    unsafe {
        let cpath = CString::new(path_str.as_bytes()).unwrap();
        let raw = SdlSurfacePtr::new(IMG_Load(cpath.as_ptr()));
        if raw.is_null() {
            return Err(format!("Couldn't load {}: {}", path_str, sdl_error()));
        }

        if let Some(color_key) = color_key {
            let key = SDL_MapRGB(raw.format, color_key.r, color_key.g, color_key.b);
            if SDL_SetColorKey(raw.get(), 1, key) < 0 {
                return Err(format!("Couldn't set color key for {}: {}", path_str, sdl_error()));
            }
        }

        let img = SdlSurfacePtr::new(SDL_ConvertSurface(raw.get(), (*screen).format, 0));
        if img.is_null() {
            return Err(format!("Couldn't convert {} to the display's pixel format: {}", path_str, sdl_error()));
        }

        if optimize && rle_enabled() && SDL_SetSurfaceRLE(img.get(), 1) < 0 {
            return Err(format!("Couldn't set RLE acceleration for {}: {}", path_str, sdl_error()));
        }

        if let Some(alpha) = alpha {
            if SDL_SetSurfaceBlendMode(img.get(), SDL_BlendMode::SDL_BLENDMODE_BLEND) < 0 {
                return Err(format!("Couldn't set blend mode for {}: {}", path_str, sdl_error()));
            }
            if SDL_SetSurfaceAlphaMod(img.get(), alpha) < 0 {
                return Err(format!("Couldn't set alpha modulation for {}: {}", path_str, sdl_error()));
            }
        }

        println!(" done");
        Ok(img)
    }
}

fn load_image(path: &Path, optimize: bool, color_key: Option<RGB>, alpha: Option<u8>) -> SdlSurfacePtr {
    try_load_image(path, optimize, color_key, alpha).unwrap_or_else(|what| throw(what))
}


unsafe fn blit_surface(src: *mut SDL_Surface, srcArea: *const SDL_Rect, dst: *mut SDL_Surface, dstArea: *mut SDL_Rect) {
    if crate::common::gfx::blit::upper_blit(src, srcArea, dst, dstArea) < 0 {
        eprintln!("SDL_BlitSurface error: {}", sdl_error());
    }
}

#[derive(Default)]
pub struct gfxSprite {
    m_picture: SdlSurfacePtr,
    m_wrap_x: Option<i32>,
    pub _alias: Aliased,
}

impl gfxSprite {
    pub const fn new() -> Self {
        gfxSprite { _alias: Aliased::new(), m_picture: SdlSurfacePtr::null(), m_wrap_x: None }
    }

    /// `gfxSprite(SdlSurfacePtr image, wrap = 640)`
    pub fn from_surface(image: SdlSurfacePtr, wrap: Option<i32>) -> Self {
        gfxSprite { _alias: Aliased::new(), m_picture: image, m_wrap_x: wrap }
    }

    /// `gfxSprite::blank(w, h)`: a screen-format surface with blending off.
    pub fn blank(w: u32, h: u32) -> Self {
        unsafe {
            let sf = (*screen).format;
            let surf = SdlSurfacePtr::new(SDL_CreateRGBSurfaceWithFormat(0x0, w as i32, h as i32, (*sf).BitsPerPixel as i32, (*sf).format));
            if surf.is_null() {
                throw(format!("Couldn't create blank surface: {}", sdl_error()));
            }

            if SDL_SetSurfaceBlendMode(surf.get(), SDL_BlendMode::SDL_BLENDMODE_NONE) < 0 {
                throw(format!("Couldn't set blend mode for blank surface: {}", sdl_error()));
            }

            Self::from_surface(surf, Some(640))
        }
    }

    /// `draw(SDL_Surface* dst, Vec2i dstPos)`: the whole sprite onto `dst`, no camera shake.
    pub fn draw_to_pos(&self, dst: *mut SDL_Surface, dstX: i32, dstY: i32) {
        self.blit(null(), dst, dstX, dstY);
    }

    /// `draw(SDL_Surface* dst, const SDL_Rect& dstRect)`: the whole sprite onto `dst`, no camera shake.
    pub fn draw_to(&self, dst: *mut SDL_Surface, dstRect: &SDL_Rect) {
        self.blit(null(), dst, dstRect.x, dstRect.y);
    }

    /// `draw(const SDL_Rect& srcRect, SDL_Surface* dst, Vec2i dstPos)`: part of the sprite onto `dst`.
    pub fn draw_src_to_pos(&self, srcRect: &SDL_Rect, dst: *mut SDL_Surface, dstX: i32, dstY: i32) {
        self.blit(srcRect, dst, dstX, dstY);
    }

    /// `draw(const SDL_Rect& srcRect, SDL_Surface* dst, const SDL_Rect& dstRect)`: part of the sprite onto `dst`.
    pub fn draw_src_to(&self, srcRect: &SDL_Rect, dst: *mut SDL_Surface, dstRect: &SDL_Rect) {
        self.blit(srcRect, dst, dstRect.x, dstRect.y);
    }

    fn blit(&self, srcRect: *const SDL_Rect, dst: *mut SDL_Surface, dstPosX: i32, dstPosY: i32) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            let mut dstRect = SDL_Rect { x: dstPosX, y: dstPosY, w: 0, h: 0 };
            blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);

            if let Some(wrap_x) = self.m_wrap_x {
                if dstRect.x + self.get_width() >= wrap_x {
                    dstRect = SDL_Rect { x: dstPosX - wrap_x, y: dstPosY, w: 0, h: 0 }; // SDL2 modifies the dst rect
                    blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);
                } else if dstRect.x < 0 {
                    dstRect = SDL_Rect { x: dstPosX + wrap_x, y: dstPosY, w: 0, h: 0 }; // SDL2 modifies the dst rect
                    blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);
                }
            }
        }
    }

    /// Draw the whole sprite at the given coordinate.
    pub fn draw(&self, x: i32, y: i32) {
        unsafe { self.blit(null(), blitdest, x + x_shake as i32, y + y_shake as i32) };
    }

    /// Draw part of the sprite at the given coordinate.
    pub fn draw_src(&self, x: i32, y: i32, srcRect: &SDL_Rect) {
        unsafe { self.blit(srcRect, blitdest, x + x_shake as i32, y + y_shake as i32) };
    }

    /// `draw(x, y, srcx, srcy, w, h)` convenience used all over the C++ via `SDL_Rect{...}` temporaries.
    pub fn draw_part(&self, x: i32, y: i32, srcx: i32, srcy: i32, w: i32, h: i32) {
        self.draw_src(x, y, &SDL_Rect { x: srcx, y: srcy, w, h });
    }

    /// Draw part of the sprite at the given coordinate, clipped along a given direction.
    pub fn draw_clip(&self, x: i32, y: i32, srcRect: &SDL_Rect, clipEdge: ClipEdge, clipTreshold: i32) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            let dstRect = SDL_Rect { x: x + x_shake as i32, y: y + y_shake as i32, w: srcRect.w, h: srcRect.h };
            let mut adjustedSrcRect = *srcRect;
            let mut adjustedDstRect = dstRect;

            if gfx_adjusthiddenrects(&mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                return;
            }

            blit_surface(self.m_picture.get(), &adjustedSrcRect, blitdest, &mut adjustedDstRect);

            if let Some(wrap_x) = self.m_wrap_x {
                if x + srcRect.w >= wrap_x {
                    adjustedSrcRect = *srcRect;
                    adjustedDstRect = dstRect;
                    adjustedDstRect.x -= wrap_x;

                    if gfx_adjusthiddenrects(&mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                        return;
                    }

                    blit_surface(self.m_picture.get(), &adjustedSrcRect, blitdest, &mut adjustedDstRect);
                } else if x < 0 {
                    adjustedSrcRect = *srcRect;
                    adjustedDstRect = dstRect;
                    adjustedDstRect.x += wrap_x;

                    if gfx_adjusthiddenrects(&mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                        return;
                    }

                    blit_surface(self.m_picture.get(), &adjustedSrcRect, blitdest, &mut adjustedDstRect);
                }
            }
        }
    }

    /// Draw a part of the sprite scaled to a destination area.
    pub fn draw_stretch(&self, srcRect: &SDL_Rect, dst: *mut SDL_Surface, dstRect: &SDL_Rect) {
        debug_assert!(!self.m_picture.is_null());

        let mut dstRect_w = *dstRect;
        unsafe {
            if SDL_UpperBlitScaled(self.m_picture.get(), srcRect, dst, &mut dstRect_w) < 0 {
                eprintln!("SDL_BlitScaled error: {}", sdl_error());
            }
        }
    }

    pub fn setalpha(&mut self, alpha: u8) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            if SDL_SetSurfaceBlendMode(self.m_picture.get(), SDL_BlendMode::SDL_BLENDMODE_BLEND) < 0 {
                eprintln!("\n ERROR: couldn't set blend mode on sprite: {}", sdl_error());
                return;
            }
            if SDL_SetSurfaceAlphaMod(self.m_picture.get(), alpha) < 0 {
                eprintln!("\n ERROR: couldn't set alpha on sprite: {}", sdl_error());
                return;
            }
        }
    }

    pub fn get_width(&self) -> i32 {
        self.m_picture.w
    }
    pub fn get_height(&self) -> i32 {
        self.m_picture.h
    }

    pub fn get_surface(&self) -> *mut SDL_Surface {
        self.m_picture.get()
    }

    pub fn set_wrap(&mut self, wrapsize: i16) {
        self.m_wrap_x = Some(wrapsize as i32);
    }
    pub fn is_wrapping(&self) -> bool {
        self.m_wrap_x.is_some()
    }
}

pub struct ImageLoader {
    m_path: PathBuf,
    m_optimize: bool,
    m_color_key: Option<RGB>,
    m_alpha: Option<u8>,
    m_wrap_x: Option<i32>,
}

impl ImageLoader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        ImageLoader { m_path: path.into(), m_optimize: true, m_color_key: Some(colors::MAGENTA), m_alpha: None, m_wrap_x: None }
    }

    pub fn with_color_key(mut self, key: RGB) -> Self {
        self.m_color_key = Some(key);
        self
    }

    pub fn without_color_key(mut self) -> Self {
        self.m_color_key = None;
        self
    }

    pub fn with_alpha(mut self, alpha: u8) -> Self {
        self.m_alpha = Some(alpha);
        self
    }

    /// `withWrapping(int wrap_x = 640)`
    pub fn with_wrapping(mut self, wrap_x: i32) -> Self {
        self.m_wrap_x = Some(wrap_x);
        self
    }

    pub fn without_optimization(mut self) -> Self {
        self.m_optimize = false;
        self
    }

    pub fn create(&self) -> gfxSprite {
        gfxSprite::from_surface(load_image(&self.m_path, self.m_optimize, self.m_color_key, self.m_alpha), self.m_wrap_x)
    }

    /// `create()` for callers that catch the `std::string` the C++ throws on a load failure.
    pub fn try_create(&self) -> Result<gfxSprite, String> {
        Ok(gfxSprite::from_surface(try_load_image(&self.m_path, self.m_optimize, self.m_color_key, self.m_alpha)?, self.m_wrap_x))
    }
}
