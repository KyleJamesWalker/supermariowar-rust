//! The game's random number generator: WELL512, with draw counters and a netplay record/playback tape.

use crate::game::Game;
use crate::globals::{Aliased, Ptr};

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

/// A random integer in `0..rMaxInt` from the game's generator. All game randomness goes through here or `RANDOM_BOOL`.
#[inline]
pub fn RANDOM_INT(g: &mut Game, rMaxInt: i32) -> i32 {
    RandomNumberGenerator::generator(g).get_integer(rMaxInt)
}

/// A fair coin flip from the game's generator.
#[inline]
pub fn RANDOM_BOOL(g: &mut Game) -> bool {
    RandomNumberGenerator::generator(g).get_boolean()
}

/// Integer and boolean draws built on `get_integer_range`.
pub trait RandomNumberGeneratorType {
    /// Seeds the C library `rand`; WELL512 overrides it to seed itself.
    fn reseed(&mut self, seed: u32) {
        unsafe { srand(seed) };
    }

    /// An integer in `min..max`; `max` must exceed `min`.
    fn get_integer_range(&mut self, min: i32, max: i32) -> i32;

    fn get_integer(&mut self, rMax: i32) -> i32 {
        self.get_integer_range(0, rMax)
    }

    /// True with probability one half.
    fn get_boolean(&mut self) -> bool {
        self.get_boolean_scale(2)
    }

    /// True with probability `1 / scaleMax`.
    fn get_boolean_scale(&mut self, scaleMax: i32) -> bool {
        0 == self.get_integer(scaleMax)
    }

    /// True when a draw in `0..scaleMax` exceeds `positiveThreshold`.
    fn get_boolean_threshold(&mut self, scaleMax: i32, positiveThreshold: i32) -> bool {
        debug_assert!(positiveThreshold < scaleMax && positiveThreshold >= 0);
        self.get_integer(scaleMax) > positiveThreshold
    }
}

/// Static access to the generator and counters held in `Game::rng`.
pub struct RandomNumberGenerator;


/// Netplay records the game host's draws and replays them on joiners (smw/net_random.rs).
#[derive(Clone, Default)]
enum Tape {
    #[default]
    Off,
    Record(Vec<u32>),
    Play(Vec<u32>, usize),
}

/// Draw counters, reported in the replay dump's `R` line, and the netplay tape.
#[derive(Clone, Default)]
struct RngCounters {
    g_callCount: u64,
    g_lastValue: u32,
    g_tape: Tape,
}

/// The RNG's part of `Game`.
#[derive(Clone)]
pub struct RngState {
    grng: Option<Well512RandomNumberGenerator>,
    /// The counters until the generator exists; then the generator holds them.
    idle: RngCounters,
}

impl RngState {
    pub const fn new() -> Self {
        RngState { grng: None, idle: RngCounters { g_callCount: 0, g_lastValue: 0, g_tape: Tape::Off } }
    }

    /// The generator's state words and index, which decide every later draw; `None` before it exists.
    pub fn generator_state(&self) -> Option<([u32; 16], u32)> {
        self.grng.as_ref().map(|r| (r.state, r.index))
    }
}

fn rng(g: &mut Game) -> &mut RngCounters {
    let st = &mut g.rng;
    match st.grng.as_mut() {
        Some(r) => r.counters.as_mut().expect("the game's generator counts"),
        None => &mut st.idle,
    }
}

impl RandomNumberGenerator {
    /// How many values the generator has drawn since the last reset; tape playback does not count.
    pub fn call_count(g: &mut Game) -> u64 {
        rng(g).g_callCount
    }

    /// The last value the generator drew.
    pub fn last_value(g: &mut Game) -> u32 {
        rng(g).g_lastValue
    }

    /// Restores the counters from a replay checkpoint (smw/checkpoint.rs).
    pub fn set_counters(g: &mut Game, calls: u64, last: u32) {
        rng(g).g_callCount = calls;
        rng(g).g_lastValue = last;
    }

    pub fn reset_call_count(g: &mut Game) {
        rng(g).g_callCount = 0;
        rng(g).g_lastValue = 0;
    }

    /// Whether netplay is recording or playing back draws.
    pub fn tape_active(g: &mut Game) -> bool {
        !matches!(rng(g).g_tape, Tape::Off)
    }

    /// Records every draw from now on, for sending to joiners.
    pub fn start_recording(g: &mut Game) {
        rng(g).g_tape = Tape::Record(Vec::new())
    }

    /// Returns `values` in order instead of drawing, until they run out.
    pub fn start_playback(g: &mut Game, values: Vec<u32>) {
        rng(g).g_tape = Tape::Play(values, 0)
    }

    /// Ends recording or playback; returns the recorded draws, or the draws playback left unused.
    pub fn stop_tape(g: &mut Game) -> Vec<u32> {
        match std::mem::replace(&mut rng(g).g_tape, Tape::Off) {
            Tape::Off => Vec::new(),
            Tape::Record(values) => values,
            Tape::Play(values, next) => values[next.min(values.len())..].to_vec(),
        }
    }

    /// The game's generator, created on first use and seeded from the wall clock until something reseeds it.
    pub fn generator(g: &mut Game) -> &mut Well512RandomNumberGenerator {
        let st = &mut g.rng;
        if st.grng.is_none() {
            let counters = std::mem::take(&mut st.idle);
            st.grng = Some(Well512RandomNumberGenerator::attached(counters));
        }
        st.grng.as_mut().unwrap()
    }
}

/// A generator on the C library `rand`; the game draws only from WELL512.
pub struct SystemRandomNumberGenerator;

impl RandomNumberGeneratorType for SystemRandomNumberGenerator {
    fn get_integer_range(&mut self, rMin: i32, rMax: i32) -> i32 {
        debug_assert!(rMax > rMin);
        let rVal = ((unsafe { rand() } as f64 / ((RAND_MAX as f32) + 1.0) as f64) * (rMax - rMin) as f64 + rMin as f64) as i32;
        debug_assert!(rVal < rMax && rVal >= rMin);
        rVal
    }
}

/// A WELL512a generator: 16 words of state and an index into them.
#[derive(Clone)]

pub struct Well512RandomNumberGenerator {
    state: [u32; 16],
    index: u32,
    /// The game's counters and tape; none for a generator made on its own.
    counters: Option<RngCounters>,
    _alias: Aliased,
}

impl Well512RandomNumberGenerator {
    pub fn new() -> Self {
        let mut r = Well512RandomNumberGenerator { _alias: Aliased::new(), state: [0; 16], index: 0, counters: None };
        r.initialize();
        r
    }

    /// A generator that counts its draws in `counters` and honors their tape.
    fn attached(counters: RngCounters) -> Self {
        Well512RandomNumberGenerator { counters: Some(counters), ..Self::new() }
    }

    /// The generator state a replay checkpoint (smw/checkpoint.rs) saves and restores.
    pub fn state_mut(&mut self) -> (&mut [u32; 16], &mut u32) {
        (&mut self.state, &mut self.index)
    }

    /// The next value: from the tape in playback, else a fresh draw, recorded when recording.
    fn get_next(&mut self) -> u32 {
        if let Some(counters) = self.counters.as_mut() {
            if let Tape::Play(values, next) = &mut counters.g_tape {
                if let Some(&v) = values.get(*next) {
                    *next += 1;
                    return v;
                }
            }
        }
        let value = self.draw();
        if let Some(counters) = self.counters.as_mut() {
            if let Tape::Record(values) = &mut counters.g_tape {
                values.push(value);
            }
        }
        value
    }

    fn draw(&mut self) -> u32 {
        if let Some(counters) = self.counters.as_mut() {
            counters.g_callCount += 1;
        }
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
        let value = state[index];
        if let Some(counters) = self.counters.as_mut() {
            counters.g_lastValue = value;
        }
        value
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

    /// Seeds the state from `seed` with a linear congruential sequence; same seed, same draws.
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

    // Reference values from tools/ref/rng_ref.cpp.
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
