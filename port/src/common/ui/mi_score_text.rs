//! Port of src/common/ui/MI_ScoreText.cpp

use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_ScoreText {
    pub ui_control: UI_Control,

    iDigitLeftSrcX: i16,
    iDigitMiddleSrcX: i16,
    iDigitRightSrcX: i16,

    iDigitLeftDstX: i16,
    iDigitMiddleDstX: i16,
    iDigitRightDstX: i16,
}
crate::impl_base!(MI_ScoreText => ui_control: UI_Control);

impl MI_ScoreText {
    pub fn new(x: i16, y: i16) -> Self {
        MI_ScoreText {
            ui_control: UI_Control::new(x, y),
            iDigitLeftSrcX: 0,
            iDigitMiddleSrcX: 0,
            iDigitRightSrcX: 0,
            iDigitLeftDstX: 0,
            iDigitMiddleDstX: 0,
            iDigitRightDstX: 0,
        }
    }

    pub fn set_score(&mut self, sScore: i16) {
        let mut iDigits: i16 = sScore;
        while iDigits > 999 {
            iDigits -= 1000;
        }

        self.iDigitLeftSrcX = iDigits / 100 * 16;
        self.iDigitMiddleSrcX = iDigits % 100 / 10 * 16;
        self.iDigitRightSrcX = iDigits % 10 * 16;

        if self.iDigitLeftSrcX == 0 {
            if self.iDigitMiddleSrcX == 0 {
                self.iDigitRightDstX = self.m_pos.x - 8;
            } else {
                self.iDigitMiddleDstX = self.m_pos.x - 16;
                self.iDigitRightDstX = self.m_pos.x;
            }
        } else {
            self.iDigitLeftDstX = self.m_pos.x - 24;
            self.iDigitMiddleDstX = self.m_pos.x - 8;
            self.iDigitRightDstX = self.m_pos.x + 8;
        }
    }
}

impl UI_ControlTrait for MI_ScoreText {
    crate::impl_ctl!();

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        let y = self.m_pos.y as i32;
        unsafe {
            rm.spr_scoretext.draw_src(self.iDigitRightDstX as i32, y, &SDL_Rect { x: self.iDigitRightSrcX as i32, y: 0, w: 16, h: 16 });

            if self.iDigitLeftSrcX > 0 {
                rm.spr_scoretext.draw_src(self.iDigitMiddleDstX as i32, y, &SDL_Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
                rm.spr_scoretext.draw_src(self.iDigitLeftDstX as i32, y, &SDL_Rect { x: self.iDigitLeftSrcX as i32, y: 0, w: 16, h: 16 });
            } else if self.iDigitMiddleSrcX > 0 {
                rm.spr_scoretext.draw_src(self.iDigitMiddleDstX as i32, y, &SDL_Rect { x: self.iDigitMiddleSrcX as i32, y: 0, w: 16, h: 16 });
            }
        }
    }
}
