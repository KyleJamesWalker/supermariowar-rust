//! Port of src/common/gfx/gfxFont.cpp
//!
//! The printf-style `drawf` / `drawRightJustified` take the already formatted text; callers
//! format with `format!` using the same conversions as the C++ format string.

use crate::common::gfx::s_font::*;
use crate::globals::*;
use sdl2::sys::image::IMG_Load;
use sdl2::sys::{SDL_BlendMode, SDL_GetError, SDL_SetSurfaceAlphaMod, SDL_SetSurfaceBlendMode};
use std::ffi::{CStr, CString};
use std::io::Write;

#[derive(Default)]
pub struct gfxFont {
    m_font: Option<Box<SFont_Font>>,
    pub _alias: Aliased,
}

impl Drop for gfxFont {
    fn drop(&mut self) {
        if let Some(font) = self.m_font.take() {
            SFont_FreeFont(font);
        }
    }
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

impl gfxFont {
    pub fn new() -> Self {
        gfxFont { _alias: Aliased::new(), m_font: None }
    }

    pub fn init(&mut self, filename: &str) -> bool {
        if let Some(font) = self.m_font.take() {
            SFont_FreeFont(font);
        }

        print!("loading font {} ... ", filename);
        let _ = std::io::stdout().flush();

        let cpath = CString::new(filename).unwrap();
        let fontsurf = unsafe { IMG_Load(cpath.as_ptr()) };
        if fontsurf.is_null() {
            println!();
            println!(" ERROR: Couldn't load file {}: {}", filename, sdl_error());
            return false;
        }

        self.m_font = SFont_InitFont(fontsurf);
        if self.m_font.is_none() {
            println!();
            println!(" ERROR: an error occurre while loading the font.");
            return false;
        }

        println!("done");
        true
    }

    fn font(&self) -> &SFont_Font {
        self.m_font.as_deref().expect("gfxFont used before init")
    }

    pub fn draw(&self, x: i32, y: i32, s: &str) {
        unsafe { SFont_Write(blitdest, self.font(), x, y, s.as_bytes()) };
    }

    pub fn drawf(&self, x: i32, y: i32, s: &str) {
        self.draw(x, y, s);
    }

    pub fn draw_centered(&self, x: i32, y: i32, text: &str) {
        unsafe { SFont_WriteCenter(blitdest, self.font(), x, y, text.as_bytes()) };
    }

    pub fn draw_chop_centered(&self, x: i32, y: i32, width: i32, text: &str) {
        unsafe { SFont_WriteChopCenter(blitdest, self.font(), x, y, width, text.as_bytes()) };
    }

    pub fn draw_right_justified(&self, x: i32, y: i32, s: &str) {
        unsafe { SFont_WriteRight(blitdest, self.font(), x, y, s.as_bytes()) };
    }

    pub fn draw_chop_right(&self, x: i32, y: i32, width: i32, s: &str) {
        unsafe { SFont_WriteChopRight(blitdest, self.font(), x, y, width, s.as_bytes()) };
    }

    pub fn draw_chop_left(&self, x: i32, y: i32, width: i32, s: &str) {
        unsafe { SFont_WriteChopLeft(blitdest, self.font(), x, y, width, s.as_bytes()) };
    }

    pub fn setalpha(&mut self, alpha: u8) {
        let surface = self.font().Surface;
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
        SFont_TextHeight(self.font())
    }

    pub fn get_width(&self, text: &str) -> i32 {
        SFont_TextWidth(self.font(), text.as_bytes())
    }
}
