//! Port of src/common/RandomNumberGenerator.cpp

use crate::globals::Aliased;

extern "C" {
    fn srand(seed: u32);
    fn rand() -> i32;
    #[cfg_attr(windows, link_name = "_time64")]
    fn time(t: *mut i64) -> i64;
}

#[cfg(not(windows))]
const RAND_MAX: i32 = 0x7fffffff;
#[cfg(windows)]
const RAND_MAX: i32 = 0x7fff;

#[inline]
pub fn RANDOM_INT(rMaxInt: i32) -> i32 {
    RandomNumberGenerator::generator().get_integer(rMaxInt)
}

#[inline]
pub fn RANDOM_BOOL() -> bool {
    RandomNumberGenerator::generator().get_boolean()
}

pub trait RandomNumberGeneratorType {
    fn reseed(&mut self, seed: u32) {
        unsafe { srand(seed) };
    }

    fn get_integer_range(&mut self, min: i32, max: i32) -> i32;

    fn get_integer(&mut self, rMax: i32) -> i32 {
        self.get_integer_range(0, rMax)
    }

    fn get_boolean(&mut self) -> bool {
        self.get_boolean_scale(2)
    }

    fn get_boolean_scale(&mut self, scaleMax: i32) -> bool {
        0 == self.get_integer(scaleMax)
    }

    fn get_boolean_threshold(&mut self, scaleMax: i32, positiveThreshold: i32) -> bool {
        debug_assert!(positiveThreshold < scaleMax && positiveThreshold >= 0);
        self.get_integer(scaleMax) > positiveThreshold
    }
}

pub struct RandomNumberGenerator;

static mut grng: Option<Well512RandomNumberGenerator> = None;

// Replay-harness counters (tools/cpp-harness.patch), reported in the dump's `R` line.
static mut g_callCount: u64 = 0;
static mut g_lastValue: u32 = 0;

/// Not in the C++: netplay records the game host's draws and replays them on joiners (smw/net_random.rs).
enum Tape {
    Off,
    Record(Vec<u32>),
    Play(Vec<u32>, usize),
}

static mut g_tape: Tape = Tape::Off;

impl RandomNumberGenerator {
    pub fn call_count() -> u64 {
        unsafe { g_callCount }
    }

    pub fn last_value() -> u32 {
        unsafe { g_lastValue }
    }

    /// Not in the C++: a replay checkpoint (smw/checkpoint.rs) restores the counters.
    pub fn set_counters(calls: u64, last: u32) {
        unsafe {
            g_callCount = calls;
            g_lastValue = last;
        }
    }

    pub fn reset_call_count() {
        unsafe {
            g_callCount = 0;
            g_lastValue = 0;
        }
    }

    pub fn tape_active() -> bool {
        unsafe { !matches!(g_tape, Tape::Off) }
    }

    pub fn start_recording() {
        unsafe { g_tape = Tape::Record(Vec::new()) }
    }

    pub fn start_playback(values: Vec<u32>) {
        unsafe { g_tape = Tape::Play(values, 0) }
    }

    /// Ends recording or playback; returns the recorded draws, or the draws playback left unused.
    pub fn stop_tape() -> Vec<u32> {
        match unsafe { std::mem::replace(&mut g_tape, Tape::Off) } {
            Tape::Off => Vec::new(),
            Tape::Record(values) => values,
            Tape::Play(values, next) => values[next.min(values.len())..].to_vec(),
        }
    }

    pub fn generator() -> &'static mut Well512RandomNumberGenerator {
        unsafe {
            if grng.is_none() {
                grng = Some(Well512RandomNumberGenerator::new());
            }
            grng.as_mut().unwrap()
        }
    }
}

pub struct SystemRandomNumberGenerator;

impl RandomNumberGeneratorType for SystemRandomNumberGenerator {
    fn get_integer_range(&mut self, rMin: i32, rMax: i32) -> i32 {
        debug_assert!(rMax > rMin);
        let rVal = ((unsafe { rand() } as f64 / ((RAND_MAX as f32) + 1.0) as f64) * (rMax - rMin) as f64 + rMin as f64) as i32;
        debug_assert!(rVal < rMax && rVal >= rMin);
        rVal
    }
}

pub struct Well512RandomNumberGenerator {
    state: [u32; 16],
    index: u32,
    _alias: Aliased,
}

impl Well512RandomNumberGenerator {
    pub fn new() -> Self {
        let mut r = Well512RandomNumberGenerator { _alias: Aliased::new(), state: [0; 16], index: 0 };
        r.initialize();
        r
    }

    /// Not in the C++: the generator state a replay checkpoint (smw/checkpoint.rs) saves and restores.
    pub fn state_mut(&mut self) -> (&mut [u32; 16], &mut u32) {
        (&mut self.state, &mut self.index)
    }

    fn get_next(&mut self) -> u32 {
        unsafe {
            if let Tape::Play(values, next) = &mut g_tape {
                if let Some(&v) = values.get(*next) {
                    *next += 1;
                    return v;
                }
            }
        }
        let value = self.draw();
        unsafe {
            if let Tape::Record(values) = &mut g_tape {
                values.push(value);
            }
        }
        value
    }

    fn draw(&mut self) -> u32 {
        unsafe { g_callCount += 1 };
        let state = &mut self.state;
        let index = self.index as usize;
        let mut a = state[index];
        let mut c = state[(index + 13) & 15];
        let b = a ^ c ^ (a << 16) ^ (c << 15);
        c = state[(index + 9) & 15];
        c ^= c >> 11;
        state[index] = b ^ c;
        a = state[index];
        let d = a ^ ((a << 5) & 0xDA442D20u32);
        let index = (index + 15) & 15;
        self.index = index as u32;
        a = state[index];
        state[index] = a ^ b ^ d ^ (a << 2) ^ (b << 18) ^ (c << 28);
        unsafe { g_lastValue = state[index] };
        state[index]
    }

    fn initialize(&mut self) {
        self.index = 0;
        unsafe {
            srand(time(std::ptr::null_mut()) as u32);
            for i in 0..16 {
                self.state[i] = rand() as u32;
            }
        }
    }

    fn initialize_seed(&mut self, seed: u32) {
        self.index = 0;
        self.state[0] = seed;
        for i in 1..16 {
            self.state[i] = (self.state[i - 1].wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
        }
    }
}

impl RandomNumberGeneratorType for Well512RandomNumberGenerator {
    fn reseed(&mut self, seed: u32) {
        self.initialize_seed(seed);
    }

    fn get_integer_range(&mut self, rMin: i32, rMax: i32) -> i32 {
        debug_assert!(rMax > rMin);
        let rVal = ((self.get_next() as f64 / ((u32::MAX as f32) + 1.0) as f64) * (rMax.wrapping_sub(rMin)) as f64 + rMin as f64) as i32;
        debug_assert!(rVal < rMax && rVal >= rMin);
        rVal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // From tools/ref/rng_ref.cpp compiled against the original RandomNumberGenerator.cpp.
    const SEED0_INT: [i32; 20] = [95, 41, 394, 147, 814, 201, 977, 648, 77, 831, 466, 169, 864, 670, 353, 492, 118, 317, 884, 166];
    const SEED0_MIXED: [i32; 30] = [
        -4, 2, 11, 4, 3, 4, 2, -1, 7, 5, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1008535754, 1566625861, 1380596918, 1977272488, 1365018658,
        1980176259, 724222123, 1554996849, 2020156719, 1941058675,
    ];
    const SEED12345_INT: [i32; 20] = [542, 998, 431, 394, 147, 814, 201, 977, 648, 735, 878, 8, 645, 922, 908, 109, 842, 574, 440, 437];
    const SEED12345_MIXED: [i32; 30] = [
        -2, 3, 6, -6, 0, 7, -3, 7, -5, -1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 618113728, 533812650, 1507440305, 654964824, 1588773747,
        945494861, 1408804606, 1799773601, 988418264, 540755907,
    ];

    fn run(seed: u32) -> (Vec<i32>, Vec<i32>) {
        let mut g = Well512RandomNumberGenerator::new();
        g.reseed(seed);
        let ints = (0..20).map(|_| g.get_integer(1000)).collect();
        let mut mixed: Vec<i32> = (0..10).map(|_| g.get_integer_range(-7, 13)).collect();
        mixed.extend((0..10).map(|_| g.get_boolean() as i32));
        mixed.extend((0..10).map(|_| g.get_integer(2147483647)));
        (ints, mixed)
    }

    #[test]
    fn well512_matches_cpp() {
        assert_eq!(run(0), (SEED0_INT.to_vec(), SEED0_MIXED.to_vec()));
        assert_eq!(run(12345), (SEED12345_INT.to_vec(), SEED12345_MIXED.to_vec()));
    }
}
