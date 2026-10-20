//! What the OpenGL presenters share: loading GL, the letterbox, integer scaling, mouse mapping and window captures.

use sdl2::sys::*;
use smw_platform::{SCREEN_H, SCREEN_W};
use std::ffi::{CStr, CString};

/// The current context's GL functions, loaded through SDL.
pub unsafe fn load_gl() -> glow::Context {
    glow::Context::from_loader_function(|name| {
        let name = CString::new(name).unwrap_or_default();
        SDL_GL_GetProcAddress(name.as_ptr()) as *const _
    })
}

/// Whether `letterbox` keeps to whole multiples of 640x480.
static INTEGER_SCALE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_integer_scale(on: bool) {
    INTEGER_SCALE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The scale and offset that fit 640x480 into a `w` by `h` pixel area, centered.
pub fn letterbox(w: i32, h: i32) -> (f32, f32, f32) {
    let fit = (w as f32 / SCREEN_W as f32).min(h as f32 / SCREEN_H as f32);
    let scale = if INTEGER_SCALE.load(std::sync::atomic::Ordering::Relaxed) && fit >= 1.0 { fit.floor() } else { fit };
    (scale, (w as f32 - SCREEN_W as f32 * scale) / 2.0, (h as f32 - SCREEN_H as f32 * scale) / 2.0)
}

/// Maps mouse events from window points to the 640x480 picture, as SDL's renderer does for its logical size.
unsafe extern "C" fn mouse_to_logical(_: *mut std::ffi::c_void, e: *mut SDL_Event) -> i32 {
    let t = (*e).type_;
    let window_id = if t == SDL_EventType::SDL_MOUSEMOTION as u32 {
        (*e).motion.windowID
    } else if t == SDL_EventType::SDL_MOUSEBUTTONDOWN as u32 || t == SDL_EventType::SDL_MOUSEBUTTONUP as u32 {
        (*e).button.windowID
    } else {
        return 1;
    };
    let window = SDL_GetWindowFromID(window_id);
    if window.is_null() {
        return 1;
    }
    let (mut w, mut h) = (0, 0);
    SDL_GetWindowSize(window, &mut w, &mut h);
    let (dw, dh) = drawable_size(window);
    if w <= 0 || h <= 0 || dw <= 0 || dh <= 0 {
        return 1;
    }
    // Events come in window points; the letterbox is laid out in drawable pixels, which differ on HiDPI screens.
    let px = |x: i32, y: i32| ((x as f32 * dw as f32 / w as f32) as i32, (y as f32 * dh as f32 / h as f32) as i32);
    if t == SDL_EventType::SDL_MOUSEMOTION as u32 {
        let m = &mut (*e).motion;
        let (x, y) = px(m.x, m.y);
        (m.x, m.y) = to_logical(x, y, dw, dh);
        let scale = letterbox(dw, dh).0 * w as f32 / dw as f32;
        (m.xrel, m.yrel) = ((m.xrel as f32 / scale) as i32, (m.yrel as f32 / scale) as i32);
    } else {
        let b = &mut (*e).button;
        let (x, y) = px(b.x, b.y);
        (b.x, b.y) = to_logical(x, y, dw, dh);
    }
    1
}

/// A pixel in a `w` by `h` drawable as a 640x480 picture pixel; pixels in the bars fall outside 0..640 or 0..480.
fn to_logical(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let (scale, ox, oy) = letterbox(w, h);
    (((x as f32 - ox) / scale).floor() as i32, ((y as f32 - oy) / scale).floor() as i32)
}

/// The window's size in physical pixels.
pub unsafe fn drawable_size(window: *mut SDL_Window) -> (i32, i32) {
    let (mut w, mut h) = (0, 0);
    SDL_GL_GetDrawableSize(window, &mut w, &mut h);
    (w, h)
}

/// Adds or removes the watch that maps mouse events to the picture.
pub unsafe fn watch_mouse(on: bool) {
    if on {
        SDL_AddEventWatch(Some(mouse_to_logical), std::ptr::null_mut());
    } else {
        SDL_DelEventWatch(Some(mouse_to_logical), std::ptr::null_mut());
    }
}

/// `SMW_SHADER_SHOT`: the present whose window is saved as `shader_<n>.bmp` in `SMW_SHOT_DIR`.
pub fn shot_frame() -> Option<usize> {
    std::env::var("SMW_SHADER_SHOT").ok().and_then(|n| n.parse().ok())
}

/// Writes the bound back buffer, `w` by `h`, overlay and letterbox included, as `shader_<n>.bmp`.
pub unsafe fn save_shot(gl: &glow::Context, w: i32, h: i32, n: usize) {
    use glow::HasContext;
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    gl.bind_framebuffer(glow::FRAMEBUFFER, Option::None);
    gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut rgba)));
    // The back buffer's first row is the bottom of the window.
    let mut upright: Vec<u8> = rgba.chunks_exact(w * 4).rev().flatten().copied().collect();
    let dir = std::env::var("SMW_SHOT_DIR").unwrap_or_else(|_| ".".to_string());
    let path = CString::new(format!("{dir}/shader_{n}.bmp")).unwrap_or_default();
    let format = SDL_PixelFormatEnum::SDL_PIXELFORMAT_ABGR8888 as u32;
    let surface = SDL_CreateRGBSurfaceWithFormatFrom(upright.as_mut_ptr() as *mut _, w as i32, h as i32, 32, (w * 4) as i32, format);
    if surface.is_null() || SDL_SaveBMP_RW(surface, SDL_RWFromFile(path.as_ptr(), c"wb".as_ptr()), 1) != 0 {
        eprintln!("[gfx] Could not save {}: {}", path.to_string_lossy(), CStr::from_ptr(SDL_GetError()).to_string_lossy());
    }
    SDL_FreeSurface(surface);
}

#[cfg(test)]
mod tests {
    use super::to_logical;

    #[test]
    fn window_points_map_to_picture_pixels() {
        assert_eq!(to_logical(0, 0, 640, 480), (0, 0));
        assert_eq!(to_logical(639, 479, 640, 480), (639, 479));
        assert_eq!(to_logical(640, 480, 1280, 960), (320, 240));
        // Pillarboxed: 1600x960 has 160-point bars at each side.
        assert_eq!(to_logical(160, 0, 1600, 960), (0, 0));
        assert_eq!(to_logical(100, 10, 1600, 960).0, -30);
        // Letterboxed: 640x600 has 60-point bars above and below.
        assert_eq!(to_logical(320, 60, 640, 600), (320, 0));
        assert_eq!(to_logical(320, 539, 640, 600), (320, 479));
    }
}
