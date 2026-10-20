//! On-screen match clock used by timed game modes.

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy_styles::ScoreboardStyle;
use crate::common::global_constants::TICKS_PER_SECOND;
use crate::globals::*;

/// Match clock in whole seconds, with the HUD digit layout for drawing it.
#[derive(Clone, Default)]
pub struct GameTimerDisplay {
    /// Seconds shown on the clock; remaining time when counting down, elapsed time otherwise.
    pub timeleft: i16,
    /// Counts down to zero when true, up without limit when false.
    pub countdown: bool,

    /// Ticks left until the clock changes by one second.
    pub framesleft_persecond: i16,
    /// Source x of each digit in the score digit strip, 16 px per digit; zero means digit 0.
    pub iDigitLeftSrcX: i16,
    pub iDigitMiddleSrcX: i16,
    pub iDigitRightSrcX: i16,
    /// Screen x of each drawn digit, recomputed with the source x so leading zeros are skipped.
    pub iDigitLeftDstX: i16,
    pub iDigitMiddleDstX: i16,
    pub iDigitRightDstX: i16,
    /// Screen x of the clock panel, set by the scoreboard style.
    pub iScoreOffsetX: i16,

    /// Ticks per clock second.
    pub iFramesPerSecond: i16,
    pub _alias: Aliased,
}

impl GameTimerDisplay {
    /// The clock's simulated state for `CGameModeTrait::harness_hash`; the digit fields only draw it.
    pub fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        h.i16s(&[self.timeleft, self.framesleft_persecond, self.iFramesPerSecond]);
        h.bools(&[self.countdown]);
    }

    pub fn new() -> Self {
        GameTimerDisplay { iFramesPerSecond: TICKS_PER_SECOND as i16, ..Default::default() }
    }

    /// Starts the clock at `iTime` seconds and places it for the current scoreboard style.
    pub fn init(&mut self, g: &mut Game, iTime: i16, fCountDown: bool) {
        self.timeleft = iTime;
        self.countdown = fCountDown;

        self.set_digit_counters();
        self.framesleft_persecond = self.iFramesPerSecond;

        if g.game_values.scoreboardstyle == ScoreboardStyle::Top {
            self.iScoreOffsetX = 5;
        } else {
            self.iScoreOffsetX = 291;
        }

        self.set_digit_counters();
        self.framesleft_persecond = self.iFramesPerSecond;
    }

    /// Advances one tick. Returns the new time when a second elapses, otherwise -1.
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

    pub fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gp.rm.sprite(gp.rm.spr_timershade).draw(g, self.iScoreOffsetX as i32, 5);
        gp.rm.sprite(gp.rm.spr_scoretext).draw_src(g, self.iDigitRightDstX as i32, 13, &Rect { x: self.iDigitRightSrcX as i32, y: 0, w: 16, h: 16 });

        if self.iDigitLeftSrcX > 0 {
            gp.rm.sprite(gp.rm.spr_scoretext).draw_src(g, self.iDigitMiddleDstX as i32, 13, &Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
            gp.rm.sprite(gp.rm.spr_scoretext).draw_src(g, self.iDigitLeftDstX as i32, 13, &Rect { x: self.iDigitLeftSrcX as i32, y: 0, w: 16, h: 16 });
        } else if self.iDigitMiddleSrcX > 0 {
            gp.rm.sprite(gp.rm.spr_scoretext).draw_src(g, self.iDigitMiddleDstX as i32, 13, &Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
        }
    }

    /// Recomputes digit sprites and positions; only the last three digits are shown.
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
