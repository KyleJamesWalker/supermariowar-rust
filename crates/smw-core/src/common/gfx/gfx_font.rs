//! Port of src/common/gfx/gfxFont.cpp
//!
//! `std::string_view` text is taken as bytes; `char` is signed on the reference platforms, so
//! bytes >= 0x80 are below `ASCII_FIRST_PRINTABLE` and advance like a space.

use crate::common::gfx::{get_raw_pixel, rle_enabled};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::util::sdl_helpers::SdlSurfacePtr;
use crate::globals::*;
use sdl2::sys::image::IMG_Load;
use sdl2::sys::{
    SDL_BlendMode, SDL_ConvertSurface, SDL_GetError, SDL_LockSurface, SDL_MapRGB, SDL_Rect, SDL_SetColorKey,
    SDL_SetSurfaceAlphaMod, SDL_SetSurfaceBlendMode, SDL_SetSurfaceRLE, SDL_UnlockSurface,
};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::Path;

const ASCII_FIRST_PRINTABLE: i32 = 33;
const SDL_RLEACCEL: u32 = 0x00000002;

#[derive(Clone, Copy, Default)]
struct GlyphArea {
    x: i32,
    w: i32,
}

#[derive(Default)]
pub struct gfxFont {
    m_sprite: gfxSprite,
    m_glyph_areas: Vec<GlyphArea>,
    pub _alias: Aliased,
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn throw(what: String) -> ! {
    std::panic::panic_any(what)
}

impl gfxFont {
    pub fn new() -> Self {
        Self::default()
    }

    /// `gfxFont(const std::filesystem::path& path)`; throws `std::string` on failure.
    pub fn from_path(path: &str) -> Self {
        let path_str = Path::new(path).to_string_lossy().into_owned();

        print!("loading font {} ...", path_str);
        let _ = std::io::stdout().flush();
        let cpath = CString::new(path_str.as_str()).unwrap();
        let surf = SdlSurfacePtr::new(unsafe { IMG_Load(cpath.as_ptr()) });
        if surf.is_null() {
            throw(format!("Couldn't load {}: {}", path_str, sdl_error()));
        }

        unsafe {
            let s = surf.get();
            let must_lock = (*s).flags & SDL_RLEACCEL != 0;
            if must_lock {
                SDL_LockSurface(s);
            }

            let mut first_row: Vec<u32> = Vec::with_capacity((*s).w as usize);
            for x in 0..(*s).w {
                first_row.push(get_raw_pixel(s, x, 0));
            }

            if must_lock {
                SDL_UnlockSurface(s);
            }

            let mut m_glyph_areas: Vec<GlyphArea> = Vec::with_capacity(127);

            let raw_magenta = SDL_MapRGB((*s).format, 255, 0, 255);
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

            if m_glyph_areas.is_empty() {
                throw(format!("Didn't find any characters on font image {}", path_str));
            }

            m_glyph_areas.shrink_to_fit();

            let color_key = get_raw_pixel(s, 0, 1);
            if SDL_SetColorKey(s, 1, color_key) < 0 {
                throw(format!("Couldn't set color key on font image: {}", sdl_error()));
            }

            let surf_opti = SdlSurfacePtr::new(SDL_ConvertSurface(s, (*screen).format, 0));
            if surf_opti.is_null() {
                throw(format!("Couldn't convert {} to the display's pixel format: {}", path_str, sdl_error()));
            }

            if rle_enabled() && SDL_SetSurfaceRLE(surf_opti.get(), 1) < 0 {
                throw(format!("Couldn't set RLE acceleration for {}: {}", path_str, sdl_error()));
            }

            let m_sprite = gfxSprite::from_surface(surf_opti, None);
            println!("done");
            gfxFont { m_sprite, m_glyph_areas, _alias: Aliased::new() }
        }
    }

    pub fn draw(&self, x: i32, y: i32, text: &str) {
        self.draw_range(x, y, text, i32::MIN, i32::MAX);
    }

    /// `draw(x, y, text, dst_min_x, dst_max_x)`
    pub fn draw_range(&self, mut x: i32, y: i32, text: &str, dst_min_x: i32, dst_max_x: i32) {
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

            let src = SDL_Rect {
                x: self.m_glyph_areas[idx].x,
                y: 1,
                w: self.m_glyph_areas[idx].w,
                h: self.m_sprite.get_height() - 1,
            };
            if dst_min_x <= x + src.w {
                self.m_sprite.draw_src(x, y, &src);
            }

            x += self.m_glyph_areas[idx].w;
        }
    }

    pub fn draw_centered(&self, x: i32, y: i32, text: &str) {
        self.draw(x - self.get_width(text) / 2, y, text);
    }

    pub fn draw_chop_centered(&self, x: i32, y: i32, mut width: i32, text: &str) {
        width /= 2;
        self.draw_range(x - self.get_width(text) / 2, y, text, x - width, x + width);
    }

    pub fn draw_right_justified(&self, x: i32, y: i32, text: &str) {
        self.draw(x - self.get_width(text), y, text);
    }

    pub fn draw_chop_right(&self, x: i32, y: i32, width: i32, text: &str) {
        self.draw_range(x, y, text, x, x + width);
    }

    pub fn draw_chop_left(&self, x: i32, y: i32, width: i32, text: &str) {
        self.draw_range(x - self.get_width(text), y, text, x - width, x);
    }

    pub fn set_alpha(&mut self, alpha: u8) {
        let surface = self.m_sprite.get_surface();
        unsafe {
            if SDL_SetSurfaceBlendMode(surface, SDL_BlendMode::SDL_BLENDMODE_BLEND) < 0 {
                eprintln!("\n ERROR: couldn't set blend mode on font surface: {}", sdl_error());
                return;
            }
            if SDL_SetSurfaceAlphaMod(surface, alpha) < 0 {
                eprintln!("\n ERROR: couldn't set alpha on font surface: {}", sdl_error());
                return;
            }
        }
    }

    pub fn get_height(&self) -> i32 {
        self.m_sprite.get_height() - 1
    }

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
