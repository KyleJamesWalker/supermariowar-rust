//! Port of src/common/gfx/SFont.cpp
//!
//! Text is taken as bytes and read up to the first NUL, like the C strings. `char` is signed on
//! the reference platforms, so bytes >= 0x80 give a negative `charoffset` and draw as spaces.

use crate::globals::*;
use sdl2::sys::{
    SDL_FreeSurface, SDL_LockSurface, SDL_MapRGB, SDL_Rect, SDL_SetColorKey, SDL_Surface,
    SDL_UnlockSurface, SDL_UpperBlit,
};

pub struct SFont_Font {
    pub Surface: *mut SDL_Surface,
    pub CharPos: [i32; 512],
    pub MaxPos: i32,
}

fn c_str(text: &[u8]) -> &[u8] {
    match text.iter().position(|&c| c == 0) {
        Some(n) => &text[..n],
        None => text,
    }
}

unsafe fn GetPixel(Surface: *mut SDL_Surface, X: i32, Y: i32) -> u32 {
    assert!(X >= 0);
    assert!(X < (*Surface).w);

    let fmt = (*Surface).format;
    let Bpp = (*fmt).BytesPerPixel as i32;
    let pitch = (*Surface).pitch;
    let pixels = (*Surface).pixels as *mut u8;
    let bits = pixels.offset((Y * pitch + X * Bpp) as isize);

    match Bpp {
        1 => *pixels.offset((Y * pitch + X) as isize) as u32,
        2 => *(pixels as *mut u16).offset((Y * pitch / 2 + X) as isize) as u32,
        3 => {
            let r = *bits.offset(((*fmt).Rshift / 8) as isize);
            let g = *bits.offset(((*fmt).Gshift / 8) as isize);
            let b = *bits.offset(((*fmt).Bshift / 8) as isize);
            SDL_MapRGB(fmt, r, g, b)
        }
        4 => *(pixels as *mut u32).offset((Y * pitch / 4 + X) as isize),
        _ => 0,
    }
}

pub fn SFont_InitFont(Surface: *mut SDL_Surface) -> Option<Box<SFont_Font>> {
    let mut x: i32 = 0;
    let mut i: usize = 0;

    if Surface.is_null() {
        return None;
    }

    unsafe {
        let mut Font = Box::new(SFont_Font { Surface, CharPos: [0; 512], MaxPos: 0 });

        SDL_LockSurface(Surface);

        let pink = SDL_MapRGB((*Surface).format, 255, 0, 255);
        while x < (*Surface).w {
            if GetPixel(Surface, x, 0) == pink {
                Font.CharPos[i] = x;
                i += 1;
                while (x < (*Surface).w) && (GetPixel(Surface, x, 0) == pink) {
                    x += 1;
                }
                Font.CharPos[i] = x;
                i += 1;
            }
            x += 1;
        }
        Font.MaxPos = x - 1;

        let pixel = GetPixel(Surface, 0, (*Surface).h - 1);
        SDL_UnlockSurface(Surface);
        SDL_SetColorKey(Surface, 1, pixel);

        Some(Font)
    }
}

pub fn SFont_FreeFont(FontInfo: Box<SFont_Font>) {
    unsafe { SDL_FreeSurface(FontInfo.Surface) };
}

#[inline]
fn char_offset(c: u8) -> i32 {
    ((c as i8) as i32 - 33) * 2 + 1
}

#[inline]
fn is_blank(Font: &SFont_Font, c: u8, charoffset: i32) -> bool {
    c == b' ' || charoffset < 0 || charoffset > Font.MaxPos
}

#[inline]
fn cp(Font: &SFont_Font, idx: i32) -> i32 {
    Font.CharPos[idx as usize]
}

#[inline]
unsafe fn glyph_rects(Font: &SFont_Font, charoffset: i32, x: i32, y: i32, srcrect: &mut SDL_Rect, dstrect: &mut SDL_Rect) {
    let w = (((cp(Font, charoffset + 2) + cp(Font, charoffset + 1)) >> 1)
        - ((cp(Font, charoffset) + cp(Font, charoffset - 1)) >> 1)) as i16 as i32;
    srcrect.w = w;
    dstrect.w = w;

    srcrect.x = ((cp(Font, charoffset) + cp(Font, charoffset - 1)) as i16 >> 1) as i32;

    dstrect.x = (x as f32 - ((cp(Font, charoffset) - cp(Font, charoffset - 1)) >> 1) as f32) as i16 as i32
        + x_shake as i32;

    dstrect.y = (y + y_shake as i32) as i16 as i32;
}

#[inline]
unsafe fn fixed_rects(Font: &SFont_Font) -> (SDL_Rect, SDL_Rect) {
    let h = ((*Font.Surface).h as u16) as i32 - 1;
    (SDL_Rect { x: 0, y: 1, w: 0, h }, SDL_Rect { x: 0, y: 0, w: 0, h })
}

pub fn SFont_Write(Surface: *mut SDL_Surface, Font: &SFont_Font, mut x: i32, y: i32, text: &[u8]) {
    let text = c_str(text);
    unsafe {
        // these values won't change in the loop
        let (mut srcrect, mut dstrect) = fixed_rects(Font);

        for &c in text {
            if !(x <= (*Surface).w) {
                break;
            }
            let charoffset = char_offset(c);
            // skip spaces and nonprintable characters
            if is_blank(Font, c, charoffset) {
                x += cp(Font, 2) - cp(Font, 1);
                continue;
            }

            glyph_rects(Font, charoffset, x, y, &mut srcrect, &mut dstrect);

            SDL_UpperBlit(Font.Surface, &srcrect, Surface, &mut dstrect);

            x += cp(Font, charoffset + 1) - cp(Font, charoffset);
        }
    }
}

pub fn SFont_TextWidth(Font: &SFont_Font, text: &[u8]) -> i32 {
    let text = c_str(text);
    let mut width = 0;

    for &c in text {
        let charoffset = char_offset(c);

        // skip spaces and nonprintable characters
        if is_blank(Font, c, charoffset) {
            width += cp(Font, 2) - cp(Font, 1);
            continue;
        }

        width += cp(Font, charoffset + 1) - cp(Font, charoffset);
    }

    width
}

pub fn SFont_TextHeight(Font: &SFont_Font) -> i32 {
    unsafe { (*Font.Surface).h - 1 }
}

pub fn SFont_WriteCenter(Surface: *mut SDL_Surface, Font: &SFont_Font, x: i32, y: i32, text: &[u8]) {
    SFont_Write(Surface, Font, x - (SFont_TextWidth(Font, text) >> 1), y, text);
}

pub fn SFont_WriteChopCenter(Surface: *mut SDL_Surface, Font: &SFont_Font, x: i32, y: i32, w: i32, text: &[u8]) {
    let text = c_str(text);
    let mut iCurrentWidth = 0;
    let mut iCurrentChar: usize = 0;

    //array overflow safe
    let mut szText: Vec<u8> = text.iter().copied().take(255).collect();

    for &c in text {
        let charoffset = char_offset(text[iCurrentChar]);

        let iNextWidth = if is_blank(Font, c, charoffset) {
            cp(Font, 2) - cp(Font, 1)
        } else {
            cp(Font, charoffset + 1) - cp(Font, charoffset)
        };

        if iCurrentWidth + iNextWidth > w {
            szText.truncate(iCurrentChar);
            break;
        }

        iCurrentChar += 1;
        iCurrentWidth += iNextWidth;
    }

    SFont_WriteCenter(Surface, Font, x, y, &szText);
}

//Right Aligned
pub fn SFont_WriteRight(Surface: *mut SDL_Surface, Font: &SFont_Font, x: i32, y: i32, text: &[u8]) {
    SFont_Write(Surface, Font, x - SFont_TextWidth(Font, text), y, text);
}

//Left aligned with fixed width
pub fn SFont_WriteChopRight(Surface: *mut SDL_Surface, Font: &SFont_Font, mut x: i32, y: i32, w: i32, text: &[u8]) {
    let text = c_str(text);
    let startx = x;

    unsafe {
        let (mut srcrect, mut dstrect) = fixed_rects(Font);

        for &c in text {
            if !(x <= (*Surface).w) {
                break;
            }
            let charoffset = char_offset(c);
            // skip spaces and nonprintable characters
            if is_blank(Font, c, charoffset) {
                x += cp(Font, 2) - cp(Font, 1);
                continue;
            }

            let gw = (((cp(Font, charoffset + 2) + cp(Font, charoffset + 1)) >> 1)
                - ((cp(Font, charoffset) + cp(Font, charoffset - 1)) >> 1)) as i16 as i32;
            srcrect.w = gw;
            dstrect.w = gw;

            let width = cp(Font, charoffset + 1) - cp(Font, charoffset);

            if x - startx + width > w {
                break;
            }

            glyph_rects(Font, charoffset, x, y, &mut srcrect, &mut dstrect);

            SDL_UpperBlit(Font.Surface, &srcrect, Surface, &mut dstrect);

            x += width;
        }
    }
}

//Right aligned with fixed width
pub fn SFont_WriteChopLeft(Surface: *mut SDL_Surface, Font: &SFont_Font, mut x: i32, y: i32, w: i32, text: &[u8]) {
    let text = c_str(text);
    let mut iPrintedWidth: i16 = 0;
    let startx = x;

    unsafe {
        let (mut srcrect, mut dstrect) = fixed_rects(Font);

        for &c in text.iter().rev() {
            if !(x >= startx - w) {
                break;
            }
            let charoffset = char_offset(c);
            // skip spaces and nonprintable characters
            if is_blank(Font, c, charoffset) {
                let width = cp(Font, 2) - cp(Font, 1);
                x -= width;

                iPrintedWidth = (iPrintedWidth as i32 + width) as i16;
                continue;
            }

            let width = cp(Font, charoffset + 1) - cp(Font, charoffset);

            iPrintedWidth = (iPrintedWidth as i32 + width) as i16;

            if iPrintedWidth as i32 > w {
                break;
            }

            x -= width;

            glyph_rects(Font, charoffset, x, y, &mut srcrect, &mut dstrect);

            SDL_UpperBlit(Font.Surface, &srcrect, Surface, &mut dstrect);
        }
    }
}
