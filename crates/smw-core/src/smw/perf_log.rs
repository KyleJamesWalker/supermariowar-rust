//! With `SMW_PERF=1`, logs how long each frame's work takes every five seconds. It reads the wall clock only for the
//! log, so it never changes the game.

use crate::common::global_constants::TICKS_PER_SECOND;
use std::time::Instant;

/// One tick of wall time.
const BUDGET_US: u64 = 1_000_000 / TICKS_PER_SECOND as u64;
/// Frames per report: five seconds of ticks.
const REPORT_FRAMES: u32 = 300;

/// Frame time totals for the current report, in microseconds.
#[derive(Clone, Default)]
pub struct PerfLog {
    frames: u32,
    /// Tick time since the last present, which loop iterations that do not present also add to.
    pending_update_us: u64,
    update_us: u64,
    render_us: u64,
    flip_us: u64,
    /// The backend's split of `flip_us`, and how many frames reported one.
    upload_us: u64,
    swap_us: u64,
    split_frames: u32,
    worst_us: u64,
    over_budget: u32,
}

impl PerfLog {
    /// A log when `SMW_PERF` is set to anything but 0.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("SMW_PERF").is_some_and(|v| v != "0").then(PerfLog::default)
    }

    /// Adds time spent ticking, counted into the next present's update and draw.
    pub fn tick_time(&mut self, d: std::time::Duration) {
        self.pending_update_us += d.as_micros() as u64;
    }

    /// Adds one present's times (the ticks since the last one, interpolated render, then present, with the backend's
    /// upload and swap split of the present when it has one) and prints a report every `REPORT_FRAMES`.
    pub fn record(&mut self, render_start: Instant, render_end: Instant, flip_end: Instant, split: Option<(u64, u64)>) {
        let update = std::mem::take(&mut self.pending_update_us);
        let render = (render_end - render_start).as_micros() as u64;
        let flip = (flip_end - render_end).as_micros() as u64;
        let total = update + render + flip;
        self.frames += 1;
        self.update_us += update;
        self.render_us += render;
        self.flip_us += flip;
        if let Some((upload, swap)) = split {
            self.upload_us += upload;
            self.swap_us += swap;
            self.split_frames += 1;
        }
        self.worst_us = self.worst_us.max(total);
        if total > BUDGET_US {
            self.over_budget += 1;
        }
        if self.frames == REPORT_FRAMES {
            let n = self.frames as u64;
            let ms = |us: u64, n: u64| us as f64 / n as f64 / 1000.0;
            let avg = (self.update_us + self.render_us + self.flip_us) / n;
            let split = match self.split_frames as u64 {
                0 => String::new(),
                k => format!(" (upload {:.2} ms, draw and swap {:.2} ms)", ms(self.upload_us, k), ms(self.swap_us, k)),
            };
            println!(
                "[perf] {} frames: update+draw {:.2} ms, render {:.2} ms, present {:.2} ms{}, total {:.2} ms ({}% of the {:.2} ms tick), worst {:.2} ms, {} over budget",
                self.frames,
                ms(self.update_us, n),
                ms(self.render_us, n),
                ms(self.flip_us, n),
                split,
                avg as f64 / 1000.0,
                avg * 100 / BUDGET_US,
                BUDGET_US as f64 / 1000.0,
                self.worst_us as f64 / 1000.0,
                self.over_budget
            );
            *self = PerfLog::default();
        }
    }
}
