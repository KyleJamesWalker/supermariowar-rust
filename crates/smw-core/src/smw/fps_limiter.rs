//! Port of src/smw/FPSLimiter.cpp

use crate::globals::*;
use crate::smw::harness;


pub struct FPSLimiter {
    framestart: u32,
    ticks: u32,
    realfps: f32,
    flipfps: f32,
    pub _alias: Aliased,
}

static mut instance: FPSLimiter = FPSLimiter { _alias: Aliased::new(), framestart: 0, ticks: 0, realfps: 0.0, flipfps: 0.0 };

impl FPSLimiter {
    pub fn instance() -> &'static mut FPSLimiter {
        unsafe { &mut instance }
    }

    // the browser limits the frame rate
    #[cfg(target_os = "emscripten")]
    pub fn frame_start(&mut self) {}
    #[cfg(target_os = "emscripten")]
    pub fn before_flip(&mut self) {}
    #[cfg(target_os = "emscripten")]
    pub fn after_flip(&mut self) {}

    #[cfg(not(target_os = "emscripten"))]
    pub fn frame_start(&mut self) {
        self.framestart = crate::services::ticks();
    }

    #[cfg(not(target_os = "emscripten"))]
    pub fn before_flip(&mut self) {
        unsafe {
            self.ticks = crate::services::ticks().wrapping_sub(self.framestart);
            if self.ticks == 0 {
                self.ticks = 1;
            }

            // `#if !_DEBUG`: the reference build is Release.
            if game_values.showfps {
                let potentialFps = 1000.0f32 / (if game_values.framelimiter == 0 { 1 } else { game_values.framelimiter }) as f32;
                // TODO(resource_manager): rm->menu_font_large.drawf(0, 480 - rm->menu_font_large.getHeight(),
                //     "Actual:%.1f/%.1f, Flip:%.1f, Potential:%.1f", realfps, potentialFps, flipfps, 1000.0f / (float)ticks);
            }
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    pub fn after_flip(&mut self) {
        unsafe {
            self.flipfps = 1000.0f32 / self.ticks as f32;

            if harness::no_limit() {
                return;
            }

            //Sleep for time just under what we need
            let framelimiter = if harness::speed() == 1.0 { game_values.framelimiter as i32 } else { (game_values.framelimiter as f32 / harness::speed()) as i32 };
            let mut delay: i16 = (framelimiter as u32).wrapping_sub(crate::services::ticks()).wrapping_add(self.framestart).wrapping_sub(2) as i16;

            if delay > 0 {
                if delay as i32 > framelimiter {
                    delay = framelimiter as i16;
                }

                crate::services::delay(delay as u32);
            }

            //Fine tune wait here
            while crate::services::ticks().wrapping_sub(self.framestart) < (framelimiter as u16) as u32 {
                crate::services::delay(0); //keep framerate constant at 1000/game_values.framelimiter fps
            }

            self.ticks = crate::services::ticks().wrapping_sub(self.framestart);
            if self.ticks == 0 {
                self.ticks = framelimiter as u32;
            }

            self.realfps = 1000.0f32 / self.ticks as f32;
        }
    }
}
