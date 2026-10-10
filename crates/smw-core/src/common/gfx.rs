//! Port of src/common/gfx.cpp

pub mod blit;
pub mod color;
pub mod gfx_font;
pub mod gfx_palette;
pub mod gfx_sdl;
pub mod gfx_sprite;
pub mod soft_image;
pub mod soft_surface;

use crate::common::gfx::color::{colors, RGB};
use crate::common::gfx::gfx_palette::{gfxPalette, PlayerPalette};
use crate::common::gfx::gfx_sdl::GraphicsSDL;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge, ImageLoader};
use crate::common::global_constants::PGFX_LAST;
use crate::common::util::sdl_helpers::SdlSurfacePtr;
use crate::globals::*;
use sdl2::sys::image::IMG_Load;
use sdl2::sys::*;
use std::ffi::{CStr, CString};
use std::path::Path;

pub type SpriteStrip = [gfxSprite; PGFX_LAST as usize];

pub fn new_sprite_strip() -> SpriteStrip {
    std::array::from_fn(|_| gfxSprite::new())
}

pub static mut gfx: Global<GraphicsSDL> = Global::uninit();
pub static mut gfx_palette: Global<gfxPalette> = Global::uninit();

/// Constructors of the globals defined in gfx.cpp, in definition order.
pub fn init_globals() {
    unsafe {
        gfx.init(GraphicsSDL::new());
        gfx_palette.init(gfxPalette::new());
    }
}

/// Not in upstream: `SDL_SetSurfaceRLE` only on the web by default (docs/sdl2-compat-rle.md); `SMW_RLE=0`/`1` overrides.
pub fn rle_enabled() -> bool {
    static RLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *RLE.get_or_init(|| match std::env::var("SMW_RLE").as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => cfg!(target_os = "emscripten"),
    })
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

pub unsafe fn get_raw_pixel(surf: *mut SDL_Surface, x: i32, y: i32) -> u32 {
    assert!(!surf.is_null());
    assert!(0 <= x && x < (*surf).w);
    assert!(0 <= y && y < (*surf).h);

    let bpp = (*(*surf).format).BytesPerPixel;
    let idx = (y * (*surf).pitch + x * bpp as i32) as usize;
    let pixel8 = ((*surf).pixels as *const u8).add(idx);

    match bpp {
        1 => *pixel8 as u32,
        2 => (pixel8 as *const u16).read_unaligned() as u32,
        3 => {
            if SDL_BYTEORDER == SDL_BIG_ENDIAN {
                (*pixel8 as u32) << 16 | (*pixel8.add(1) as u32) << 8 | *pixel8.add(2) as u32
            } else {
                *pixel8 as u32 | (*pixel8.add(1) as u32) << 8 | (*pixel8.add(2) as u32) << 16
            }
        }
        4 => (pixel8 as *const u32).read_unaligned(),
        _ => unreachable!(),
    }
}

unsafe fn set_raw_pixel(surf: *mut SDL_Surface, x: i32, y: i32, value: u32) {
    assert!(!surf.is_null());
    assert!(0 <= x && x < (*surf).w);
    assert!(0 <= y && y < (*surf).h);

    let bpp = (*(*surf).format).BytesPerPixel;
    let idx = (y * (*surf).pitch + x * bpp as i32) as usize;
    let pixel8 = ((*surf).pixels as *mut u8).add(idx);

    match bpp {
        1 => *pixel8 = value as u8,
        2 => (pixel8 as *mut u16).write_unaligned(value as u16),
        3 => {
            if SDL_BYTEORDER == SDL_BIG_ENDIAN {
                *pixel8 = ((value >> 16) & 0xFF) as u8;
                *pixel8.add(1) = ((value >> 8) & 0xFF) as u8;
                *pixel8.add(2) = (value & 0xFF) as u8;
            } else {
                *pixel8 = (value & 0xFF) as u8;
                *pixel8.add(1) = ((value >> 8) & 0xFF) as u8;
                *pixel8.add(2) = ((value >> 16) & 0xFF) as u8;
            }
        }
        4 => (pixel8 as *mut u32).write_unaligned(value),
        _ => unreachable!(),
    }
}

unsafe fn set_rgb(surf: *mut SDL_Surface, x: i32, y: i32, color: &RGB) {
    assert!(!surf.is_null());
    let rawPixel = SDL_MapRGB((*surf).format, color.r, color.g, color.b);
    set_raw_pixel(surf, x, y, rawPixel);
}

#[inline]
fn must_lock(surf: *mut SDL_Surface) -> bool {
    unsafe { (*surf).flags & SDL_RLEACCEL != 0 }
}

/// Makes team-colored skin surface frame from a loaded sprite strip.
fn create_skin_surface(source: &gfxSprite, sourceFrame: usize, team: usize, allStates: bool, mirrored: bool) -> gfxSprite {
    unsafe {
        //Take the loaded skin and colorize it for each state (normal, 3 frames of invincibility, shielded, tagged, ztarred, got shine, frozen)
        let outFrameCount: usize = if allStates { gfx_palette::COUNT as usize } else { 1 };

        let out = gfxSprite::blank(32 * outFrameCount as u32, 32);

        if must_lock(out.get_surface()) {
            SDL_LockSurface(out.get_surface());
        }

        if must_lock(source.get_surface()) {
            SDL_LockSurface(source.get_surface());
        }

        let startX = (sourceFrame * 32) as i32;

        for y in 0..32 {
            for srcX in 0..32 {
                let dstX = if mirrored { 31 - srcX } else { srcX };

                let pixelColor = get_rgb(source.get_surface(), startX + srcX, y);

                if let Some(sheet) = gfx_palette.color_sheets().get(&pixelColor) {
                    for outFrame in 0..outFrameCount {
                        let paletteColor = sheet.replacement_for(team, outFrame as PlayerPalette);
                        set_rgb(out.get_surface(), outFrame as i32 * 32 + dstX, y, &paletteColor);
                    }
                } else {
                    for outFrame in 0..outFrameCount {
                        set_rgb(out.get_surface(), outFrame as i32 * 32 + dstX, y, &pixelColor);
                    }
                }
            }
        }

        SDL_UnlockSurface(source.get_surface());
        SDL_UnlockSurface(out.get_surface());

        let color_key = SDL_MapRGB((*out.get_surface()).format, colors::MAGENTA.r, colors::MAGENTA.g, colors::MAGENTA.b);
        if SDL_SetColorKey(out.get_surface(), 1, color_key) < 0 {
            panic!("Couldn't set color key for new skin surface: {}", sdl_error());
        }
        if rle_enabled() && SDL_SetSurfaceRLE(out.get_surface(), 1) < 0 {
            panic!("Couldn't set RLE acceleration for new skin surface: {}", sdl_error());
        }

        out
    }
}

fn valid_skin_surface(skin: &gfxSprite) -> bool {
    skin.get_width() == 192 && skin.get_height() == 32
}

pub fn get_rgb(surf: *mut SDL_Surface, x: i32, y: i32) -> RGB {
    unsafe {
        assert!(!surf.is_null());
        let rawPixel = get_raw_pixel(surf, x, y);
        let mut color = RGB::default();
        SDL_GetRGB(rawPixel, (*surf).format, &mut color.r, &mut color.g, &mut color.b);
        color
    }
}

pub fn gfx_init(w: i32, h: i32, fullscreen: bool) -> bool {
    unsafe { (*gfx).init(fullscreen) }
}

pub fn gfx_changefullscreen(fullscreen: bool) {
    unsafe { gfx.change_full_screen(fullscreen) }
}

pub fn gfx_flipscreen() {
    unsafe { gfx.flip_screen() }
}

pub fn gfx_settitle(title: &str) {
    unsafe { gfx.set_title(title) }
}

pub fn gfx_show_catched_error(error: &str) {
    let mut message = String::from(
        "It seems the game has unexpectedly crashed. If you could tell us\n\
         what happened exactly, we might be able to fix this bug. Consider\n\
         reporting it on the link below, thanks!\n\n\
         https://github.com/mmatyas/supermariowar/issues\n\n\
         Sincerely,\nThe Developers",
    );
    if !error.is_empty() {
        message += "\n\n\nThe error message:\n";
        message += error;
    }
    eprintln!("\n{}", message);
    unsafe { gfx.show_error_box(&message) }
}

pub fn gfx_take_screenshot() {
    unsafe { gfx.take_screenshot() }
}

pub fn gfx_close() {}

pub fn gfx_loadpalette(palette_path: &Path) -> bool {
    unsafe { gfx_palette.load(palette_path) }
}

/// Not in the C++: writes `screen` as a BMP for the differential replay harness.
pub fn gfx_save_screen_bmp(path: &str) -> bool {
    let cpath = CString::new(path).unwrap();
    unsafe {
        let rw = SDL_RWFromFile(cpath.as_ptr(), b"wb\0".as_ptr() as *const _);
        !rw.is_null() && SDL_SaveBMP_RW(screen, rw, 1) == 0
    }
}

/// Appends the screen's 640x480 pixels to `out` as raw ARGB8888 (BGRA bytes on little-endian).
pub fn gfx_write_screen_raw(out: &mut impl std::io::Write) -> std::io::Result<()> {
    unsafe {
        let argb = SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32;
        let surface = if (*(*screen).format).format == argb {
            screen
        } else {
            SDL_ConvertSurfaceFormat(screen, argb, 0)
        };
        if surface.is_null() {
            return Err(std::io::Error::other("cannot convert the screen surface"));
        }
        SDL_LockSurface(surface);
        let (w, rows, pitch) = ((*surface).w as usize * 4, (*surface).h as usize, (*surface).pitch as usize);
        let pixels = std::slice::from_raw_parts((*surface).pixels as *const u8, pitch * rows);
        let result = (0..rows).try_for_each(|y| out.write_all(&pixels[y * pitch..y * pitch + w]));
        SDL_UnlockSurface(surface);
        if surface != screen {
            SDL_FreeSurface(surface);
        }
        result
    }
}

fn load_skin(path: &Path) -> Result<gfxSprite, String> {
    let skin = ImageLoader::new(path).without_color_key().without_optimization().try_create()?;

    if !valid_skin_surface(&skin) {
        return Err(format!("Invalid skin file: {} has incorrect dimensions", path.display()));
    }

    Ok(skin)
}

/// Returns `Err` where the C++ throws a `std::string`.
pub fn gfx_loadmenuskin(path: &Path, colorScheme: i16, fLoadBothDirections: bool) -> Result<SpriteStrip, String> {
    let skin = load_skin(path)?;

    let mut strip = new_sprite_strip();

    for i in 0..2usize {
        let skinSurface = create_skin_surface(&skin, i, colorScheme as usize, true, false);
        strip[i * 2] = skinSurface;
    }

    if fLoadBothDirections {
        for i in 0..2usize {
            let skinSurface = create_skin_surface(&skin, i, colorScheme as usize, true, true);
            strip[i * 2 + 1] = skinSurface;
        }
    }

    Ok(strip)
}

/// Returns `Err` where the C++ throws a `std::string`.
pub fn gfx_loadfullskin(path: &Path, colorScheme: i16) -> Result<SpriteStrip, String> {
    let skin = load_skin(path)?;

    let mut strip = new_sprite_strip();

    for k in 0..4usize {
        for j in 0..2usize {
            let skinSurface = create_skin_surface(&skin, k, colorScheme as usize, true, j != 0);
            strip[(k * 2) + j] = skinSurface;
        }
    }

    //Dead Flying Sprite
    let skinSurface = create_skin_surface(&skin, 4, colorScheme as usize, true, false);
    strip[8] = skinSurface;

    //Dead Stomped Sprite
    let skinSurface = create_skin_surface(&skin, 5, colorScheme as usize, true, false);
    strip[9] = skinSurface;

    Ok(strip)
}

pub fn gfx_cliprect(srcRect: &mut SDL_Rect, dstRect: &mut SDL_Rect, clipArea: &SDL_Rect) {
    if dstRect.x >= clipArea.x + clipArea.w
        || dstRect.x + dstRect.w < clipArea.x
        || dstRect.y >= clipArea.y + clipArea.h
        || dstRect.y + dstRect.h < clipArea.y
    {
        srcRect.w = 0;
        srcRect.h = 0;
        return;
    }

    if dstRect.x < clipArea.x {
        let iDiffX = clipArea.x - dstRect.x;
        srcRect.x += iDiffX;
        srcRect.w -= iDiffX;
        dstRect.x = clipArea.x;
    }

    if dstRect.x + dstRect.w >= clipArea.x + clipArea.w {
        let iDiffX = dstRect.x + dstRect.w - clipArea.x - clipArea.w;
        srcRect.w -= iDiffX;
    }

    if dstRect.y < clipArea.y {
        let iDiffY = clipArea.y - dstRect.y;
        srcRect.y += iDiffY;
        srcRect.h -= iDiffY;
        dstRect.y = clipArea.y;
    }

    if dstRect.y + dstRect.h >= clipArea.y + clipArea.h {
        let iDiffY = dstRect.y + dstRect.h - clipArea.y - clipArea.h;
        srcRect.h -= iDiffY;
    }
}

/// Clips a source and destination area pair, so that the destination area doesn't go past
/// a certain threshold in a given direction. Returns true if the destination area is fully hidden.
pub fn gfx_adjusthiddenrects(src: &mut SDL_Rect, dst: &mut SDL_Rect, edge: ClipEdge, threshold: i32) -> bool {
    unsafe {
        match edge {
            ClipEdge::Top => {
                if threshold < dst.y {
                    // fully visible
                } else if dst.y + dst.h <= threshold {
                    return true;
                } else {
                    let diff = threshold - dst.y;
                    src.y += diff;
                    src.h -= diff;
                    dst.y = threshold + y_shake as i32;
                    dst.h = src.h;
                }
            }
            ClipEdge::Right => {
                if dst.x + src.w < threshold {
                    // fully visible
                } else if threshold <= dst.x {
                    return true;
                } else {
                    src.w = threshold - dst.x;
                    dst.w = src.w;
                }
            }
            ClipEdge::Bottom => {
                if dst.y + src.h < threshold {
                    // fully visible
                } else if threshold <= dst.y {
                    return true;
                } else {
                    src.h = threshold - dst.y;
                    dst.h = src.h;
                }
            }
            ClipEdge::Left => {
                if threshold < dst.x {
                    // fully visible
                } else if dst.x + src.w <= threshold {
                    return true;
                } else {
                    let diff = threshold - dst.x;
                    src.x += diff;
                    src.w -= diff;
                    dst.x = threshold + x_shake as i32;
                    dst.w = src.w;
                }
            }
        }
    }

    false
}

pub fn gfx_drawpreview(
    sprite: &gfxSprite,
    dstX: i16,
    dstY: i16,
    srcX: i16,
    srcY: i16,
    iw: i16,
    ih: i16,
    clipRect: &SDL_Rect,
    wrap: bool,
    clip: Option<(ClipEdge, i32)>,
) {
    unsafe {
        //need to set source rect before each blit so it can be clipped correctly
        let mut rSrcRect = SDL_Rect { x: srcX as i32, y: srcY as i32, w: iw as i32, h: ih as i32 };
        let mut rDstRect = SDL_Rect { x: dstX as i32, y: dstY as i32, w: iw as i32, h: ih as i32 };

        gfx_cliprect(&mut rSrcRect, &mut rDstRect, clipRect);

        if let Some((edge, threshold)) = clip {
            if gfx_adjusthiddenrects(&mut rSrcRect, &mut rDstRect, edge, threshold) {
                return;
            }
        }

        // Blit onto the screen surface
        sprite.draw_src_to(&rSrcRect, blitdest, &rDstRect);

        if wrap {
            //Deal with wrapping over sides of screen
            let mut fBlitSide = false;
            if (dstX as i32) < clipRect.x {
                rDstRect.x = dstX as i32 + 320;
                fBlitSide = true;
            } else if dstX as i32 + iw as i32 >= clipRect.x + clipRect.w {
                rDstRect.x = dstX as i32 - 320;
                fBlitSide = true;
            }

            if fBlitSide {
                //need to set source rect before each blit so it can be clipped correctly
                rSrcRect.x = srcX as i32;
                rSrcRect.y = srcY as i32;
                rSrcRect.w = iw as i32;
                rSrcRect.h = ih as i32;

                rDstRect.y = dstY as i32;
                rDstRect.w = iw as i32;
                rDstRect.h = ih as i32;

                gfx_cliprect(&mut rSrcRect, &mut rDstRect, clipRect);

                if let Some((edge, threshold)) = clip {
                    if gfx_adjusthiddenrects(&mut rSrcRect, &mut rDstRect, edge, threshold) {
                        return;
                    }
                }

                sprite.draw_src_to(&rSrcRect, blitdest, &rDstRect);
            }
        }
    }
}

pub fn gfx_setjoystickteamcolor(joystick: *mut SDL_Joystick, team: i16, mut brightness: f32) {
    if team < 0 {
        // A non-playing user has no team
        return;
    }
    brightness = 0f32.max(1f32.min(brightness));
    unsafe {
        if let Some(color) = gfx_palette.replacement_for(RGB { r: 0x80, g: 0x00, b: 0x00 }, team as usize, gfx_palette::normal) {
            SDL_JoystickSetLED(
                joystick,
                (brightness * color.r as f32) as u8,
                (brightness * color.g as f32) as u8,
                (brightness * color.b as f32) as u8,
            );
        }
    }
}
