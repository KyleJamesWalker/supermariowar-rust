//! Not in the C++: with `SMW_PERF=1`, the game logs how long each frame's work takes every five seconds, to measure
//! the headroom on slow hardware. It reads the wall clock only for the log, so it never changes the game.

use std::time::Instant;

/// The frame limiter's frame time.
const BUDGET_US: u64 = 16_000;
const REPORT_FRAMES: u32 = 300;

#[derive(Clone)]
pub struct PerfLog {
    frames: u32,
    update_us: u64,
    flip_us: u64,
    worst_us: u64,
    over_budget: u32,
}

impl PerfLog {
    pub fn from_env() -> Option<Self> {
        std::env::var_os("SMW_PERF")
            .is_some_and(|v| v != "0")
            .then_some(PerfLog { frames: 0, update_us: 0, flip_us: 0, worst_us: 0, over_budget: 0 })
    }

    pub fn record(&mut self, frame_start: Instant, update_end: Instant, flip_end: Instant) {
        let update = (update_end - frame_start).as_micros() as u64;
        let flip = (flip_end - update_end).as_micros() as u64;
        self.frames += 1;
        self.update_us += update;
        self.flip_us += flip;
        self.worst_us = self.worst_us.max(update + flip);
        if update + flip > BUDGET_US {
            self.over_budget += 1;
        }
        if self.frames == REPORT_FRAMES {
            let n = self.frames as u64;
            let avg = (self.update_us + self.flip_us) / n;
            println!(
                "[perf] {} frames: update+draw {:.2} ms, present {:.2} ms, total {:.2} ms ({}% of the 16 ms budget), worst {:.2} ms, {} over budget",
                self.frames,
                self.update_us as f64 / n as f64 / 1000.0,
                self.flip_us as f64 / n as f64 / 1000.0,
                avg as f64 / 1000.0,
                avg * 100 / BUDGET_US,
                self.worst_us as f64 / 1000.0,
                self.over_budget
            );
            *self = PerfLog { frames: 0, update_us: 0, flip_us: 0, worst_us: 0, over_budget: 0 };
        }
    }
}
