//! Bitmap fonts loaded from a PNG strip of glyphs. Text is drawn byte by byte; bytes below `!`, including every byte
//! >= 0x80 (read as signed), advance by the width of the first glyph.

use crate::common::gfx::soft_surface::SurfacePtr;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::{blit, soft_image};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::globals::*;
use std::io::Write;
use std::path::Path;

/// Character code of the font image's first glyph (`!`).
const ASCII_FIRST_PRINTABLE: i32 = 33;

/// A glyph's column span in the font image, in pixels.
#[derive(Clone, Copy, Default)]
struct GlyphArea {
    x: i32,
    w: i32,
}

/// A bitmap font: row 0 of the image separates glyphs with magenta markers, rows below hold the glyphs, and
/// the pixel at (0, 1) is the color key.
#[derive(Default)]
pub struct gfxFont {
    m_sprite: gfxSprite,
    m_glyph_areas: Vec<GlyphArea>,
    pub _alias: Aliased,
}

fn throw(what: String) -> ! {
    std::panic::panic_any(what)
}

impl gfxFont {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a font image; panics with a `String` payload when it cannot be read or holds no glyphs.
    pub fn from_path(g: &mut Game, path: &str) -> Self {
        let path_str = Path::new(path).to_string_lossy().into_owned();

        print!("loading font {} ...", path_str);
        let _ = std::io::stdout().flush();
        match blit::decode_for_load(Path::new(path)) {
            Some(image) => Self::from_decoded(&path_str, &image),
            None => throw(format!("Couldn't load {}: not a readable PNG", path_str)),
        }
    }

    fn from_decoded(path_str: &str, image: &soft_image::Decoded) -> Self {
        let (width, _) = image.size();
        let raw_magenta = image.map_rgb(255, 0, 255);
        let first_row: Vec<u32> = (0..width).map(|x| image.raw_pixel(x, 0)).collect();
        let m_glyph_areas = glyph_areas(&first_row, raw_magenta);
        if m_glyph_areas.is_empty() {
            throw(format!("Didn't find any characters on font image {}", path_str));
        }
        let surface = soft_image::convert_to_screen(image, Some(image.raw_pixel(0, 1)));
        let surf_opti = SurfacePtr::new(surface);
        let m_sprite = gfxSprite::from_surface(surf_opti, None);
        println!("done");
        gfxFont { m_sprite, m_glyph_areas, _alias: Aliased::new() }
    }

    /// Draws `text` with its left edge at `x` and top at `y`, offset by camera shake.
    pub fn draw(&self, g: &mut Game, x: i32, y: i32, text: &str) {
        self.draw_range(g, x, y, text, i32::MIN, i32::MAX);
    }

    /// Draws `text` at (`x`, `y`), skipping glyphs that end left of `dst_min_x` and stopping past `dst_max_x`.
    pub fn draw_range(&self, g: &mut Game, mut x: i32, y: i32, text: &str, dst_min_x: i32, dst_max_x: i32) {
        for &b in text.as_bytes() {
            if dst_max_x < x {
                break;
            }

            let ch = b as i8 as i32;
            if ch < ASCII_FIRST_PRINTABLE {
                x += self.m_glyph_areas[0].w;
                continue;
            }

            let idx = (ch - ASCII_FIRST_PRINTABLE) as usize;
            if idx >= self.m_glyph_areas.len() {
                continue;
            }

            let src = Rect {
                x: self.m_glyph_areas[idx].x,
                y: 1,
                w: self.m_glyph_areas[idx].w,
                h: self.m_sprite.get_height() - 1,
            };
            if dst_min_x <= x + src.w {
                self.m_sprite.draw_src(g, x, y, &src);
            }

            x += self.m_glyph_areas[idx].w;
        }
    }

    /// Draws `text` centered on `x`.
    pub fn draw_centered(&self, g: &mut Game, x: i32, y: i32, text: &str) {
        let mut gp = g.ptr();
        self.draw(gp.get(), x - self.get_width(text) / 2, y, text);
    }

    /// Draws `text` centered on `x`, cut to the `width` pixels around it.
    pub fn draw_chop_centered(&self, g: &mut Game, x: i32, y: i32, mut width: i32, text: &str) {
        width /= 2;
        self.draw_range(g, x - self.get_width(text) / 2, y, text, x - width, x + width);
    }

    /// Draws `text` ending at `x`.
    pub fn draw_right_justified(&self, g: &mut Game, x: i32, y: i32, text: &str) {
        let mut gp = g.ptr();
        self.draw(gp.get(), x - self.get_width(text), y, text);
    }

    /// Draws `text` from `x`, cut at `width` pixels to the right.
    pub fn draw_chop_right(&self, g: &mut Game, x: i32, y: i32, width: i32, text: &str) {
        self.draw_range(g, x, y, text, x, x + width);
    }

    /// Draws `text` right-justified at `x`, cut at `width` pixels to the left.
    pub fn draw_chop_left(&self, g: &mut Game, x: i32, y: i32, width: i32, text: &str) {
        self.draw_range(g, x - self.get_width(text), y, text, x - width, x);
    }

    pub fn set_alpha(&mut self, alpha: u8) {
        let surface = self.m_sprite.get_surface();
        unsafe {
            (*surface).blend = crate::common::gfx::soft_surface::BlendMode::Blend;
            (*surface).alpha_mod = alpha;
        }
    }

    /// Glyph height in pixels, excluding the marker row.
    pub fn get_height(&self) -> i32 {
        self.m_sprite.get_height() - 1
    }

    /// Width of `text` in pixels.
    pub fn get_width(&self, text: &str) -> i32 {
        let mut width = 0;
        for &b in text.as_bytes() {
            let ch = b as i8 as i32;
            let mut idx: usize = 0;
            if ch >= ASCII_FIRST_PRINTABLE {
                idx = (ch - ASCII_FIRST_PRINTABLE) as usize;
            }
            if idx < self.m_glyph_areas.len() {
                width += self.m_glyph_areas[idx].w;
            }
        }
        width
    }
}

/// The glyph columns between the magenta markers in a font image's first row.
fn glyph_areas(first_row: &[u32], raw_magenta: u32) -> Vec<GlyphArea> {
    let mut m_glyph_areas: Vec<GlyphArea> = Vec::with_capacity(127);
    let width = first_row.len() as i32;
    let mut x: i32 = 0;

    // Find first marker
    while x < width && first_row[x as usize] != raw_magenta {
        x += 1;
    }

    while x < width {
        // Find marker end
        while x < width && first_row[x as usize] == raw_magenta {
            x += 1;
        }

        if width <= x {
            break;
        }

        let start = x;

        // Grow until next marker
        while x < width && first_row[x as usize] != raw_magenta {
            x += 1;
        }

        m_glyph_areas.push(GlyphArea { x: start, w: x - start });
    }

    m_glyph_areas.shrink_to_fit();
    m_glyph_areas
}
