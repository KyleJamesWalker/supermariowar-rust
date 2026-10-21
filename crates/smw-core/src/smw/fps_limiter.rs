//! Paces the level editor's frames to the tick rate; a no-op in the browser build.

use crate::game::Game;
use crate::globals::*;
use crate::smw::harness;


/// Frame timing in ms of the platform clock.
pub struct FPSLimiter {
    /// Clock time when the current frame started, in ms.
    framestart: u32,
    /// Length of the last frame in ms, at least 1.
    ticks: u32,
    realfps: f32,
    flipfps: f32,
    pub _alias: Aliased,
}

impl FPSLimiter {
    pub const fn new() -> Self {
        FPSLimiter { _alias: Aliased::new(), framestart: 0, ticks: 0, realfps: 0.0, flipfps: 0.0 }
    }

    pub fn instance(g: &mut Game) -> &'static mut FPSLimiter {
        &mut g.ptr().get().fps_limiter
    }

    #[cfg(target_os = "emscripten")]
    pub fn frame_start(&mut self, _g: &mut Game) {}
    #[cfg(target_os = "emscripten")]
    pub fn before_flip(&mut self, _g: &mut Game) {}
    #[cfg(target_os = "emscripten")]
    pub fn after_flip(&mut self, _g: &mut Game) {}

    #[cfg(not(target_os = "emscripten"))]
    pub fn frame_start(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.framestart = crate::services::ticks(gp.get());
    }

    #[cfg(not(target_os = "emscripten"))]
    pub fn before_flip(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.ticks = crate::services::ticks(gp.get()).wrapping_sub(self.framestart);
        if self.ticks == 0 {
            self.ticks = 1;
        }

        if g.game_values.showfps {
            let potentialFps = crate::common::global_constants::frame_rate_cap(g.game_values.framelimiter).unwrap_or(crate::common::global_constants::TICKS_PER_SECOND) as f32;
            // The F1 FPS overlay is drawn by `main::draw_fps_overlay`.
        }
    }

    /// Sleeps until the next tick boundary, scaled by the harness speed; no wait with the harness's no-limit flag.
    #[cfg(not(target_os = "emscripten"))]
    pub fn after_flip(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.flipfps = 1000.0f32 / self.ticks as f32;

        if harness::no_limit(g) {
            return;
        }

        //Sleep for time just under what we need
        let tick_ms = crate::common::global_constants::ms_to_next_tick(self.framestart, self.framestart) as i32;
        let framelimiter = if harness::speed(g) == 1.0 { tick_ms } else { (tick_ms as f32 / harness::speed(g)) as i32 };
        let mut delay: i16 = (framelimiter as u32).wrapping_sub(crate::services::ticks(gp.get())).wrapping_add(self.framestart).wrapping_sub(2) as i16;

        if delay > 0 {
            if delay as i32 > framelimiter {
                delay = framelimiter as i16;
            }

            crate::services::delay(gp.get(), delay as u32);
        }

        //Fine tune wait here
        while crate::services::ticks(gp.get()).wrapping_sub(self.framestart) < (framelimiter as u16) as u32 {
            crate::services::delay(gp.get(), 0);
        }

        self.ticks = crate::services::ticks(gp.get()).wrapping_sub(self.framestart);
        if self.ticks == 0 {
            self.ticks = framelimiter as u32;
        }

        self.realfps = 1000.0f32 / self.ticks as f32;
    }
}
