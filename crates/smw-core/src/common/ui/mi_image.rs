//! An image menu item, optionally animated, pulsing, swirling or blinking.

use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{cosf, sinf};
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;

/// A sprite region drawn at the control's position.
pub struct MI_Image {
    pub ui_control: UI_Control,

    spr: SpriteId,

    /// Animation frames across and down the sprite, from `isrcx`, `isrcy`.
    iNumXFrames: i16,
    iNumYFrames: i16,
    isrcx: i16,
    isrcy: i16,
    /// Frame size in pixels.
    iw: i16,
    ih: i16,
    /// Ticks per animation frame; 0 for none.
    iSpeed: i16,
    iTimer: i16,
    /// The current frame's source position in pixels.
    iXFrame: i16,
    iYFrame: i16,

    /// Grows and shrinks the image by up to 10 pixels each side.
    fPulse: bool,
    iPulseValue: i16,
    iPulseDelay: i16,
    fPulseOut: bool,

    /// Spirals in to the position: the offset's radius shrinks and its angle turns each tick until the radius reaches 0.
    fSwirl: bool,
    dSwirlRadius: f32,
    dSwirlAngle: f32,
    dSwirlRadiusSpeed: f32,
    dSwirlAngleSpeed: f32,

    /// Toggles visibility every `iBlinkInterval + 1` ticks.
    fBlink: bool,
    iBlinkInterval: i16,
    iBlinkCounter: i16,
    fBlinkShow: bool,
}
crate::impl_base!(MI_Image => ui_control: UI_Control);

impl MI_Image {
    #[allow(clippy::too_many_arguments)]
    /// `w`, `h` is one frame; `speed` is ticks per frame, 0 for a still image.
    pub fn new(nspr: SpriteId, x: i16, y: i16, srcx: i16, srcy: i16, w: i16, h: i16, numxframes: i16, numyframes: i16, speed: i16) -> Self {
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
    /// Shows a new region of the sprite, from its first frame.
    pub fn set_image(&mut self, srcx: i16, srcy: i16, w: i16, h: i16) {
        self.isrcx = srcx;
        self.isrcy = srcy;
        self.iw = w;
        self.ih = h;
        self.iXFrame = srcx;
        self.iYFrame = srcy;
    }
    pub fn set_image_source(&mut self, nspr: SpriteId) {
        self.spr = nspr;
    }

    pub fn set_pulse(&mut self, pulse: bool) {
        self.fPulse = pulse;
    }
    /// Starts or stops a swirl from `radius` pixels at `angle` radians, shrinking and turning by the speeds each tick.
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

    fn update(&mut self, g: &mut Game) {
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

    fn draw(&mut self, g: &mut Game) {
        if !self.m_visible || (self.fBlink && !self.fBlinkShow) {
            return;
        }

        let mut iXOffset: i16 = 0;
        let mut iYOffset: i16 = 0;

        if self.fSwirl {
            iXOffset = (self.dSwirlRadius * cosf(self.dSwirlAngle)) as i16;
            iYOffset = (self.dSwirlRadius * sinf(self.dSwirlAngle)) as i16;
        }

        let src = Rect { x: self.iXFrame as i32, y: self.iYFrame as i32, w: self.iw as i32, h: self.ih as i32 };
        if self.fPulse {
            let pv = self.iPulseValue as i32;
            let dst = Rect {
                x: self.m_pos.x as i32 - pv + iXOffset as i32,
                y: self.m_pos.y as i32 - pv + iYOffset as i32,
                w: self.iw as i32 + (pv << 1),
                h: self.ih as i32 + (pv << 1),
            };
            g.rm.sprite(self.spr).draw_stretch(&src, g.blitdest, &dst);
        } else {
            g.rm.sprite(self.spr).draw_src(g, self.m_pos.x as i32 + iXOffset as i32, self.m_pos.y as i32 + iYOffset as i32, &src);
        }
    }
}
