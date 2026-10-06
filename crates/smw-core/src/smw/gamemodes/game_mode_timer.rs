//! Port of src/smw/gamemodes/GameModeTimer.cpp

use crate::common::eyecandy_styles::ScoreboardStyle;
use crate::common::global_constants::WAITTIME;
use crate::globals::*;
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct GameTimerDisplay {
    pub timeleft: i16,
    pub countdown: bool,

    pub framesleft_persecond: i16,
    pub iDigitLeftSrcX: i16,
    pub iDigitMiddleSrcX: i16,
    pub iDigitRightSrcX: i16,
    pub iDigitLeftDstX: i16,
    pub iDigitMiddleDstX: i16,
    pub iDigitRightDstX: i16,
    pub iScoreOffsetX: i16,

    pub iFramesPerSecond: i16,
    pub _alias: Aliased,
}

impl GameTimerDisplay {
    pub fn new() -> Self {
        GameTimerDisplay { iFramesPerSecond: (1000 / WAITTIME) as i16, ..Default::default() }
    }

    pub fn init(&mut self, iTime: i16, fCountDown: bool) {
        self.timeleft = iTime;
        self.countdown = fCountDown;

        self.set_digit_counters();
        self.framesleft_persecond = self.iFramesPerSecond;

        unsafe {
            if game_values.scoreboardstyle == ScoreboardStyle::Top {
                self.iScoreOffsetX = 5;
            } else {
                self.iScoreOffsetX = 291;
            }
        }

        self.set_digit_counters();
        self.framesleft_persecond = self.iFramesPerSecond;
    }

    pub fn run_clock(&mut self) -> i16 {
        if self.timeleft > 0 || !self.countdown {
            self.framesleft_persecond -= 1;
            if self.framesleft_persecond < 1 {
                self.framesleft_persecond = self.iFramesPerSecond;

                if self.countdown {
                    self.timeleft -= 1;
                } else {
                    self.timeleft += 1;
                }

                self.set_digit_counters();

                return self.timeleft;
            }
        }

        -1
    }

    pub fn draw(&mut self) {
        unsafe {
            rm.spr_timershade.draw(self.iScoreOffsetX as i32, 5);
            rm.spr_scoretext.draw_src(self.iDigitRightDstX as i32, 13, &SDL_Rect { x: self.iDigitRightSrcX as i32, y: 0, w: 16, h: 16 });

            if self.iDigitLeftSrcX > 0 {
                rm.spr_scoretext.draw_src(self.iDigitMiddleDstX as i32, 13, &SDL_Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
                rm.spr_scoretext.draw_src(self.iDigitLeftDstX as i32, 13, &SDL_Rect { x: self.iDigitLeftSrcX as i32, y: 0, w: 16, h: 16 });
            } else if self.iDigitMiddleSrcX > 0 {
                rm.spr_scoretext.draw_src(self.iDigitMiddleDstX as i32, 13, &SDL_Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
            }
        }
    }

    fn set_digit_counters(&mut self) {
        let mut iDigits: i16 = self.timeleft;
        while iDigits > 999 {
            iDigits -= 1000;
        }

        self.iDigitLeftSrcX = iDigits / 100 * 16;
        self.iDigitMiddleSrcX = iDigits % 100 / 10 * 16;
        self.iDigitRightSrcX = iDigits % 10 * 16;

        if self.iDigitLeftSrcX == 0 {
            if self.iDigitMiddleSrcX == 0 {
                self.iDigitRightDstX = self.iScoreOffsetX + 21;
            } else {
                self.iDigitMiddleDstX = self.iScoreOffsetX + 12;
                self.iDigitRightDstX = self.iScoreOffsetX + 30;
            }
        } else {
            self.iDigitLeftDstX = self.iScoreOffsetX + 3;
            self.iDigitMiddleDstX = self.iScoreOffsetX + 21;
            self.iDigitRightDstX = self.iScoreOffsetX + 39;
        }
    }

    pub fn set_time(&mut self, iTime: i16) {
        self.timeleft = iTime;
        self.set_digit_counters();
    }

    pub fn add_time(&mut self, iTime: i16) {
        self.timeleft += iTime;
        self.set_digit_counters();
    }
}
