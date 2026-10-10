//! `SDL_UpperBlit` and `SDL_FillRect` for the game's surfaces. `SMW_BLIT=rust` draws with the software blitter
//! (`soft_surface`) on ARGB8888 surfaces; the default, `SMW_BLIT=sdl`, calls SDL.

use crate::common::gfx::soft_surface::{self, BlendMode, BlitState, Pixels, PixelsMut, Rect};
use sdl2::sys::*;

pub fn rust_blit() -> bool {
    static RUST: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *RUST.get_or_init(|| std::env::var("SMW_BLIT").as_deref() == Ok("rust"))
}

unsafe fn is_argb8888(surf: *mut SDL_Surface) -> bool {
    !surf.is_null()
        && !(*surf).pixels.is_null()
        && (*surf).flags & SDL_RLEACCEL == 0
        && (*(*surf).format).format == SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32
        && (*surf).pitch % 4 == 0
}

unsafe fn clip_of(surf: *mut SDL_Surface) -> Rect {
    let c = (*surf).clip_rect;
    Rect::new(c.x, c.y, c.w, c.h)
}

unsafe fn pixels_mut<'a>(surf: *mut SDL_Surface) -> PixelsMut<'a> {
    let pitch = (*surf).pitch as usize / 4;
    PixelsMut {
        w: (*surf).w,
        h: (*surf).h,
        pitch,
        pixels: std::slice::from_raw_parts_mut((*surf).pixels as *mut u32, pitch * (*surf).h as usize),
        clip: clip_of(surf),
    }
}

/// The source state, or `None` for a state the software blitter does not reproduce.
unsafe fn blit_state(src: *mut SDL_Surface) -> Option<BlitState> {
    let mut mode = SDL_BlendMode::SDL_BLENDMODE_NONE;
    SDL_GetSurfaceBlendMode(src, &mut mode);
    let blend = match mode {
        SDL_BlendMode::SDL_BLENDMODE_NONE => BlendMode::None,
        SDL_BlendMode::SDL_BLENDMODE_BLEND => BlendMode::Blend,
        _ => return Option::None,
    };
    let (mut r, mut g, mut b) = (0, 0, 0);
    SDL_GetSurfaceColorMod(src, &mut r, &mut g, &mut b);
    if (r, g, b) != (255, 255, 255) {
        return Option::None;
    }
    let mut alpha_mod = 255;
    SDL_GetSurfaceAlphaMod(src, &mut alpha_mod);
    let mut key = 0;
    let color_key = (SDL_GetColorKey(src, &mut key) == 0).then_some(key);
    Some(BlitState { color_key, alpha_mod, blend })
}

/// `SDL_UpperBlit`.
pub unsafe fn upper_blit(src: *mut SDL_Surface, src_rect: *const SDL_Rect, dst: *mut SDL_Surface, dst_rect: *mut SDL_Rect) -> i32 {
    if !rust_blit() || src == dst || !is_argb8888(src) || !is_argb8888(dst) {
        return SDL_UpperBlit(src, src_rect, dst, dst_rect);
    }
    let Some(state) = blit_state(src) else {
        return SDL_UpperBlit(src, src_rect, dst, dst_rect);
    };
    let spitch = (*src).pitch as usize / 4;
    let view = Pixels {
        w: (*src).w,
        h: (*src).h,
        pitch: spitch,
        pixels: std::slice::from_raw_parts((*src).pixels as *const u32, spitch * (*src).h as usize),
    };
    let sr = src_rect.as_ref().map(|r| Rect::new(r.x, r.y, r.w, r.h));
    let mut dr = dst_rect.as_ref().map(|r| Rect::new(r.x, r.y, r.w, r.h));
    soft_surface::blit(&view, &state, sr.as_ref(), &mut pixels_mut(dst), dr.as_mut());
    if let (Some(out), Some(r)) = (dst_rect.as_mut(), dr) {
        *out = SDL_Rect { x: r.x, y: r.y, w: r.w, h: r.h };
    }
    0
}

/// `SDL_FillRect`.
pub unsafe fn fill_rect(dst: *mut SDL_Surface, rect: *const SDL_Rect, color: u32) -> i32 {
    if !rust_blit() || !is_argb8888(dst) {
        return SDL_FillRect(dst, rect, color);
    }
    let r = rect.as_ref().map(|r| Rect::new(r.x, r.y, r.w, r.h));
    soft_surface::fill_rect(&mut pixels_mut(dst), r.as_ref(), color);
    0
}
