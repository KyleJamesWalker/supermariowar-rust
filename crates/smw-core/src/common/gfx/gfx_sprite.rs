//! Port of src/common/gfx/gfxSprite.cpp

use crate::common::gfx::soft_surface::{Surface, SurfacePtr};
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::color::{colors, RGB};
use crate::common::gfx::{blit, gfx_adjusthiddenrects, soft_image};
use crate::globals::*;
use crate::common::gfx::soft_surface::BlendMode;
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

/// Mirrors the C++ `throw std::format(...)`: loading failures are fatal unless a caller catches them.
fn throw(msg: String) -> ! {
    panic!("{}", msg)
}

fn try_load_image(_g: &mut Game, path: &Path, _optimize: bool, color_key: Option<RGB>, alpha: Option<u8>) -> Result<SurfacePtr, String> {
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

    let Some(image) = blit::decode_for_load(path) else {
        return Err(format!("Couldn't load {}: not a readable PNG", path_str));
    };
    let key = color_key.map(|c| image.map_rgb(c.r, c.g, c.b));
    let mut img = soft_image::convert_to_screen(&image, key);
    if let Some(alpha) = alpha {
        img.blend = BlendMode::Blend;
        img.alpha_mod = alpha;
    }

    println!(" done");
    Ok(SurfacePtr::new(img))
}

fn load_image(g: &mut Game, path: &Path, optimize: bool, color_key: Option<RGB>, alpha: Option<u8>) -> SurfacePtr {
    try_load_image(g, path, optimize, color_key, alpha).unwrap_or_else(|what| throw(what))
}


unsafe fn blit_surface(src: *mut Surface, srcArea: *const Rect, dst: *mut Surface, dstArea: *mut Rect) {
    if crate::common::gfx::blit::upper_blit(src, srcArea, dst, dstArea) < 0 {
        eprintln!("SDL_BlitSurface error: null surface");
    }
}

#[derive(Default)]
pub struct gfxSprite {
    m_picture: SurfacePtr,
    m_wrap_x: Option<i32>,
    pub _alias: Aliased,
}

impl gfxSprite {
    pub const fn new() -> Self {
        gfxSprite { _alias: Aliased::new(), m_picture: SurfacePtr::null(), m_wrap_x: None }
    }

    /// `gfxSprite(SurfacePtr image, wrap = 640)`
    pub fn from_surface(image: SurfacePtr, wrap: Option<i32>) -> Self {
        gfxSprite { _alias: Aliased::new(), m_picture: image, m_wrap_x: wrap }
    }

    /// `gfxSprite::blank(w, h)`: a screen-format surface with blending off.
    pub fn blank(g: &mut Game, w: u32, h: u32) -> Self {
        let screen_alpha = unsafe { (*g.screen).alpha };
        let surf = Surface { blend: BlendMode::None, alpha: screen_alpha, ..Surface::new(w as i32, h as i32) };
        Self::from_surface(SurfacePtr::new(surf), Some(640))
    }

    /// `draw(Surface* dst, Vec2i dstPos)`: the whole sprite onto `dst`, no camera shake.
    pub fn draw_to_pos(&self, dst: *mut Surface, dstX: i32, dstY: i32) {
        self.blit(null(), dst, dstX, dstY);
    }

    /// `draw(Surface* dst, const Rect& dstRect)`: the whole sprite onto `dst`, no camera shake.
    pub fn draw_to(&self, dst: *mut Surface, dstRect: &Rect) {
        self.blit(null(), dst, dstRect.x, dstRect.y);
    }

    /// `draw(const Rect& srcRect, Surface* dst, Vec2i dstPos)`: part of the sprite onto `dst`.
    pub fn draw_src_to_pos(&self, srcRect: &Rect, dst: *mut Surface, dstX: i32, dstY: i32) {
        self.blit(srcRect, dst, dstX, dstY);
    }

    /// `draw(const Rect& srcRect, Surface* dst, const Rect& dstRect)`: part of the sprite onto `dst`.
    pub fn draw_src_to(&self, srcRect: &Rect, dst: *mut Surface, dstRect: &Rect) {
        self.blit(srcRect, dst, dstRect.x, dstRect.y);
    }

    fn blit(&self, srcRect: *const Rect, dst: *mut Surface, dstPosX: i32, dstPosY: i32) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            let mut dstRect = Rect { x: dstPosX, y: dstPosY, w: 0, h: 0 };
            blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);

            if let Some(wrap_x) = self.m_wrap_x {
                if dstRect.x + self.get_width() >= wrap_x {
                    dstRect = Rect { x: dstPosX - wrap_x, y: dstPosY, w: 0, h: 0 }; // SDL2 modifies the dst rect
                    blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);
                } else if dstRect.x < 0 {
                    dstRect = Rect { x: dstPosX + wrap_x, y: dstPosY, w: 0, h: 0 }; // SDL2 modifies the dst rect
                    blit_surface(self.m_picture.get(), srcRect, dst, &mut dstRect);
                }
            }
        }
    }

    /// Draw the whole sprite at the given coordinate.
    pub fn draw(&self, g: &mut Game, x: i32, y: i32) {
        unsafe { self.blit(null(), g.blitdest, x + g.x_shake as i32, y + g.y_shake as i32) };
    }

    /// Draw part of the sprite at the given coordinate.
    pub fn draw_src(&self, g: &mut Game, x: i32, y: i32, srcRect: &Rect) {
        unsafe { self.blit(srcRect, g.blitdest, x + g.x_shake as i32, y + g.y_shake as i32) };
    }

    /// `draw(x, y, srcx, srcy, w, h)` convenience used all over the C++ via `Rect{...}` temporaries.
    pub fn draw_part(&self, g: &mut Game, x: i32, y: i32, srcx: i32, srcy: i32, w: i32, h: i32) {
        self.draw_src(g, x, y, &Rect { x: srcx, y: srcy, w, h });
    }

    /// Draw part of the sprite at the given coordinate, clipped along a given direction.
    pub fn draw_clip(&self, g: &mut Game, x: i32, y: i32, srcRect: &Rect, clipEdge: ClipEdge, clipTreshold: i32) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            let dstRect = Rect { x: x + g.x_shake as i32, y: y + g.y_shake as i32, w: srcRect.w, h: srcRect.h };
            let mut adjustedSrcRect = *srcRect;
            let mut adjustedDstRect = dstRect;

            if gfx_adjusthiddenrects(g, &mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                return;
            }

            blit_surface(self.m_picture.get(), &adjustedSrcRect, g.blitdest, &mut adjustedDstRect);

            if let Some(wrap_x) = self.m_wrap_x {
                if x + srcRect.w >= wrap_x {
                    adjustedSrcRect = *srcRect;
                    adjustedDstRect = dstRect;
                    adjustedDstRect.x -= wrap_x;

                    if gfx_adjusthiddenrects(g, &mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                        return;
                    }

                    blit_surface(self.m_picture.get(), &adjustedSrcRect, g.blitdest, &mut adjustedDstRect);
                } else if x < 0 {
                    adjustedSrcRect = *srcRect;
                    adjustedDstRect = dstRect;
                    adjustedDstRect.x += wrap_x;

                    if gfx_adjusthiddenrects(g, &mut adjustedSrcRect, &mut adjustedDstRect, clipEdge, clipTreshold) {
                        return;
                    }

                    blit_surface(self.m_picture.get(), &adjustedSrcRect, g.blitdest, &mut adjustedDstRect);
                }
            }
        }
    }

    /// Draw a part of the sprite scaled to a destination area.
    pub fn draw_stretch(&self, srcRect: &Rect, dst: *mut Surface, dstRect: &Rect) {
        debug_assert!(!self.m_picture.is_null());

        let mut dstRect_w = *dstRect;
        unsafe {
            if blit::upper_blit_scaled(self.m_picture.get(), srcRect, dst, &mut dstRect_w) < 0 {
                eprintln!("SDL_BlitScaled error: unsupported scaled blit");
            }
        }
    }

    pub fn setalpha(&mut self, alpha: u8) {
        debug_assert!(!self.m_picture.is_null());

        unsafe {
            let pic = &mut *self.m_picture.get();
            pic.blend = BlendMode::Blend;
            pic.alpha_mod = alpha;
        }
    }

    pub fn get_width(&self) -> i32 {
        self.m_picture.w
    }
    pub fn get_height(&self) -> i32 {
        self.m_picture.h
    }

    pub fn get_surface(&self) -> *mut Surface {
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

    pub fn create(&self, g: &mut Game) -> gfxSprite {
        gfxSprite::from_surface(load_image(g, &self.m_path, self.m_optimize, self.m_color_key, self.m_alpha), self.m_wrap_x)
    }

    /// `create()` for callers that catch the `std::string` the C++ throws on a load failure.
    pub fn try_create(&self, g: &mut Game) -> Result<gfxSprite, String> {
        Ok(gfxSprite::from_surface(try_load_image(g, &self.m_path, self.m_optimize, self.m_color_key, self.m_alpha)?, self.m_wrap_x))
    }
}
