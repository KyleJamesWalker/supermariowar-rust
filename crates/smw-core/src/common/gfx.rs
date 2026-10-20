//! Graphics entry points: window and screen calls, player skin loading and recoloring, and clipped preview drawing.

pub mod blit;
pub mod display_list;
pub mod color;
pub mod gfx_font;
pub mod gfx_palette;
pub mod gfx_sdl;
pub mod gfx_sprite;
pub mod soft_image;
pub mod soft_surface;

use crate::common::gfx::soft_surface::{Surface, SurfacePtr};
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::color::{colors, RGB};
use crate::common::gfx::gfx_palette::{gfxPalette, PlayerPalette};
use crate::common::gfx::gfx_sdl::GraphicsSDL;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge, ImageLoader};
use crate::common::global_constants::PGFX_LAST;
use crate::globals::*;
use std::path::Path;

/// A player's skin frames, indexed by the `PGFX_*` constants.
pub type SpriteStrip = [gfxSprite; PGFX_LAST as usize];

pub fn new_sprite_strip() -> SpriteStrip {
    std::array::from_fn(|_| gfxSprite::new())
}

/// Creates the graphics and palette globals.
pub fn init_globals(g: &mut Game) {
    g.gfx.init(GraphicsSDL::new());
    g.gfx_palette.init(gfxPalette::new());
}

/// Pixel value at (`x`, `y`) of `surf` in its own format; panics outside the surface.
pub unsafe fn get_raw_pixel(surf: *mut Surface, x: i32, y: i32) -> u32 {
    assert!(!surf.is_null());
    assert!(0 <= x && x < (*surf).w);
    assert!(0 <= y && y < (*surf).h);
    (*surf).pixel(x, y)
}

unsafe fn set_rgb(surf: *mut Surface, x: i32, y: i32, color: &RGB) {
    assert!(!surf.is_null());
    assert!(0 <= x && x < (*surf).w);
    assert!(0 <= y && y < (*surf).h);
    let s = &mut *surf;
    let value = s.map_rgb(color.r, color.g, color.b);
    s.pixels[(y * s.w + x) as usize] = value;
}

/// Recolors 32x32 frame `sourceFrame` of a skin for `team`: one frame per palette state side by side when `allStates`,
/// else the normal state only.
fn create_skin_surface(g: &mut Game, source: &gfxSprite, sourceFrame: usize, team: usize, allStates: bool, mirrored: bool) -> gfxSprite {
    unsafe {
        //Take the loaded skin and colorize it for each state (normal, 3 frames of invincibility, shielded, tagged, ztarred, got shine, frozen)
        let outFrameCount: usize = if allStates { gfx_palette::COUNT as usize } else { 1 };

        let out = gfxSprite::blank(g, 32 * outFrameCount as u32, 32);

        let startX = (sourceFrame * 32) as i32;

        for y in 0..32 {
            for srcX in 0..32 {
                let dstX = if mirrored { 31 - srcX } else { srcX };

                let pixelColor = get_rgb(source.get_surface(), startX + srcX, y);

                if let Some(sheet) = g.gfx_palette.color_sheets().get(&pixelColor) {
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

        let s = &mut *out.get_surface();
        s.color_key = Some(s.map_rgb(colors::MAGENTA.r, colors::MAGENTA.g, colors::MAGENTA.b));

        out
    }
}

/// A skin image is six 32x32 frames in a row.
fn valid_skin_surface(skin: &gfxSprite) -> bool {
    skin.get_width() == 192 && skin.get_height() == 32
}

/// Color of the pixel at (`x`, `y`) of `surf`; panics outside the surface.
pub fn get_rgb(surf: *mut Surface, x: i32, y: i32) -> RGB {
    unsafe {
        assert!(!surf.is_null());
        let (r, g, b) = (*surf).get_rgb(get_raw_pixel(surf, x, y));
        RGB { r, g, b }
    }
}

/// Opens the game window; `w` and `h` are unused, the screen is always 640x480.
pub fn gfx_init(g: &mut Game, w: i32, h: i32, fullscreen: bool) -> bool {
        let mut gp = g.ptr();
    (*gp.gfx).init(g, fullscreen)
}

pub fn gfx_changefullscreen(g: &mut Game, fullscreen: bool) {
    let mut gp = g.ptr();
    gp.gfx.change_full_screen(g, fullscreen)
}

pub fn gfx_flipscreen(g: &mut Game) {
    let mut gp = g.ptr();
    gp.gfx.flip_screen(g)
}

pub fn gfx_settitle(g: &mut Game, title: &str) {
    let mut gp = g.ptr();
    gp.gfx.set_title(g, title)
}

/// Logs and shows a crash dialog that includes `error` when it is not empty.
pub fn gfx_show_catched_error(g: &mut Game, error: &str) {
    let mut gp = g.ptr();
    let mut message = String::from(
        "It seems the game has unexpectedly crashed. If you could tell us\n\
         what happened exactly, we might be able to fix this bug. Consider\n\
         reporting it on the link below, thanks!\n\n\
         https://github.com/KyleJamesWalker/supermariowar-rust/issues\n\n\
         Sincerely,\nThe Developers",
    );
    if !error.is_empty() {
        message += "\n\n\nThe error message:\n";
        message += error;
    }
    eprintln!("\n{}", message);
    gp.gfx.show_error_box(g, &message)
}

pub fn gfx_take_screenshot(g: &mut Game) {
        let mut gp = g.ptr();
    gp.gfx.take_screenshot(g)
}

pub fn gfx_close() {}

/// Loads the player color palette image; see [`gfxPalette::load`].
pub fn gfx_loadpalette(g: &mut Game, palette_path: &Path) -> bool {
    let mut gp = g.ptr();
    gp.gfx_palette.load(g, palette_path)
}

/// Writes `screen` as a BMP for the differential replay harness; returns whether it succeeded.
pub fn gfx_save_screen_bmp(g: &mut Game, path: &str) -> bool {
    unsafe { (*g.screen).save_bmp(Path::new(path)).is_ok() }
}

/// Appends the screen's 640x480 pixels to `out` as raw ARGB8888 (BGRA bytes on little-endian).
pub fn gfx_write_screen_raw(g: &mut Game, out: &mut impl std::io::Write) -> std::io::Result<()> {
    let screen = unsafe { &*g.screen };
    let bytes: Vec<u8> = screen.pixels.iter().flat_map(|&p| (if screen.alpha { p } else { p | 0xFF00_0000 }).to_le_bytes()).collect();
    out.write_all(&bytes)
}

fn load_skin(g: &mut Game, path: &Path) -> Result<gfxSprite, String> {
    let skin = ImageLoader::new(path).without_color_key().without_optimization().try_create(g)?;

    if !valid_skin_surface(&skin) {
        return Err(format!("Invalid skin file: {} has incorrect dimensions", path.display()));
    }

    Ok(skin)
}

/// Recolored menu frames of a skin for team `colorScheme`: frames 0 and 1 at strip indices 0 and 2, and their
/// mirrored copies at 1 and 3 when `fLoadBothDirections`. `Err` holds the load error.
pub fn gfx_loadmenuskin(g: &mut Game, path: &Path, colorScheme: i16, fLoadBothDirections: bool) -> Result<SpriteStrip, String> {
    let skin = load_skin(g, path)?;

    let mut strip = new_sprite_strip();

    for i in 0..2usize {
        let skinSurface = create_skin_surface(g, &skin, i, colorScheme as usize, true, false);
        strip[i * 2] = skinSurface;
    }

    if fLoadBothDirections {
        for i in 0..2usize {
            let skinSurface = create_skin_surface(g, &skin, i, colorScheme as usize, true, true);
            strip[i * 2 + 1] = skinSurface;
        }
    }

    Ok(strip)
}

/// All recolored frames of a skin for team `colorScheme`: frame `k` (0-3) at index `k * 2`, mirrored at `k * 2 + 1`,
/// then the dead-flying and dead-stomped frames at 8 and 9. `Err` holds the load error.
pub fn gfx_loadfullskin(g: &mut Game, path: &Path, colorScheme: i16) -> Result<SpriteStrip, String> {
    let skin = load_skin(g, path)?;

    let mut strip = new_sprite_strip();

    for k in 0..4usize {
        for j in 0..2usize {
            let skinSurface = create_skin_surface(g, &skin, k, colorScheme as usize, true, j != 0);
            strip[(k * 2) + j] = skinSurface;
        }
    }

    //Dead Flying Sprite
    let skinSurface = create_skin_surface(g, &skin, 4, colorScheme as usize, true, false);
    strip[8] = skinSurface;

    //Dead Stomped Sprite
    let skinSurface = create_skin_surface(g, &skin, 5, colorScheme as usize, true, false);
    strip[9] = skinSurface;

    Ok(strip)
}

/// Clips `dstRect` to `clipArea`, moving its origin and shrinking `srcRect` to match; a destination entirely
/// outside zeroes `srcRect`'s size. `dstRect`'s size is not updated.
pub fn gfx_cliprect(srcRect: &mut Rect, dstRect: &mut Rect, clipArea: &Rect) {
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
pub fn gfx_adjusthiddenrects(g: &mut Game, src: &mut Rect, dst: &mut Rect, edge: ClipEdge, threshold: i32) -> bool {
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
                dst.y = threshold + g.y_shake as i32;
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
                dst.x = threshold + g.x_shake as i32;
                dst.w = src.w;
            }
        }
    }

    false
}

/// Draws part of a half-scale map preview clipped to `clipRect`, wrapping across its sides (320 pixels) when `wrap`.
pub fn gfx_drawpreview(g: &mut Game, 
    sprite: &gfxSprite,
    dstX: i16,
    dstY: i16,
    srcX: i16,
    srcY: i16,
    iw: i16,
    ih: i16,
    clipRect: &Rect,
    wrap: bool,
    clip: Option<(ClipEdge, i32)>,
) {
    //need to set source rect before each blit so it can be clipped correctly
    let mut rSrcRect = Rect { x: srcX as i32, y: srcY as i32, w: iw as i32, h: ih as i32 };
    let mut rDstRect = Rect { x: dstX as i32, y: dstY as i32, w: iw as i32, h: ih as i32 };

    gfx_cliprect(&mut rSrcRect, &mut rDstRect, clipRect);

    if let Some((edge, threshold)) = clip {
        if gfx_adjusthiddenrects(g, &mut rSrcRect, &mut rDstRect, edge, threshold) {
            return;
        }
    }

    // Blit onto the screen surface
    sprite.draw_src_to(&rSrcRect, g.blitdest, &rDstRect);

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
                if gfx_adjusthiddenrects(g, &mut rSrcRect, &mut rDstRect, edge, threshold) {
                    return;
                }
            }

            sprite.draw_src_to(&rSrcRect, g.blitdest, &rDstRect);
        }
    }
}

/// Sets `player`'s controller LED to their team color scaled by `brightness` (clamped to 0-1).
pub fn gfx_setjoystickteamcolor(g: &mut Game, player: i32, team: i16, mut brightness: f32) {
    if team < 0 {
        // A non-playing user has no team
        return;
    }
    brightness = 0f32.max(1f32.min(brightness));
    if let Some(color) = g.gfx_palette.replacement_for(RGB { r: 0x80, g: 0x00, b: 0x00 }, team as usize, gfx_palette::normal) {
        let rgb = [(brightness * color.r as f32) as u8, (brightness * color.g as f32) as u8, (brightness * color.b as f32) as u8];
        crate::services::services(g).pads.set_led(player, rgb);
    }
}
