//! Pointer-based blit and fill calls for game surfaces: each records screen writes in the display list, then draws
//! with the software blitter (`soft_surface`).

use crate::common::gfx::soft_surface::{self, Rect, Surface};
use std::sync::atomic::{AtomicBool, Ordering};

static WARNED: AtomicBool = AtomicBool::new(false);

/// Prints the first blit whose surface state the software blitter is not verified against SDL for.
fn warn_unverified(src: &Surface, dst: &Surface) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        println!(
            "[blit] unverified blit: src {}x{} alpha={} key={:?} mod={} blend={:?}, dst alpha={}",
            src.w, src.h, src.alpha, src.color_key, src.alpha_mod, src.blend, dst.alpha
        );
    }
}

/// Blits `src_rect` of `src` (all of it when null) to `dst` at `dst_rect`'s position, clipped, and writes the clipped
/// rectangle back to `dst_rect`. Returns -1 when either surface is null, else 0.
pub unsafe fn upper_blit(src: *mut Surface, src_rect: *const Rect, dst: *mut Surface, dst_rect: *mut Rect) -> i32 {
    if src.is_null() || dst.is_null() {
        return -1;
    }
    let sr = src_rect.as_ref().copied();
    let mut dr = dst_rect.as_ref().copied();
    crate::common::gfx::display_list::before_write(dst);
    crate::common::gfx::display_list::record_blit(src, sr, dst, dr, false);
    if crate::common::gfx::display_list::record_only(dst) {
        soft_surface::clip_blit(&(*src).view(), sr.as_ref(), &mut (*dst).view_mut(), dr.as_mut());
        if let (Some(out), Some(r)) = (dst_rect.as_mut(), dr) {
            *out = r;
        }
        return 0;
    }
    if !soft_surface::blit_supported(&(*src).state(), (*src).alpha, (*dst).alpha) {
        warn_unverified(&*src, &*dst);
    }
    if src == dst {
        let copy = (*src).clone();
        soft_surface::blit(&copy.view(), &copy.state(), sr.as_ref(), &mut (*dst).view_mut(), dr.as_mut());
    } else {
        soft_surface::blit(&(*src).view(), &(*src).state(), sr.as_ref(), &mut (*dst).view_mut(), dr.as_mut());
    }
    if let (Some(out), Some(r)) = (dst_rect.as_mut(), dr) {
        *out = r;
    }
    0
}

/// Fills `rect` (the clip rectangle when null) of `dst` with the raw pixel value `color`. Returns -1 on a null `dst`.
pub unsafe fn fill_rect(dst: *mut Surface, rect: *const Rect, color: u32) -> i32 {
    if dst.is_null() {
        return -1;
    }
    crate::common::gfx::display_list::before_write(dst);
    crate::common::gfx::display_list::record_fill(dst, rect.as_ref().copied(), color);
    if !crate::common::gfx::display_list::record_only(dst) {
        (*dst).fill_rect(rect.as_ref(), color);
    }
    0
}

/// The decoded PNG at `path`.
pub fn decode_for_load(path: &std::path::Path) -> Option<crate::common::gfx::soft_image::Decoded> {
    crate::common::gfx::soft_image::decode_png(&std::fs::read(path).ok()?)
}

/// Stretches `src_rect` of `src` onto `dst_rect` of `dst`. Returns -1 for null or identical surfaces, or a source
/// state the scaler does not support.
pub unsafe fn upper_blit_scaled(src: *mut Surface, src_rect: *const Rect, dst: *mut Surface, dst_rect: *mut Rect) -> i32 {
    if src.is_null() || dst.is_null() || src == dst {
        return -1;
    }
    let sr = src_rect.as_ref().copied();
    let mut dr = dst_rect.as_ref().copied();
    crate::common::gfx::display_list::before_write(dst);
    crate::common::gfx::display_list::record_blit(src, sr, dst, dr, true);
    if crate::common::gfx::display_list::record_only(dst) {
        return 0;
    }
    if !soft_surface::blit_scaled(&(*src).view(), &(*src).state(), sr.as_ref(), &mut (*dst).view_mut(), dr.as_mut()) {
        warn_unverified(&*src, &*dst);
        return -1;
    }
    if let (Some(out), Some(r)) = (dst_rect.as_mut(), dr) {
        *out = r;
    }
    0
}
