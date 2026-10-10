//! PNG decoding into the surface layouts SDL_image returns, and their conversion to the screen format.

use crate::common::gfx::soft_surface::{convert_indexed, BlendMode, Surface};

/// A decoded image in the pixel layout `IMG_Load` gives it.
#[derive(Clone, Debug)]
pub enum Decoded {
    /// `SDL_PIXELFORMAT_INDEX8`: one index per pixel and the PLTE colors (opaque).
    Indexed { w: i32, h: i32, palette: Vec<[u8; 4]>, indices: Vec<u8> },
    /// `SDL_PIXELFORMAT_RGB24`: three bytes per pixel.
    Rgb { w: i32, h: i32, bytes: Vec<u8> },
    /// `SDL_PIXELFORMAT_RGBA32`: four bytes per pixel, straight alpha.
    Rgba { w: i32, h: i32, bytes: Vec<u8> },
}

impl Decoded {
    pub fn size(&self) -> (i32, i32) {
        match self {
            Decoded::Indexed { w, h, .. } | Decoded::Rgb { w, h, .. } | Decoded::Rgba { w, h, .. } => (*w, *h),
        }
    }
}

/// Decodes 8-bit RGB, 8-bit RGBA and 1-8 bit palette PNGs without a tRNS chunk. Other PNGs return `None`.
pub fn decode_png(data: &[u8]) -> Option<Decoded> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if info.trns.is_some() || info.interlaced {
        return None;
    }
    let (w, h) = (info.width as i32, info.height as i32);
    let depth = info.bit_depth as u8;
    let color = info.color_type;
    let palette: Option<Vec<[u8; 4]>> = info.palette.as_ref().map(|p| p.chunks_exact(3).map(|c| [c[0], c[1], c[2], 255]).collect());

    let mut buf = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buf).ok()?;
    let stride = frame.line_size;

    match (color, depth) {
        (png::ColorType::Rgb, 8) => {
            let mut bytes = Vec::with_capacity((w * h * 3) as usize);
            for y in 0..h as usize {
                bytes.extend_from_slice(&buf[y * stride..y * stride + w as usize * 3]);
            }
            Some(Decoded::Rgb { w, h, bytes })
        }
        (png::ColorType::Rgba, 8) => {
            let mut bytes = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h as usize {
                bytes.extend_from_slice(&buf[y * stride..y * stride + w as usize * 4]);
            }
            Some(Decoded::Rgba { w, h, bytes })
        }
        (png::ColorType::Indexed, 1 | 2 | 4 | 8) => {
            let per_byte = 8 / depth as usize;
            let mask = (1u16 << depth) as u8 - 1;
            let mut indices = Vec::with_capacity((w * h) as usize);
            for y in 0..h as usize {
                let row = &buf[y * stride..(y + 1) * stride];
                for x in 0..w as usize {
                    let byte = row[x / per_byte];
                    let shift = 8 - depth as usize * (x % per_byte + 1);
                    indices.push((byte >> shift) & mask);
                }
            }
            Some(Decoded::Indexed { w, h, palette: palette?, indices })
        }
        _ => None,
    }
}

/// SDL3's `SDL_FindColor`: the palette entry nearest to the color (squared distance, the first on ties).
fn find_color(palette: &[[u8; 4]], r: u8, g: u8, b: u8, a: u8) -> u8 {
    let mut best = 0;
    let mut smallest = u32::MAX;
    for (i, c) in palette.iter().enumerate() {
        let d = |x: u8, y: u8| (x as i32 - y as i32).pow(2) as u32;
        let distance = d(c[0], r) + d(c[1], g) + d(c[2], b) + d(c[3], a);
        if distance < smallest {
            best = i as u8;
            if distance == 0 {
                break;
            }
            smallest = distance;
        }
    }
    best
}

/// `IMG_Load`, then `SDL_SetColorKey(SDL_MapRGB(key))` when a key is given, then `SDL_ConvertSurface` to ARGB8888.
/// Key pixels become alpha 0, and the result blends without a color key.
pub fn convert_to_screen(image: &Decoded, key: Option<(u8, u8, u8)>) -> Surface {
    let mut out = match image {
        Decoded::Indexed { w, h, palette, indices } => {
            let key_index = key.map(|(r, g, b)| find_color(palette, r, g, b, 255));
            return convert_indexed(*w, *h, *w as usize, indices, palette, key_index);
        }
        Decoded::Rgb { w, h, bytes } => {
            let mut s = Surface::new(*w, *h);
            for (p, c) in s.pixels.iter_mut().zip(bytes.chunks_exact(3)) {
                *p = 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
            }
            s
        }
        Decoded::Rgba { w, h, bytes } => {
            let mut s = Surface::new(*w, *h);
            for (p, c) in s.pixels.iter_mut().zip(bytes.chunks_exact(4)) {
                *p = (c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
            }
            s
        }
    };
    if let Some((r, g, b)) = key {
        let rgb = (r as u32) << 16 | (g as u32) << 8 | b as u32;
        for p in &mut out.pixels {
            if *p & 0x00FF_FFFF == rgb {
                *p &= 0x00FF_FFFF;
            }
        }
    }
    out.blend = BlendMode::Blend;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::sys::image::IMG_Load;
    use sdl2::sys::*;
    use std::ffi::CString;
    use std::path::{Path, PathBuf};

    fn sdl_pixels(surf: *mut SDL_Surface) -> Vec<u32> {
        unsafe {
            let mut out = Vec::new();
            for y in 0..(*surf).h {
                let row = ((*surf).pixels as *const u8).add((y * (*surf).pitch) as usize) as *const u32;
                for x in 0..(*surf).w {
                    out.push(*row.add(x as usize));
                }
            }
            out
        }
    }

    fn pngs(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_dir() {
                pngs(&p, out);
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
                out.push(p);
            }
        }
    }

    /// Every PNG in `data/`, with and without the magenta key the game uses, against SDL_image and SDL.
    #[test]
    fn data_images_match_sdl() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut files = Vec::new();
        pngs(&data, &mut files);
        if files.is_empty() {
            eprintln!("no data/ checkout; skipping");
            return;
        }
        files.sort();
        let mut decoded = 0;
        for path in &files {
            let bytes = std::fs::read(path).unwrap();
            let Some(image) = decode_png(&bytes) else { continue };
            decoded += 1;
            for key in [Option::None, Some((255, 0, 255))] {
                unsafe {
                    let c = CString::new(path.to_str().unwrap()).unwrap();
                    let raw = IMG_Load(c.as_ptr());
                    assert!(!raw.is_null(), "IMG_Load {}", path.display());
                    if let Some((r, g, b)) = key {
                        SDL_SetColorKey(raw, 1, SDL_MapRGB((*raw).format, r, g, b));
                    }
                    let fmt = SDL_AllocFormat(SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32);
                    let conv = SDL_ConvertSurface(raw, fmt, 0);
                    let ours = convert_to_screen(&image, key);
                    assert_eq!((ours.w, ours.h), ((*conv).w, (*conv).h), "{}", path.display());
                    let theirs = sdl_pixels(conv);
                    if let Some(i) = ours.pixels.iter().zip(&theirs).position(|(a, b)| a != b) {
                        panic!(
                            "{} key {key:?}: pixel {i}: rust {:08x}, sdl {:08x}",
                            path.display(),
                            ours.pixels[i],
                            theirs[i]
                        );
                    }
                    let mut k = 0;
                    assert_eq!(SDL_GetColorKey(conv, &mut k), -1, "{} keeps no color key", path.display());
                    SDL_FreeFormat(fmt);
                    SDL_FreeSurface(conv);
                    SDL_FreeSurface(raw);
                }
            }
        }
        eprintln!("{decoded} of {} PNGs decoded and compared", files.len());
        assert_eq!(decoded, files.len(), "every data/ PNG decodes");
    }
}
