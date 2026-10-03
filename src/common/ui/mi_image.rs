//! Port of src/common/ui/MI_Image.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_Image {
    pub ui_control: UI_Control,

    spr: Ptr<gfxSprite>,

    iNumXFrames: i16,
    iNumYFrames: i16,
    isrcx: i16,
    isrcy: i16,
    iw: i16,
    ih: i16,
    iSpeed: i16,
    iTimer: i16,
    iXFrame: i16,
    iYFrame: i16,

    fPulse: bool,
    iPulseValue: i16,
    iPulseDelay: i16,
    fPulseOut: bool,

    fSwirl: bool,
    dSwirlRadius: f32,
    dSwirlAngle: f32,
    dSwirlRadiusSpeed: f32,
    dSwirlAngleSpeed: f32,

    fBlink: bool,
    iBlinkInterval: i16,
    iBlinkCounter: i16,
    fBlinkShow: bool,
}
crate::impl_base!(MI_Image => ui_control: UI_Control);

impl MI_Image {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, srcx: i16, srcy: i16, w: i16, h: i16, numxframes: i16, numyframes: i16, speed: i16) -> Self {
        MI_Image {
            ui_control: UI_Control::new(x, y),
            spr: nspr,
            iNumXFrames: numxframes,
            iNumYFrames: numyframes,
            isrcx: srcx,
            isrcy: srcy,
            iw: w,
            ih: h,
            iSpeed: speed,
            iTimer: 0,
            iXFrame: srcx,
            iYFrame: srcy,
            fPulse: false,
            iPulseValue: 0,
            iPulseDelay: 0,
            fPulseOut: true,
            fSwirl: false,
            dSwirlRadius: 0.0,
            dSwirlAngle: 0.0,
            dSwirlRadiusSpeed: 0.0,
            dSwirlAngleSpeed: 0.0,
            fBlink: false,
            iBlinkInterval: 0,
            iBlinkCounter: 0,
            fBlinkShow: true,
        }
    }

    pub fn set_animation_speed(&mut self, speed: i16) {
        self.iSpeed = speed;
    }
    pub fn set_image(&mut self, srcx: i16, srcy: i16, w: i16, h: i16) {
        self.isrcx = srcx;
        self.isrcy = srcy;
        self.iw = w;
        self.ih = h;
        self.iXFrame = srcx;
        self.iYFrame = srcy;
    }
    pub fn set_image_source(&mut self, nspr: Ptr<gfxSprite>) {
        self.spr = nspr;
    }

    pub fn set_pulse(&mut self, pulse: bool) {
        self.fPulse = pulse;
    }
    pub fn set_swirl(&mut self, swirl: bool, radius: f32, angle: f32, radiusSpeed: f32, angleSpeed: f32) {
        self.fSwirl = swirl;
        self.dSwirlRadius = radius;
        self.dSwirlAngle = angle;
        self.dSwirlRadiusSpeed = radiusSpeed;
        self.dSwirlAngleSpeed = angleSpeed;
    }
    pub fn stop_swirl(&mut self) {
        self.set_swirl(false, 0.0, 0.0, 0.0, 0.0);
    }
    pub fn set_blink(&mut self, blink: bool, interval: i16) {
        self.fBlink = blink;
        self.iBlinkInterval = interval;
    }

    pub fn is_swirling(&self) -> bool {
        self.fSwirl
    }

    pub fn get_position_and_size(&self, x: &mut i16, y: &mut i16, w: &mut i16, h: &mut i16) {
        *x = self.m_pos.x;
        *y = self.m_pos.y;
        *w = self.iw;
        *h = self.ih;
    }
}

impl UI_ControlTrait for MI_Image {
    crate::impl_ctl!();

    fn update(&mut self) {
        if !self.m_visible {
            return;
        }

        if self.iSpeed > 0 && {
            self.iTimer += 1;
            self.iTimer >= self.iSpeed
        } {
            self.iTimer = 0;
            self.iXFrame += self.iw;

            if self.iXFrame as i32 >= self.iNumXFrames as i32 * self.iw as i32 + self.isrcx as i32 {
                self.iXFrame = self.isrcx;
                self.iYFrame += self.ih;

                if self.iYFrame as i32 >= self.iNumYFrames as i32 * self.ih as i32 + self.isrcy as i32 {
                    self.iYFrame = self.isrcy;
                }
            }
        }

        if self.fPulse {
            self.iPulseDelay += 1;
            if self.iPulseDelay >= 3 {
                self.iPulseDelay = 0;

                if self.fPulseOut {
                    self.iPulseValue += 1;
                    if self.iPulseValue >= 10 {
                        self.fPulseOut = false;
                    }
                } else {
                    self.iPulseValue -= 1;
                    if self.iPulseValue <= 0 {
                        self.fPulseOut = true;
                    }
                }
            }
        }

        if self.fSwirl {
            self.dSwirlRadius -= self.dSwirlRadiusSpeed;

            if self.dSwirlRadius <= 0.0 {
                self.fSwirl = false;
            } else {
                self.dSwirlAngle += self.dSwirlAngleSpeed;
            }
        }

        if self.fBlink {
            self.iBlinkCounter += 1;
            if self.iBlinkCounter > self.iBlinkInterval {
                self.fBlinkShow = !self.fBlinkShow;
                self.iBlinkCounter = 0;
            }
        }
    }

    fn draw(&mut self) {
        if !self.m_visible || (self.fBlink && !self.fBlinkShow) {
            return;
        }

        let mut iXOffset: i16 = 0;
        let mut iYOffset: i16 = 0;

        if self.fSwirl {
            iXOffset = (self.dSwirlRadius * self.dSwirlAngle.cos()) as i16;
            iYOffset = (self.dSwirlRadius * self.dSwirlAngle.sin()) as i16;
        }

        let src = SDL_Rect { x: self.iXFrame as i32, y: self.iYFrame as i32, w: self.iw as i32, h: self.ih as i32 };
        if self.fPulse {
            let pv = self.iPulseValue as i32;
            let dst = SDL_Rect {
                x: self.m_pos.x as i32 - pv + iXOffset as i32,
                y: self.m_pos.y as i32 - pv + iYOffset as i32,
                w: self.iw as i32 + (pv << 1),
                h: self.ih as i32 + (pv << 1),
            };
            self.spr.draw_stretch(&src, unsafe { blitdest }, &dst);
        } else {
            self.spr.draw_src(self.m_pos.x as i32 + iXOffset as i32, self.m_pos.y as i32 + iYOffset as i32, &src);
        }
    }
}
