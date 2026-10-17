//! Port of src/smw/FPSLimiter.cpp

use crate::game::Game;
use crate::globals::*;
use crate::smw::harness;


pub struct FPSLimiter {
    framestart: u32,
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

    // the browser limits the frame rate
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

        // `#if !_DEBUG`: the reference build is Release.
        if g.game_values.showfps {
            let potentialFps = 1000.0f32 / (if g.game_values.framelimiter == 0 { 1 } else { g.game_values.framelimiter }) as f32;
            // TODO(resource_manager): rm->menu_font_large.drawf(0, 480 - rm->menu_font_large.getHeight(),
            //     "Actual:%.1f/%.1f, Flip:%.1f, Potential:%.1f", realfps, potentialFps, flipfps, 1000.0f / (float)ticks);
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    pub fn after_flip(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.flipfps = 1000.0f32 / self.ticks as f32;

        if harness::no_limit(g) {
            return;
        }

        //Sleep for time just under what we need
        let tick_ms = 1000 / crate::common::global_constants::TICKS_PER_SECOND as i32;
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
            crate::services::delay(gp.get(), 0); //keep framerate constant at 1000/game_values.framelimiter fps
        }

        self.ticks = crate::services::ticks(gp.get()).wrapping_sub(self.framestart);
        if self.ticks == 0 {
            self.ticks = framelimiter as u32;
        }

        self.realfps = 1000.0f32 / self.ticks as f32;
    }
}
