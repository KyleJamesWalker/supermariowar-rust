//! Software ARGB8888 surfaces and the SDL3 blit paths the game uses.

/// `SDL_Rect`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    /// `SDL_IntersectRect`; `None` when the intersection is empty.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        if self.w <= 0 || self.h <= 0 || other.w <= 0 || other.h <= 0 {
            return None;
        }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = (self.x + self.w).min(other.x + other.w) - x;
        let h = (self.y + self.h).min(other.y + other.h) - y;
        if w <= 0 || h <= 0 {
            return None;
        }
        Some(Rect { x, y, w, h })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    None,
    Blend,
}

/// A 32-bit ARGB8888 surface: the screen format, which every loaded image is converted to.
#[derive(Clone, Debug)]
pub struct Surface {
    pub w: i32,
    pub h: i32,
    pub pixels: Vec<u32>,
    pub clip: Rect,
    pub color_key: Option<u32>,
    pub alpha_mod: u8,
    pub blend: BlendMode,
}

const RGB_MASK: u32 = 0x00FF_FFFF;

/// SDL3's `MULT_DIV_255`.
#[inline]
fn mult_div_255(a: u32, b: u32) -> u32 {
    let mut x = (a * b) as u16;
    x = x.wrapping_add(1);
    x = x.wrapping_add(x >> 8);
    (x >> 8) as u32
}

#[inline]
fn channels(p: u32) -> [u32; 4] {
    [(p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF, p >> 24]
}

#[inline]
fn pack(r: u32, g: u32, b: u32, a: u32) -> u32 {
    (a << 24) | (r << 16) | (g << 8) | b
}

/// `FACTOR_BLEND_8888`: `dst = (src * factor + dst * (255 - factor)) / 255`, rounded, per channel.
#[inline]
fn factor_blend(src: u32, dst: u32, factor: u32) -> u32 {
    let mut out = 0;
    for shift in [0, 8, 16, 24] {
        let s = (src >> shift) & 0xFF;
        let d = (dst >> shift) & 0xFF;
        let mut v = s * factor + d * (255 - factor);
        v += 1;
        v += v >> 8;
        out |= ((v >> 8) & 0xFF) << shift;
    }
    out
}

/// `Blit8888to8888PixelAlpha` (blend, no alpha modulation, no color key).
#[inline]
fn blend_pixel_alpha(src: u32, dst: u32) -> u32 {
    factor_blend(src | 0xFF00_0000, dst, src >> 24)
}

/// `SDL_Blit_ARGB8888_ARGB8888_Modulate_Blend` (blend with alpha modulation, no color key).
#[inline]
fn blend_modulate(src: u32, dst: u32, alpha_mod: u32) -> u32 {
    let [mut sr, mut sg, mut sb, mut sa] = channels(src);
    let [dr, dg, db, da] = channels(dst);
    sa = mult_div_255(sa, alpha_mod);
    if sa < 255 {
        sr = mult_div_255(sr, sa);
        sg = mult_div_255(sg, sa);
        sb = mult_div_255(sb, sa);
    }
    pack(
        mult_div_255(255 - sa, dr) + sr,
        mult_div_255(255 - sa, dg) + sg,
        mult_div_255(255 - sa, db) + sb,
        mult_div_255(255 - sa, da) + sa,
    )
}

/// `SDL_Blit_Slow`'s blend (color key with blending), which divides with truncation.
#[inline]
fn blend_slow(src: u32, dst: u32, alpha_mod: Option<u32>) -> u32 {
    let [mut sr, mut sg, mut sb, mut sa] = channels(src);
    let [dr, dg, db, da] = channels(dst);
    if let Some(m) = alpha_mod {
        sa = sa * m / 255;
    }
    if sa < 255 {
        sr = sr * sa / 255;
        sg = sg * sa / 255;
        sb = sb * sa / 255;
    }
    pack(
        sr + (255 - sa) * dr / 255,
        sg + (255 - sa) * dg / 255,
        sb + (255 - sa) * db / 255,
        sa + (255 - sa) * da / 255,
    )
}

impl Surface {
    /// `SDL_CreateRGBSurfaceWithFormat(.., ARGB8888)`: zeroed pixels, blending on, no color key.
    pub fn new(w: i32, h: i32) -> Self {
        assert!(w >= 0 && h >= 0);
        Surface {
            w,
            h,
            pixels: vec![0; (w * h) as usize],
            clip: Rect::new(0, 0, w, h),
            color_key: None,
            alpha_mod: 255,
            blend: BlendMode::Blend,
        }
    }

    pub fn pixel(&self, x: i32, y: i32) -> u32 {
        self.pixels[(y * self.w + x) as usize]
    }

    /// `SDL_SetClipRect`: `None` clips to the whole surface. Returns whether the result is non-empty.
    pub fn set_clip_rect(&mut self, rect: Option<&Rect>) -> bool {
        let full = Rect::new(0, 0, self.w, self.h);
        match rect.map(|r| r.intersect(&full)) {
            None => {
                self.clip = full;
                true
            }
            Some(Some(r)) => {
                self.clip = r;
                true
            }
            Some(None) => {
                self.clip = Rect::new(0, 0, 0, 0);
                false
            }
        }
    }

    /// `SDL_FillRect`: `None` fills the clip rectangle; a rectangle is intersected with it.
    pub fn fill_rect(&mut self, rect: Option<&Rect>, color: u32) {
        let area = match rect {
            None => Some(self.clip),
            Some(r) => r.intersect(&self.clip),
        };
        let Some(area) = area else { return };
        if area.w <= 0 || area.h <= 0 {
            return;
        }
        for y in area.y..area.y + area.h {
            let row = (y * self.w + area.x) as usize;
            self.pixels[row..row + area.w as usize].fill(color);
        }
    }
}

/// sdl2-compat's `SDL_UpperBlit`: clips `src_rect` to `src` and the destination to `dst`'s clip rectangle, then
/// blits. On a blit, `dst_rect` becomes the clipped destination; otherwise only its size is zeroed.
pub fn upper_blit(src: &Surface, src_rect: Option<&Rect>, dst: &mut Surface, dst_rect: Option<&mut Rect>) {
    let mut r_src = Rect::new(0, 0, src.w, src.h);
    let mut r_dst = match &dst_rect {
        Some(d) => Rect::new(d.x, d.y, 0, 0),
        None => Rect::default(),
    };

    let clipped = (|| {
        if let Some(sr) = src_rect {
            let tmp = sr.intersect(&r_src)?;
            r_dst.x += tmp.x - sr.x;
            r_dst.y += tmp.y - sr.y;
            r_src = tmp;
        }
        r_dst.w = r_src.w;
        r_dst.h = r_src.h;

        let tmp = r_dst.intersect(&dst.clip)?;
        r_src.x += tmp.x - r_dst.x;
        r_src.y += tmp.y - r_dst.y;
        r_src.w = tmp.w;
        r_src.h = tmp.h;
        r_dst = tmp;
        Some(())
    })();

    match (clipped, dst_rect) {
        (Some(()), out) => {
            if let Some(out) = out {
                *out = r_dst;
            }
            lower_blit(src, &r_src, dst, r_dst.x, r_dst.y);
        }
        (None, Some(out)) => {
            out.w = 0;
            out.h = 0;
        }
        (None, None) => {}
    }
}

/// `SDL_LowerBlit` on rectangles already clipped by `upper_blit`.
fn lower_blit(src: &Surface, sr: &Rect, dst: &mut Surface, dx: i32, dy: i32) {
    let blend = src.blend == BlendMode::Blend;
    let modulate = src.alpha_mod != 255;
    let alpha_mod = src.alpha_mod as u32;
    let key = src.color_key.map(|k| k & RGB_MASK);

    for row in 0..sr.h {
        let s0 = ((sr.y + row) * src.w + sr.x) as usize;
        let d0 = ((dy + row) * dst.w + dx) as usize;
        let srow = &src.pixels[s0..s0 + sr.w as usize];
        let drow = &mut dst.pixels[d0..d0 + sr.w as usize];
        for (s, d) in srow.iter().zip(drow.iter_mut()) {
            let s = *s;
            if let Some(k) = key {
                if s & RGB_MASK == k {
                    continue;
                }
            }
            *d = match (blend, key.is_some(), modulate) {
                (false, _, false) => s,
                (false, _, true) => (s & RGB_MASK) | (mult_div_255(s >> 24, alpha_mod) << 24),
                (true, false, false) => blend_pixel_alpha(s, *d),
                (true, false, true) => blend_modulate(s, *d, alpha_mod),
                (true, true, m) => blend_slow(s, *d, m.then_some(alpha_mod)),
            };
        }
    }
}

/// SDL3's `SDL_ConvertSurface` of an 8-bit indexed image to ARGB8888. The color key becomes alpha 0 in the
/// key index's pixels, and the result blends without a color key.
pub fn convert_indexed(w: i32, h: i32, pitch: usize, indices: &[u8], palette: &[[u8; 4]], key_index: Option<u8>) -> Surface {
    let mut out = Surface::new(w, h);
    let opaque = palette.iter().all(|c| c[3] == 255);
    let colors: Vec<u32> = (0..256usize)
        .map(|i| {
            let c = palette.get(i).copied().unwrap_or([0, 0, 0, 255]);
            let a = if Some(i as u8) == key_index {
                0
            } else if opaque {
                255
            } else {
                c[3] as u32
            };
            pack(c[0] as u32, c[1] as u32, c[2] as u32, a)
        })
        .collect();
    for y in 0..h as usize {
        for x in 0..w as usize {
            out.pixels[y * w as usize + x] = colors[indices[y * pitch + x] as usize];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::sys::*;

    /// xorshift32, so the cases are the same on every run.
    struct Rng(u32, bool);
    impl Rng {
        fn new(seed: u32) -> Self {
            Rng(seed, true)
        }

        fn next(&mut self) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            self.0
        }
        fn range(&mut self, lo: i32, hi: i32) -> i32 {
            lo + (self.next() % (hi - lo + 1) as u32) as i32
        }
        fn pixel(&mut self) -> u32 {
            let partial = self.1;
            let a = match self.next() % 4 {
                0 => 0,
                1 => 255,
                _ if partial => self.next() & 0xFF,
                _ => 255,
            };
            (a << 24) | (self.next() & RGB_MASK)
        }
    }

    /// The goldens come from sdl2-compat, whose versions start at 2.32.50. Real SDL2 rounds partial per-pixel
    /// alpha and sets the converted blend mode differently, so those checks only run against sdl2-compat.
    fn linked_sdl_is_sdl2_compat() -> bool {
        let mut v = SDL_version { major: 0, minor: 0, patch: 0 };
        unsafe { SDL_GetVersion(&mut v) };
        (v.major, v.minor) == (2, 32) && v.patch >= 50
    }

    fn random_surface(rng: &mut Rng, w: i32, h: i32) -> Surface {
        let mut s = Surface::new(w, h);
        for p in &mut s.pixels {
            *p = rng.pixel();
        }
        s
    }

    unsafe fn to_sdl(s: &Surface) -> *mut SDL_Surface {
        let surf = SDL_CreateRGBSurfaceWithFormat(0, s.w, s.h, 32, SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32);
        assert!(!surf.is_null());
        for y in 0..s.h {
            let row = ((*surf).pixels as *mut u8).add((y * (*surf).pitch) as usize) as *mut u32;
            for x in 0..s.w {
                *row.add(x as usize) = s.pixel(x, y);
            }
        }
        if let Some(k) = s.color_key {
            SDL_SetColorKey(surf, 1, k);
        }
        let mode = match s.blend {
            BlendMode::None => SDL_BlendMode::SDL_BLENDMODE_NONE,
            BlendMode::Blend => SDL_BlendMode::SDL_BLENDMODE_BLEND,
        };
        SDL_SetSurfaceBlendMode(surf, mode);
        SDL_SetSurfaceAlphaMod(surf, s.alpha_mod);
        let c = s.clip;
        SDL_SetClipRect(surf, &SDL_Rect { x: c.x, y: c.y, w: c.w, h: c.h });
        surf
    }

    unsafe fn sdl_pixels(surf: *mut SDL_Surface) -> Vec<u32> {
        let mut out = Vec::new();
        for y in 0..(*surf).h {
            let row = ((*surf).pixels as *const u8).add((y * (*surf).pitch) as usize) as *const u32;
            for x in 0..(*surf).w {
                out.push(*row.add(x as usize));
            }
        }
        out
    }

    fn sdl_rect(r: &Rect) -> SDL_Rect {
        SDL_Rect { x: r.x, y: r.y, w: r.w, h: r.h }
    }

    fn first_difference(a: &[u32], b: &[u32], w: i32) -> Option<String> {
        a.iter()
            .zip(b)
            .position(|(x, y)| x != y)
            .map(|i| format!("pixel ({}, {}): rust {:08x}, sdl {:08x}", i as i32 % w, i as i32 / w, a[i], b[i]))
    }

    /// Blits random surfaces in one source state through both blitters and compares pixels and the written rect.
    fn check_blits(blend: BlendMode, keyed: bool, alpha_mod: Option<u8>, seed: u32) {
        let mut rng = Rng::new(seed);
        // Real SDL2 matches sdl2-compat only on opaque and transparent pixels for this path.
        rng.1 = !(blend == BlendMode::Blend && !keyed && alpha_mod.is_none()) || linked_sdl_is_sdl2_compat();
        for case in 0..300 {
            let (sw, sh) = (rng.range(1, 24), rng.range(1, 24));
            let (dw, dh) = (rng.range(1, 24), rng.range(1, 24));
            let mut src = random_surface(&mut rng, sw, sh);
            src.blend = blend;
            src.alpha_mod = alpha_mod.unwrap_or(255);
            if keyed {
                let k = src.pixel(rng.range(0, sw - 1), rng.range(0, sh - 1));
                src.color_key = Some(k ^ ((rng.next() & 1) << 31));
            }
            let mut dst = random_surface(&mut rng, dw, dh);
            if rng.next() % 3 == 0 {
                dst.set_clip_rect(Some(&Rect::new(rng.range(-4, dw), rng.range(-4, dh), rng.range(0, 20), rng.range(0, 20))));
            }
            let src_rect = (rng.next() % 4 != 0)
                .then(|| Rect::new(rng.range(-8, sw + 2), rng.range(-8, sh + 2), rng.range(-2, 30), rng.range(-2, 30)));
            let dst_rect = (rng.next() % 4 != 0)
                .then(|| Rect::new(rng.range(-30, dw + 4), rng.range(-30, dh + 4), rng.range(-2, 30), rng.range(-2, 30)));

            unsafe {
                let ssrc = to_sdl(&src);
                let sdst = to_sdl(&dst);
                let ssr = src_rect.map(|r| sdl_rect(&r));
                let mut sdr = dst_rect.map(|r| sdl_rect(&r));
                let rc = SDL_UpperBlit(
                    ssrc,
                    ssr.as_ref().map_or(std::ptr::null(), |r| r as *const SDL_Rect),
                    sdst,
                    sdr.as_mut().map_or(std::ptr::null_mut(), |r| r as *mut SDL_Rect),
                );
                assert_eq!(rc, 0);

                let mut rdr = dst_rect;
                upper_blit(&src, src_rect.as_ref(), &mut dst, rdr.as_mut());

                let expected = sdl_pixels(sdst);
                if let Some(d) = first_difference(&dst.pixels, &expected, dst.w) {
                    panic!("{blend:?} keyed={keyed} mod={alpha_mod:?} case {case}: {d}");
                }
                if let (Some(r), Some(s)) = (rdr, sdr) {
                    assert_eq!((r.x, r.y, r.w, r.h), (s.x, s.y, s.w, s.h), "dst rect, case {case}");
                }
                SDL_FreeSurface(ssrc);
                SDL_FreeSurface(sdst);
            }
        }
    }

    #[test]
    fn copy_matches_sdl() {
        check_blits(BlendMode::None, false, Option::None, 1);
    }

    #[test]
    fn color_key_matches_sdl() {
        check_blits(BlendMode::None, true, Option::None, 2);
    }

    #[test]
    fn pixel_alpha_matches_sdl() {
        check_blits(BlendMode::Blend, false, Option::None, 3);
    }

    #[test]
    fn alpha_mod_matches_sdl() {
        for (i, m) in [0u8, 1, 64, 127, 128, 200, 254].into_iter().enumerate() {
            check_blits(BlendMode::Blend, false, Some(m), 10 + i as u32);
        }
    }

    #[test]
    fn color_key_with_blend_matches_sdl() {
        check_blits(BlendMode::Blend, true, Option::None, 4);
        for (i, m) in [0u8, 64, 128, 200].into_iter().enumerate() {
            check_blits(BlendMode::Blend, true, Some(m), 20 + i as u32);
        }
    }

    #[test]
    fn fill_rect_matches_sdl() {
        let mut rng = Rng::new(5);
        for case in 0..300 {
            let (w, h) = (rng.range(1, 24), rng.range(1, 24));
            let mut dst = random_surface(&mut rng, w, h);
            if rng.next() % 3 == 0 {
                dst.set_clip_rect(Some(&Rect::new(rng.range(-4, w), rng.range(-4, h), rng.range(0, 20), rng.range(0, 20))));
            }
            let rect = (rng.next() % 4 != 0).then(|| Rect::new(rng.range(-10, w), rng.range(-10, h), rng.range(-2, 30), rng.range(-2, 30)));
            let color = rng.next();
            unsafe {
                let sdst = to_sdl(&dst);
                let sr = rect.map(|r| sdl_rect(&r));
                SDL_FillRect(sdst, sr.as_ref().map_or(std::ptr::null(), |r| r as *const SDL_Rect), color);
                dst.fill_rect(rect.as_ref(), color);
                if let Some(d) = first_difference(&dst.pixels, &sdl_pixels(sdst), w) {
                    panic!("fill case {case}: {d}");
                }
                SDL_FreeSurface(sdst);
            }
        }
    }

    #[test]
    fn indexed_conversion_matches_sdl() {
        let mut rng = Rng::new(6);
        for case in 0..50 {
            let (w, h) = (rng.range(1, 20), rng.range(1, 20));
            let ncolors = rng.range(1, 256) as usize;
            let palette: Vec<[u8; 4]> = (0..ncolors)
                .map(|_| {
                    let p = rng.next();
                    let a = if case % 2 == 0 { 255 } else { (p >> 24) as u8 };
                    [p as u8, (p >> 8) as u8, (p >> 16) as u8, a]
                })
                .collect();
            let key = (rng.next() % 2 == 0).then(|| rng.range(0, ncolors as i32 - 1) as u8);
            unsafe {
                let surf = SDL_CreateRGBSurfaceWithFormat(0, w, h, 8, SDL_PixelFormatEnum::SDL_PIXELFORMAT_INDEX8 as u32);
                let colors: Vec<SDL_Color> = palette.iter().map(|c| SDL_Color { r: c[0], g: c[1], b: c[2], a: c[3] }).collect();
                SDL_SetPaletteColors((*(*surf).format).palette, colors.as_ptr(), 0, ncolors as i32);
                let pitch = (*surf).pitch as usize;
                let mut indices = vec![0u8; pitch * h as usize];
                for y in 0..h as usize {
                    for x in 0..w as usize {
                        indices[y * pitch + x] = rng.range(0, ncolors as i32 - 1) as u8;
                    }
                }
                std::ptr::copy_nonoverlapping(indices.as_ptr(), (*surf).pixels as *mut u8, indices.len());
                if let Some(k) = key {
                    SDL_SetColorKey(surf, 1, k as u32);
                }
                let fmt = SDL_AllocFormat(SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32);
                let conv = SDL_ConvertSurface(surf, fmt, 0);
                assert!(!conv.is_null());

                let ours = convert_indexed(w, h, pitch, &indices, &palette, key);
                if let Some(d) = first_difference(&ours.pixels, &sdl_pixels(conv), w) {
                    panic!("indexed case {case}: {d}");
                }
                let mut sdl_key = 0;
                assert_eq!(SDL_GetColorKey(conv, &mut sdl_key), -1, "converted surface keeps no color key, case {case}");
                let mut mode = SDL_BlendMode::SDL_BLENDMODE_NONE;
                SDL_GetSurfaceBlendMode(conv, &mut mode);
                if linked_sdl_is_sdl2_compat() {
                    assert!(mode == SDL_BlendMode::SDL_BLENDMODE_BLEND, "blend mode, case {case}");
                }
                SDL_FreeFormat(fmt);
                SDL_FreeSurface(conv);
                SDL_FreeSurface(surf);
            }
        }
    }
}
