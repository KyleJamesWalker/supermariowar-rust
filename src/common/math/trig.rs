//! Correctly rounded `sinf`, `cosf` and `atan2f` from CORE-MATH (https://core-math.gitlabpages.inria.fr/,
//! commit 284b3b0e, `src/binary32/{sin,cos,atan2}`), ported to Rust. The C++ reference links the same C files.
//!
//! sinf: Copyright (c) 2022-2025 Alexei Sibidanov.
//! cosf: Copyright (c) 2022-2023 Alexei Sibidanov.
//! atan2f: Copyright (c) 2022-2025 Alexei Sibidanov and Paul Zimmermann.
//!
//! This file is part of the CORE-MATH project
//! (https://core-math.gitlabpages.inria.fr/).
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

/// A C hexadecimal floating literal, exactly (every one used here is a double).
const fn hx(s: &str) -> f64 {
    let b = s.as_bytes();
    let mut i = 0;
    let neg = b[0] == b'-';
    if neg {
        i += 1;
    }
    i += 2;
    let mut m: u64 = 0;
    let mut frac_digits = 0i32;
    let mut in_frac = false;
    while b[i] != b'p' {
        if b[i] == b'.' {
            in_frac = true;
        } else {
            let d = b[i];
            let v = if d >= b'a' { d - b'a' + 10 } else { d - b'0' };
            m = (m << 4) | v as u64;
            if in_frac {
                frac_digits += 1;
            }
        }
        i += 1;
    }
    i += 1;
    let eneg = b[i] == b'-';
    if b[i] == b'-' || b[i] == b'+' {
        i += 1;
    }
    let mut e = 0i32;
    while i < b.len() {
        e = e * 10 + (b[i] - b'0') as i32;
        i += 1;
    }
    if eneg {
        e = -e;
    }
    let sign = if neg { 1u64 << 63 } else { 0 };
    if m == 0 {
        return f64::from_bits(sign);
    }
    let lead = 63 - m.leading_zeros() as i32;
    assert!(lead <= 52);
    let e = e - 4 * frac_digits + lead;
    assert!(e > -1023 && e < 1024);
    let mant = (m << (52 - lead)) & ((1u64 << 52) - 1);
    f64::from_bits(sign | ((e + 1023) as u64) << 52 | mant)
}

const IPI: [u64; 4] = [0xfe5163abdebbc562, 0xdb6295993c439041, 0xfc2757d1f534ddc0, 0xa2f9836e4e441529];
const B: [f64; 4] = [
    hx("0x1.3bd3cc9be45dcp-6"),
    hx("-0x1.03c1f081b0833p-14"),
    hx("0x1.55d3c6fc9ac1fp-24"),
    hx("-0x1.e1d3ff281b40dp-35"),
];
const A: [f64; 4] = [
    hx("0x1.921fb54442d17p-3"),
    hx("-0x1.4abbce6256a39p-10"),
    hx("0x1.466bc5a518c16p-19"),
    hx("-0x1.32bdc61074ff6p-29"),
];
const SIN_TB: [f64; 32] = [
    hx("0x0p+0"),
    hx("0x1.8f8b83c69a60bp-3"),
    hx("0x1.87de2a6aea963p-2"),
    hx("0x1.1c73b39ae68c8p-1"),
    hx("0x1.6a09e667f3bcdp-1"),
    hx("0x1.a9b66290ea1a3p-1"),
    hx("0x1.d906bcf328d46p-1"),
    hx("0x1.f6297cff75cbp-1"),
    hx("0x1p+0"),
    hx("0x1.f6297cff75cbp-1"),
    hx("0x1.d906bcf328d46p-1"),
    hx("0x1.a9b66290ea1a3p-1"),
    hx("0x1.6a09e667f3bcdp-1"),
    hx("0x1.1c73b39ae68c8p-1"),
    hx("0x1.87de2a6aea963p-2"),
    hx("0x1.8f8b83c69a60bp-3"),
    hx("0x0p+0"),
    hx("-0x1.8f8b83c69a60bp-3"),
    hx("-0x1.87de2a6aea963p-2"),
    hx("-0x1.1c73b39ae68c8p-1"),
    hx("-0x1.6a09e667f3bcdp-1"),
    hx("-0x1.a9b66290ea1a3p-1"),
    hx("-0x1.d906bcf328d46p-1"),
    hx("-0x1.f6297cff75cbp-1"),
    hx("-0x1p+0"),
    hx("-0x1.f6297cff75cbp-1"),
    hx("-0x1.d906bcf328d46p-1"),
    hx("-0x1.a9b66290ea1a3p-1"),
    hx("-0x1.6a09e667f3bcdp-1"),
    hx("-0x1.1c73b39ae68c8p-1"),
    hx("-0x1.87de2a6aea963p-2"),
    hx("-0x1.8f8b83c69a60bp-3"),
];
const COS_TB: [f64; 32] = [
    hx("0x1p+0"),
    hx("0x1.f6297cff75cbp-1"),
    hx("0x1.d906bcf328d46p-1"),
    hx("0x1.a9b66290ea1a3p-1"),
    hx("0x1.6a09e667f3bcdp-1"),
    hx("0x1.1c73b39ae68c8p-1"),
    hx("0x1.87de2a6aea963p-2"),
    hx("0x1.8f8b83c69a60bp-3"),
    hx("0x0p+0"),
    hx("-0x1.8f8b83c69a60bp-3"),
    hx("-0x1.87de2a6aea963p-2"),
    hx("-0x1.1c73b39ae68c8p-1"),
    hx("-0x1.6a09e667f3bcdp-1"),
    hx("-0x1.a9b66290ea1a3p-1"),
    hx("-0x1.d906bcf328d46p-1"),
    hx("-0x1.f6297cff75cbp-1"),
    hx("-0x1p+0"),
    hx("-0x1.f6297cff75cbp-1"),
    hx("-0x1.d906bcf328d46p-1"),
    hx("-0x1.a9b66290ea1a3p-1"),
    hx("-0x1.6a09e667f3bcdp-1"),
    hx("-0x1.1c73b39ae68c8p-1"),
    hx("-0x1.87de2a6aea963p-2"),
    hx("-0x1.8f8b83c69a60bp-3"),
    hx("0x0p+0"),
    hx("0x1.8f8b83c69a60bp-3"),
    hx("0x1.87de2a6aea963p-2"),
    hx("0x1.1c73b39ae68c8p-1"),
    hx("0x1.6a09e667f3bcdp-1"),
    hx("0x1.a9b66290ea1a3p-1"),
    hx("0x1.d906bcf328d46p-1"),
    hx("0x1.f6297cff75cbp-1"),
];

fn rbig(u: u32) -> (f64, i32) {
    let e = ((u >> 23) & 0xff) as i32;
    let m = ((u & (!0u32 >> 9)) | 1 << 23) as u64;
    let p0 = m as u128 * IPI[0] as u128;
    let mut p1 = m as u128 * IPI[1] as u128;
    p1 += p0 >> 64;
    let mut p2 = m as u128 * IPI[2] as u128;
    p2 += p1 >> 64;
    let mut p3 = m as u128 * IPI[3] as u128;
    p3 += p2 >> 64;
    let (p3h, p3l, p2l, p1l) = ((p3 >> 64) as u64, p3 as u64, p2 as u64, p1 as u64);
    let k = e - 124;
    let s = k - 23;
    let (mut i, a) = if s < 64 {
        ((p3h << s | p3l >> (64 - s)) as i32, (p3l << s | p2l >> (64 - s)) as i64)
    } else if s == 64 {
        (p3l as i32, p2l as i64)
    } else {
        ((p3l << (s - 64) | p2l >> (128 - s)) as i32, (p2l << (s - 64) | p1l >> (128 - s)) as i64)
    };
    let sgn = (u as i32) >> 31;
    let sm = a >> 63;
    i = i.wrapping_sub(sm as i32);
    let z = (a ^ sgn as i64) as f64 * hx("0x1p-64");
    i = (i ^ sgn).wrapping_sub(sgn);
    (z, i)
}

fn rltl(z: f32) -> (f64, i32) {
    let x = z as f64;
    let idl = const { hx("-0x1.b1bbead603d8bp-29") } * x;
    let idh = const { hx("0x1.45f306ep+2") } * x;
    let id = idh.round_ties_even();
    ((idh - id) + idl, (const { hx("0x1.8p52") } + id).to_bits() as i32)
}

fn rltl0(x: f64) -> (f64, i32) {
    let idh = const { hx("0x1.45f306dc9c883p+2") } * x;
    let id = idh.round_ties_even();
    (idh - id, (const { hx("0x1.8p52") } + id).to_bits() as i32)
}

fn poly(z2: f64, c: &[f64; 4]) -> f64 {
    let z4 = z2 * z2;
    (c[0] + z2 * c[1]) + z4 * (c[2] + z2 * c[3])
}

struct Exception {
    arg: u32,
    rh: f32,
    rl: f32,
}

const fn exc(arg: &str, rh: &str, rl: &str) -> Exception {
    Exception { arg: (hx(arg) as f32).to_bits(), rh: hx(rh) as f32, rl: hx(rl) as f32 }
}

fn sinf_database(x: f32) -> f32 {
    const ST: [Exception; 4] = [
        exc("0x1.33333p+13", "-0x1.63f4bap-2", "-0x1p-27"),
        exc("0x1.75b8a2p-1", "0x1.55688ap-1", "-0x1p-26"),
        exc("0x1.4f0654p+0", "0x1.ee836cp-1", "-0x1p-26"),
        exc("0x1.2d97c8p+3", "-0x1.99bc5ap-26", "-0x1p-51"),
    ];
    let ax = x.to_bits() & (!0u32 >> 1);
    let e = ST.iter().find(|e| e.arg == ax).unwrap();
    let sgn = 1.0f32.copysign(x);
    sgn * e.rh + sgn * e.rl
}

fn sinf_big(x: f32) -> f32 {
    let u = x.to_bits();
    let ax = u << 1;
    if ax >= 0xff << 24 {
        return if ax << 8 != 0 { x + x } else { f32::NAN };
    }
    let (z, ia) = rbig(u);
    let z2 = z * z;
    let (aa, bb) = (poly(z2, &A), poly(z2, &B));
    let (s0, c0) = (SIN_TB[(ia & 31) as usize], SIN_TB[((ia as u32).wrapping_add(8) & 31) as usize]);
    (s0 + z * (aa * c0 - bb * (z * s0))) as f32
}

pub fn sinf(x: f32) -> f32 {
    let ax = x.to_bits() << 1;
    if ax > 0x99000000 || ax < 0x73000000 {
        if ax < 0x73000000 {
            if ax < 0x66000000 {
                if ax == 0 {
                    return x;
                }
                return (-x).mul_add(x.abs(), x);
            }
            return (const { hx("-0x1.555556p-3") as f32 } * x) * (x * x) + x;
        }
        return sinf_big(x);
    }
    let (z, ia) = if ax < 0x822d97c8 {
        if ax == 0x7e75b8a2 || ax == 0x7f4f0654 {
            return sinf_database(x);
        }
        rltl0(x as f64)
    } else {
        if ax == 0x8c333330 {
            return sinf_database(x);
        }
        rltl(x)
    };
    let z2 = z * z;
    let (aa, bb) = (poly(z2, &A), poly(z2, &B));
    let (s0, c0) = (SIN_TB[(ia & 31) as usize], SIN_TB[((ia + 8) & 31) as usize]);
    (s0 + aa * (z * c0) - bb * (z2 * s0)) as f32
}

fn cosf_database(x: f32, r: f64) -> f32 {
    const ST: [Exception; 5] = [
        exc("0x1.2d97c8p+2", "0x1.99bc5cp-27", "-0x1p-52"),
        exc("0x1.4555p+51", "0x1.115d7ep-1", "-0x1p-26"),
        exc("0x1.48a858p+54", "0x1.f48148p-2", "0x1p-27"),
        exc("0x1.3170fp+63", "0x1.fe2976p-1", "0x1p-26"),
        exc("0x1.2b9622p+67", "0x1.f0285ep-1", "-0x1p-26"),
    ];
    let ax = x.to_bits() & (!0u32 >> 1);
    match ST.iter().find(|e| e.arg == ax) {
        Some(e) => e.rh + e.rl,
        None => r as f32,
    }
}

fn cosf_big(x: f32) -> f32 {
    let u = x.to_bits();
    let ax = u << 1;
    if ax >= 0xff << 24 {
        return if ax << 8 != 0 { x + x } else { f32::NAN };
    }
    let (z, ia) = rbig(u);
    let z2 = z * z;
    let (aa, bb) = (poly(z2, &A), poly(z2, &B));
    let (s0, c0) = (COS_TB[((ia as u32).wrapping_add(8) & 31) as usize], COS_TB[(ia & 31) as usize]);
    let r = c0 + z * (aa * s0 - bb * (z * c0));
    let tail = r.to_bits().wrapping_add(6) & (!0u64 >> 36);
    if tail <= 12 {
        return cosf_database(x, r);
    }
    r as f32
}

pub fn cosf(x: f32) -> f32 {
    let ax = x.to_bits() << 1;
    if ax > 0x99000000 || ax < 0x74000000 {
        if ax < 0x74000000 {
            return 0.5 * x.mul_add(-x, 2.0);
        }
        return cosf_big(x);
    }
    let (z, ia) = if ax < 0x82a41896 {
        if ax == 0x812d97c8 {
            return cosf_database(x, 0.0);
        }
        rltl0(x as f64)
    } else {
        rltl(x)
    };
    let z2 = z * z;
    let (aa, bb) = (poly(z2, &A), poly(z2, &B));
    let (c0, s0) = (COS_TB[(ia & 31) as usize], COS_TB[((ia + 8) & 31) as usize]);
    (c0 + aa * (z * s0) - bb * (z2 * c0)) as f32
}

fn muldd(xh: f64, xl: f64, ch: f64, cl: f64) -> (f64, f64) {
    let ahlh = ch * xl;
    let alhh = cl * xh;
    let ahhh = ch * xh;
    let mut ahhl = ch.mul_add(xh, -ahhh);
    ahhl += alhh + ahlh;
    let h = ahhh + ahhl;
    (h, (ahhh - h) + ahhl)
}

fn polydd(xh: f64, xl: f64, c: &[[f64; 2]]) -> (f64, f64) {
    let mut i = c.len() - 1;
    let (mut ch, mut cl) = (c[i][0], c[i][1]);
    while i > 0 {
        i -= 1;
        (ch, cl) = muldd(xh, xl, ch, cl);
        let th = ch + c[i][0];
        let tl = (c[i][0] - th) + ch;
        ch = th;
        cl += tl + c[i][1];
    }
    (ch, cl)
}

fn atan2f_tiny(y: f32, x: f32) -> f32 {
    let (dy, dx) = (y as f64, x as f64);
    let z = dy / dx;
    let mut e = (-z).mul_add(dx, dy);
    let c = const { hx("-0x1.5555555555555p-2") };
    let zz = z * z;
    let cz = c * z;
    e = e / dx + cz * zz;
    let mut t = z.to_bits();
    if t & 0xfffffff == 0 {
        if z * e > 0.0 {
            t = t.wrapping_add(1);
        } else {
            t = t.wrapping_sub(1);
        }
    }
    f64::from_bits(t) as f32
}

const CN: [f64; 7] = [
    hx("0x1p+0"),
    hx("0x1.40e0698f94c35p+1"),
    hx("0x1.248c5da347f0dp+1"),
    hx("0x1.d873386572976p-1"),
    hx("0x1.46fa40b20f1dp-3"),
    hx("0x1.33f5e041eed0fp-7"),
    hx("0x1.546bbf28667c5p-14"),
];
const CD: [f64; 7] = [
    hx("0x1p+0"),
    hx("0x1.6b8b143a3f6dap+1"),
    hx("0x1.8421201d18ed5p+1"),
    hx("0x1.8221d086914ebp+0"),
    hx("0x1.670657e3a07bap-2"),
    hx("0x1.0f4951fd1e72dp-5"),
    hx("0x1.b3874b8798286p-11"),
];
const C32: [[f64; 2]; 32] = [
    [hx("0x1p+0"), hx("-0x1.8c1dac5492248p-87")],
    [hx("-0x1.5555555555555p-2"), hx("-0x1.55553bf3a2abep-56")],
    [hx("0x1.999999999999ap-3"), hx("-0x1.99deed1ec9071p-57")],
    [hx("-0x1.2492492492492p-3"), hx("-0x1.fd99c8d18269ap-58")],
    [hx("0x1.c71c71c71c717p-4"), hx("-0x1.651eee4c4d9dp-61")],
    [hx("-0x1.745d1745d1649p-4"), hx("-0x1.632683d6c44a6p-58")],
    [hx("0x1.3b13b13b11c63p-4"), hx("0x1.bf69c1f8af41dp-58")],
    [hx("-0x1.11111110e6338p-4"), hx("0x1.3c3e431e8bb68p-61")],
    [hx("0x1.e1e1e1dc45c4ap-5"), hx("-0x1.be2db05c77bbfp-59")],
    [hx("-0x1.af286b8164b4fp-5"), hx("0x1.a4673491f0942p-61")],
    [hx("0x1.86185e9ad4846p-5"), hx("0x1.e12e32d79fceep-59")],
    [hx("-0x1.642c6d5161faep-5"), hx("0x1.3ce76c1ca03fp-59")],
    [hx("0x1.47ad6f277e5bfp-5"), hx("-0x1.abd8d85bdb714p-60")],
    [hx("-0x1.2f64a2ee8896dp-5"), hx("0x1.ef87d4b615323p-61")],
    [hx("0x1.1a6a2b31741b5p-5"), hx("0x1.a5d9d973547eep-62")],
    [hx("-0x1.07fbdad65e0a6p-5"), hx("-0x1.65ac07f5d35f4p-61")],
    [hx("0x1.ee9932a9a5f8bp-6"), hx("0x1.f8b9623f6f55ap-61")],
    [hx("-0x1.ce8b5b9584dc6p-6"), hx("0x1.fe5af96e8ea2dp-61")],
    [hx("0x1.ac9cb288087b7p-6"), hx("-0x1.450cdfceaf5cap-60")],
    [hx("-0x1.84b025351f3e6p-6"), hx("0x1.579561b0d73dap-61")],
    [hx("0x1.52f5b8ecdd52bp-6"), hx("0x1.036bd2c6fba47p-60")],
    [hx("-0x1.163a8c44909dcp-6"), hx("0x1.18f735ffb9f16p-60")],
    [hx("0x1.a400dce3eea6fp-7"), hx("-0x1.c90569c0c1b5cp-61")],
    [hx("-0x1.1caa78ae6db3ap-7"), hx("-0x1.4c60f8161ea09p-61")],
    [hx("0x1.52672453c0731p-8"), hx("0x1.834efb598c338p-62")],
    [hx("-0x1.5850c5be137cfp-9"), hx("-0x1.445fc150ca7f5p-63")],
    [hx("0x1.23eb98d22e1cap-10"), hx("-0x1.388fbaf1d783p-64")],
    [hx("-0x1.8f4e974a40741p-12"), hx("0x1.271198a97da34p-66")],
    [hx("0x1.a5cf2e9cf76e5p-14"), hx("-0x1.887eb4a63b665p-68")],
    [hx("-0x1.420c270719e32p-16"), hx("0x1.efd595b27888bp-71")],
    [hx("0x1.3ba2d69b51677p-19"), hx("-0x1.4fb06829cdfc7p-73")],
    [hx("-0x1.29b7e6f676385p-23"), hx("-0x1.a783b6de718fbp-77")],
];
const PI: f64 = hx("0x1.921fb54442d18p+1");
const PI2: f64 = hx("0x1.921fb54442d18p+0");
const PI2L: f64 = hx("0x1.1a62633145c07p-54");
const OFF: [f64; 8] = [0.0, PI2, PI, PI2, -0.0, -PI2, -PI, -PI2];
const OFFL: [f64; 8] = [0.0, PI2L, 2.0 * PI2L, PI2L, -0.0, -PI2L, -2.0 * PI2L, -PI2L];
const SGN: [f64; 2] = [1.0, -1.0];

pub fn atan2f(y: f32, x: f32) -> f32 {
    let m = [0.0f64, 1.0];
    let (ux, uy) = (x.to_bits(), y.to_bits());
    let (ax, ay) = (ux & (!0u32 >> 1), uy & (!0u32 >> 1));
    if ay >= 0xff << 23 || ax >= 0xff << 23 {
        if ay > 0xff << 23 {
            return x + y;
        }
        if ax > 0xff << 23 {
            return x + y;
        }
        let yinf = ay == 0xff << 23;
        let xinf = ax == 0xff << 23;
        if yinf && xinf {
            return if ux >> 31 != 0 {
                (const { hx("0x1.2d97c7f3321d2p+1") } * SGN[(uy >> 31) as usize]) as f32
            } else {
                (const { hx("0x1.921fb54442d18p-1") } * SGN[(uy >> 31) as usize]) as f32
            };
        }
        if xinf {
            return if ux >> 31 != 0 { (PI * SGN[(uy >> 31) as usize]) as f32 } else { (0.0 * SGN[(uy >> 31) as usize]) as f32 };
        }
        if yinf {
            return (PI2 * SGN[(uy >> 31) as usize]) as f32;
        }
    }
    if ay == 0 {
        if ax == 0 {
            let i = ((uy >> 31) * 4 + (ux >> 31) * 2) as usize;
            return if ux >> 31 != 0 { (OFF[i] + OFFL[i]) as f32 } else { OFF[i] as f32 };
        }
        if ux >> 31 == 0 {
            return (0.0 * SGN[(uy >> 31) as usize]) as f32;
        }
    }
    let gt = (ay > ax) as usize;
    let i = (uy >> 31) as usize * 4 + (ux >> 31) as usize * 2 + gt;

    let (zx, zy) = (x as f64, y as f64);
    let mut z = (m[gt] * zx + m[1 - gt] * zy) / (m[gt] * zy + m[1 - gt] * zx);
    let mut r;
    let d = ax as i32 - ay as i32;
    if d < (27 << 23) && d > -(27 << 23) {
        let z2 = z * z;
        let z4 = z2 * z2;
        let z8 = z4 * z4;
        let mut cn0 = CN[0] + z2 * CN[1];
        let cn2 = CN[2] + z2 * CN[3];
        let mut cn4 = CN[4] + z2 * CN[5];
        let cn6 = CN[6];
        cn0 += z4 * cn2;
        cn4 += z4 * cn6;
        cn0 += z8 * cn4;
        let mut cd0 = CD[0] + z2 * CD[1];
        let cd2 = CD[2] + z2 * CD[3];
        let mut cd4 = CD[4] + z2 * CD[5];
        let cd6 = CD[6];
        cd0 += z4 * cd2;
        cd4 += z4 * cd6;
        cd0 += z8 * cd4;
        r = cn0 / cd0;
    } else {
        r = 1.0;
    }
    z *= SGN[gt];
    r = z * r + OFF[i];
    if (r.to_bits().wrapping_add(8) & 0xfffffff) <= 16 {
        if ay < ax && ((ax - ay) >> 23 >= 25) {
            return atan2f_tiny(y, x);
        }
        let (mut zh, mut zl);
        if gt == 0 {
            zh = zy / zx;
            zl = zh.mul_add(-zx, zy) / zx;
        } else {
            zh = zx / zy;
            zl = zh.mul_add(-zy, zx) / zy;
        }
        let (z2h, z2l) = muldd(zh, zl, zh, zl);
        let (mut ph, mut pl) = polydd(z2h, z2l, &C32);
        zh *= SGN[gt];
        zl *= SGN[gt];
        (ph, pl) = muldd(zh, zl, ph, pl);
        let sh = ph + OFF[i];
        let sl = ((OFF[i] - sh) + ph) + pl + OFFL[i];
        let rf = sh as f32;
        let th = rf as f64;
        let dh = sh - th;
        let mut tm = dh + sl;
        if th + th * const { hx("0x1p-60") } == th - th * const { hx("0x1p-60") } {
            let mut tth = th.to_bits();
            tth &= 0x7ff << 52;
            tth = tth.wrapping_sub(24 << 52);
            if tm.abs() > f64::from_bits(tth) {
                tm *= 1.25;
            } else {
                tm *= 0.75;
            }
        }
        r = th + tm;
    }
    r as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The f32 nearest to r (known to within an f64 ulp), or None too close to a midpoint to tell.
    fn nearest(r: f64) -> Option<f32> {
        let c = r as f32;
        if c as f64 == r || !r.is_finite() {
            return Some(c);
        }
        let n = if (c as f64) < r { c.next_up() } else { c.next_down() };
        let mid = (c as f64 + n as f64) / 2.0;
        let ulp = f64::from_bits(r.abs().to_bits() + 1) - r.abs();
        ((r - mid).abs() > 2.0 * ulp).then_some(c)
    }

    #[test]
    fn values_the_platform_libms_round_differently() {
        let f = f32::from_bits;
        assert_eq!(sinf(f(0x39e89769)).to_bits(), 0x39e89768);
        assert_eq!(sinf(f(0x39e894cd)).to_bits(), 0x39e894cd);
        assert_eq!(cosf(f(0x3a0f1bbd)).to_bits(), 0x3f7ffffd);
        assert_eq!(cosf(f(0x39dda4c2)).to_bits(), 0x3f7fffff);
        assert_eq!(atan2f(f(0x0bf34dad), f(0xdc1b77ae)).to_bits(), 0x40490fdb);
        assert_eq!(sinf(hx("0x1.33333p+13") as f32), hx("-0x1.63f4bap-2") as f32 + hx("-0x1p-27") as f32);
        assert_eq!(cosf(hx("0x1.2d97c8p+2") as f32), hx("0x1.99bc5cp-27") as f32 + hx("-0x1p-52") as f32);
    }

    #[test]
    fn correctly_rounded() {
        let mut seed = 0x2545f491u32;
        for i in (0..=u32::MAX).step_by(4093) {
            let x = f32::from_bits(i);
            if !x.is_finite() {
                continue;
            }
            if let Some(want) = nearest((x as f64).sin()) {
                assert_eq!(sinf(x).to_bits(), want.to_bits(), "sinf {i:#x}");
            }
            if let Some(want) = nearest((x as f64).cos()) {
                assert_eq!(cosf(x).to_bits(), want.to_bits(), "cosf {i:#x}");
            }
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let y = f32::from_bits(seed);
            if y.is_finite() {
                if let Some(want) = nearest((y as f64).atan2(x as f64)) {
                    assert_eq!(atan2f(y, x).to_bits(), want.to_bits(), "atan2f {seed:#x} {i:#x}");
                }
            }
        }
    }
}
