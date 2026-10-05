//! The macOS arm64 libm's `sinf`, `cosf`, `__sincosf_stret` and `atan2f`, bit for bit on every target.
//! `sin`/`cos` are `__sincosf_stret`'s, which LLVM calls for a sin and cos of one angle; they differ from `sinf`/`cosf`.

const SIN: [u64; 3] = [0xbf29943e0fc4818f, 0x3f811073afd145eb, 0xbfc555545268a02a];
const COS: [u64; 4] = [0x3ef99169fc938f4a, 0xbf56c07f16945df4, 0x3fa55553c7899699, 0xbfdffffffcb82e97];
const LARGE: [u64; 5] = [0xc010a8b9fc97b22c, 0xc00d31f4a0a55230, 0x402ef88f0b1250c1, 0x400537b6744fa779, 0x3fb39502de83d49c];
const TWO_OVER_PI: u64 = 0x3fe45f306dc9c883;
const PI_OVER_2: u64 = 0x3ff921fb54442d18;
const PI_OVER_2_LO: u64 = 0x3c91a62633145c07;

// 1/pi without the bits that only add whole periods, per exponent byte from 0x40; head * x is exact.
#[rustfmt::skip]
const INV_PI: [[u64; 2]; 64] = [
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0x3fd45f306e000000, 0xbdeb1bbead603d8b], [0x3fd45f306e000000, 0xbdeb1bbead603d8b],
    [0xbfc7419f24000000, 0xbdeb1bbead603d8b], [0xbfad067c92000000, 0x3dc391054a7f09d6],
    [0x3f77cc1b72000000, 0x3d9c882a53f84eb0], [0xbf6067c91b000000, 0xbd6bbead603d8a83],
    [0xbf09f246c7000000, 0x3d0054a7f09d5f48], [0xbf09f246c7000000, 0x3d0054a7f09d5f48],
    [0xbf09f246c7000000, 0x3d0054a7f09d5f48], [0x3ee836e4e4000000, 0x3d0054a7f09d5f48],
    [0xbecf246c6f000000, 0x3ca529fc2757d1f5], [0x3e7b727221000000, 0xbc95ac07b1505c16],
    [0x3e7b727221000000, 0xbc95ac07b1505c16], [0xbe5236377d000000, 0xbc76b01ec5417056],
    [0x3e4b939105000000, 0x3c629fc2757d1f53], [0xbe21b1bbeb000000, 0x3c34fe13abe8fa9a],
    [0xbdeb1bbead000000, 0xbc080f62a0b82b2d], [0xbdeb1bbead000000, 0xbc080f62a0b82b2d],
    [0x3dc391054a000000, 0x3befc2757d1f534e], [0x3d9c882a54000000, 0xbb7ec54170565912],
    [0xbd6bbead60000000, 0xbb7ec54170565912], [0xbd6bbead60000000, 0xbb7ec54170565912],
    [0x3d41054a7f000000, 0x3b33abe8fa9a6ee0], [0x3d0054a7f1000000, 0xbb28a82e0acb223f],
    [0x3d0054a7f1000000, 0xbb28a82e0acb223f], [0x3ca529fc27000000, 0x3ac5f47d4d377037],
    [0x3ca529fc27000000, 0x3ac5f47d4d377037], [0x3ca529fc27000000, 0x3ac5f47d4d377037],
    [0x3c84a7f09d000000, 0x3aa7d1f534ddc0db], [0x3c629fc275000000, 0x3a8f47d4d377036e],
    [0x3c34fe13ac000000, 0xba370565911f924f], [0xbc2603d8a8000000, 0xba370565911f924f],
    [0xbc080f62a1000000, 0x3a21f534ddc0db63], [0x3befc2757d000000, 0x39ef534ddc0db629],
    [0xbb7ec54170000000, 0xb99596447e493ad5], [0xbb7ec54170000000, 0xb99596447e493ad5],
    [0xbb7ec54170000000, 0xb99596447e493ad5], [0x3b33abe8fb000000, 0xb9596447e493ad4d],
    [0x3b33abe8fb000000, 0xb9596447e493ad4d], [0xbb28a82e0b000000, 0x393a6ee06db14acd],
    [0x3b0d5f47d5000000, 0xb916447e493ad4ce], [0xbad505c159000000, 0xb8f911f924eb5336],
    [0x3ac5f47d4d000000, 0x38dbb81b6c52b328], [0x3aa7d1f535000000, 0xb8b11f924eb53362],
    [0x3a8f47d4d3000000, 0x38adc0db6295993c], [0xba37056591000000, 0xb83f924eb53361de],
    [0xba37056591000000, 0xb83f924eb53361de], [0x3a21f534de000000, 0xb83f924eb53361de],
    [0x39ef534ddc000000, 0x37db6c52b3278872], [0x39ef534ddc000000, 0x37db6c52b3278872],
    [0xb99596447e000000, 0xb7b24eb53361de38], [0xb99596447e000000, 0xb7b24eb53361de38],
    [0x3984d37703000000, 0x37ab6295993c4390], [0x39634ddc0e000000, 0xb78275a99b0ef1bf],
];

// Lane 0 is sin(r * pi/2) / r, lane 1 cos(r * pi/2).
const SINCOS: [[u64; 2]; 5] = [
    [0xc02c3d3ee4f8713f, 0xc020bc0396540c13],
    [0xc02ef09ab49e3a5b, 0xc02d6f5ade6c8c37],
    [0x3f24bc899499342c, 0x3f4d9c364e9721d5],
    [0x40444921a30f0d80, 0x401d78212789bc11],
    [0x406e97264e36bd93, 0x4062c6b9c7f88fa0],
];

const ATAN: [[u64; 2]; 4] = [
    [0x4002609372076e2c, 0xc00792d5bf17c0bc],
    [0x40038f4c070b63c1, 0x4015e43168d7c2b6],
    [0x3f953db49b9fcf6b, 0xc013e370146726b3],
    [0x400e4fa479729042, 0x401ae0fb08f2dc00],
];
const ATAN_SCALE: u64 = 0x3f680bc0fe23203d;
const ATAN_TINY: u64 = 0x3e90000000000000;
const PI: u64 = 0x400921fb54442d18;
const PI_F32: u64 = 0x400921fb40080000;
const PI_3_4: u64 = 0x4002d97c7f3321d2;
const PI_1_4: u64 = 0x3fe921fb54442d18;

fn c(bits: u64) -> f64 {
    f64::from_bits(bits)
}

fn sin_poly(r: f64) -> f64 {
    let r2 = r * r;
    let r3 = r * r2;
    let a = r2.mul_add(c(SIN[0]), c(SIN[1]));
    let b = r3.mul_add(c(SIN[2]), r);
    (r2 * r3).mul_add(a, b)
}

fn cos_poly(r: f64) -> f64 {
    let r2 = r * r;
    let a = r2.mul_add(c(COS[0]), c(COS[1]));
    let b = r2.mul_add(c(COS[2]), c(COS[3]));
    let a = (r2 * r2).mul_add(a, b);
    r2.mul_add(a, 1.0)
}

/// sin or cos of r + n * pi/2.
fn quadrant(r: f64, n: i64) -> f32 {
    let sign = f64::from_bits(((n as u64) >> 1) << 63);
    let v = if n & 1 != 0 { cos_poly(r) } else { sin_poly(r) };
    f64::from_bits(v.to_bits() ^ sign.to_bits()) as f32
}

/// sin(pi * t) for t = x / pi + offset, reduced with the table for |x| >= 120.
fn large(x: f64, abs_bits: u32, offset: f64) -> f32 {
    let [hi, lo] = INV_PI[((abs_bits >> 24) - 0x40) as usize];
    let t = x * c(hi) + offset;
    let n = t.round_ties_even();
    let r = (t - n) + x * c(lo);
    let signed = f64::from_bits(r.to_bits() ^ ((n as i64 as u64) << 63));
    let r2 = r * r;
    let a = r2.mul_add(r2 + c(LARGE[0]), c(LARGE[2]));
    let b = r2.mul_add(r2 + c(LARGE[1]), c(LARGE[3]));
    ((signed * a) * (c(LARGE[4]) * b)) as f32
}

pub fn sinf(x: f32) -> f32 {
    let a = x.to_bits() & 0x7fffffff;
    if a >= 0x42f00000 {
        if a >= 0x7f800000 {
            return x - x;
        }
        return large(x as f64, a, 0.0);
    }
    if a < 0x3f490fdb {
        if a < 0x39800000 {
            return ((x as f64) * 67108865.0) as f32 * f32::from_bits(0x32800000);
        }
        return sin_poly(x as f64) as f32;
    }
    let d = x as f64;
    let n = (d * c(TWO_OVER_PI)).round_ties_even();
    quadrant((-n).mul_add(c(PI_OVER_2), d), n as i64)
}

pub fn cosf(x: f32) -> f32 {
    let x = x.abs();
    let a = x.to_bits();
    if a >= 0x42f00000 {
        if a >= 0x4c800000 {
            if a >= 0x7f800000 {
                return x - x;
            }
            return large(x as f64, a, 0.5);
        }
        let d = x as f64;
        let n = (d * c(TWO_OVER_PI)).round_ties_even();
        let r = (-n).mul_add(c(PI_OVER_2), d);
        return quadrant((-n).mul_add(c(PI_OVER_2_LO), r), n as i64 + 1);
    }
    if a < 0x3f490fdb {
        if a < 0x39800000 {
            return (f32::from_bits(0x4c800000) - x) * f32::from_bits(0x32800000);
        }
        return cos_poly(x as f64) as f32;
    }
    let d = x as f64;
    let n = (d * c(TWO_OVER_PI)).round_ties_even();
    quadrant((-n).mul_add(c(PI_OVER_2), d), n as i64 + 1)
}

/// (sin x, cos x), as `__sincosf_stret`.
pub fn sincosf(x: f32) -> (f32, f32) {
    let a = x.to_bits() & 0x7fffffff;
    let (r, n) = if a >= 0x42f00000 {
        if a >= 0x7f800000 {
            return (x - x, x - x);
        }
        let d = x as f64 + x as f64;
        let [hi, lo] = INV_PI[((a >> 24) - 0x40) as usize];
        let t = d * c(hi);
        let n = t.round_ties_even();
        ((t - n) + d * c(lo), n as i64)
    } else if a < 0x39800000 {
        let s = ((x as f64) * 67108865.0) as f32 * f32::from_bits(0x32800000);
        return (s, (f32::from_bits(0x4c800000) - x.abs()) * f32::from_bits(0x32800000));
    } else {
        let t = x as f64 * c(TWO_OVER_PI);
        let n = t.round_ties_even();
        (t - n, n as i64)
    };
    let u = r * r;
    let lane = |i: usize| {
        let p = u.mul_add(u + c(SINCOS[0][i]), c(SINCOS[3][i]));
        let q = u.mul_add(u + c(SINCOS[1][i]), c(SINCOS[4][i]));
        (p, q)
    };
    let ((p0, q0), (p1, q1)) = (lane(0), lane(1));
    let mut s = (((r * c(SINCOS[2][0])) * p0) * q0) as f32;
    let mut co = ((c(SINCOS[2][1]) * p1) * q1) as f32;
    if n & 2 != 0 {
        (s, co) = (-s, -co);
    }
    if n & 1 != 0 { (co, -s) } else { (s, co) }
}

pub fn sin(x: f32) -> f32 {
    sincosf(x).0
}

pub fn cos(x: f32) -> f32 {
    sincosf(x).1
}

/// atan(t) for |t| <= 1.
fn atan_poly(t: f64) -> f64 {
    let t2 = t * t;
    let lane = |i: usize| {
        let p = t2.mul_add(t2 + c(ATAN[0][i]), c(ATAN[1][i]));
        let q = t2.mul_add(t2 + c(ATAN[2][i]), c(ATAN[3][i]));
        p * q
    };
    (lane(0) * c(ATAN_SCALE)) * (lane(1) * t)
}

pub fn atan2f(y: f32, x: f32) -> f32 {
    let r = if y < x {
        if !(-x >= y) {
            atan_poly(y as f64 / x as f64)
        } else if -x == y {
            -c(PI_1_4)
        } else {
            -c(PI_OVER_2) - atan_poly(x as f64 / y as f64)
        }
    } else if y == x {
        if y < 0.0 {
            -c(PI_3_4)
        } else if y != 0.0 {
            c(PI_1_4)
        } else if x.is_sign_negative() {
            c(PI_F32).copysign(if y.is_sign_negative() { -1.0 } else { 1.0 })
        } else {
            return y;
        }
    } else if y.is_nan() || x.is_nan() {
        return y + x;
    } else if !(-x >= y) {
        c(PI_OVER_2) - atan_poly(x as f64 / y as f64)
    } else if -x == y {
        c(PI_3_4)
    } else {
        let t = y as f64 / x as f64;
        let pi = if y.is_sign_negative() { -c(PI) } else { c(PI) };
        if t.abs() < c(ATAN_TINY) {
            c(PI_F32).copysign(pi)
        } else {
            atan_poly(t) + pi
        }
    };
    r as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(x: f32) -> u32 {
        x.to_bits()
    }

    #[test]
    fn apple_results_where_musl_differs() {
        let f = f32::from_bits;
        assert_eq!(bits(sinf(f(0x3c2ed8c7))), 0x3c2ed7ee);
        assert_eq!(bits(sinf(f(0x3d1974bc))), 0x3d196b8c);
        assert_eq!(bits(cosf(f(0x3f33898d))), 0x3f439507);
        assert_eq!(bits(cosf(f(0x3f49edbf))), 0x3f3467c8);
        assert_eq!(bits(sin(f(0x3f77b653))), 0x3f52d399);
        assert_eq!(bits(cos(f(0x3c003009))), 0x3f7ffdfe);
        assert_eq!(bits(cos(f(0x3c8d0a6d))), 0x3f7ff649);
        assert_eq!(bits(atan2f(f(0x3efe4bf7), f(0x401ccf87))), 0x3e4ccccc);
        assert_eq!(bits(atan2f(f(0xc01b967a), f(0xbf1542ef))), 0xbfe73113);
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn matches_the_platform_libm() {
        #[repr(C)]
        struct SinCos {
            sin: f32,
            cos: f32,
        }
        extern "C" {
            fn sinf(x: f32) -> f32;
            fn cosf(x: f32) -> f32;
            fn __sincosf_stret(x: f32) -> SinCos;
            fn atan2f(y: f32, x: f32) -> f32;
        }
        let mut seed = 0x9e3779b9u32;
        for i in (0..=u32::MAX).step_by(4093) {
            let x = f32::from_bits(i);
            let pair = unsafe { __sincosf_stret(x) };
            assert_eq!(bits(super::sinf(x)), bits(unsafe { sinf(x) }), "sinf {i:#x}");
            assert_eq!(bits(super::cosf(x)), bits(unsafe { cosf(x) }), "cosf {i:#x}");
            assert_eq!((bits(sin(x)), bits(cos(x))), (bits(pair.sin), bits(pair.cos)), "sincos {i:#x}");
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let y = f32::from_bits(seed);
            assert_eq!(bits(super::atan2f(y, x)), bits(unsafe { atan2f(y, x) }), "atan2f {seed:#x} {i:#x}");
        }
    }
}
