//! Port of src/smw/objects/IO_FlameCannon.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_drawpreview;
use crate::common::map::g_rFlameRects;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_flamecannon, CObject, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class IO_FlameCannon - shoots a flame
//------------------------------------------------------------------------------
pub struct IO_FlameCannon {
    pub cobject: CObject,

    pub iFreq: i16,
    pub iTimer: i16,
    pub iCycle: i16,
    pub iFrame: i16,
    pub iDirection: i16,
}
impl_base!(IO_FlameCannon => cobject: CObject);

impl IO_FlameCannon {
    pub fn new(pos: Vec2s, freq: i16, direction: i16) -> Self {
        let mut this = IO_FlameCannon { cobject: CObject::new(Ptr::null(), pos), iFreq: freq, iTimer: 0, iCycle: 0, iFrame: 0, iDirection: direction };
        this.objectType = object_flamecannon;

        this.set_new_timer();

        this.iw = g_rFlameRects[direction as usize][0].w as i16;
        this.ih = g_rFlameRects[direction as usize][0].h as i16;

        this.collisionHeight = this.ih;
        this.collisionWidth = this.iw;
        this.collisionOffsetX = 0;
        this.collisionOffsetY = 0;

        if this.iDirection == 1 {
            this.ix -= 64;
        } else if this.iDirection == 2 {
            this.iy -= 64;
        }
        this
    }

    // For preview
    pub fn draw_offset(&mut self, iOffsetX: i16, iOffsetY: i16) {
        if self.state > 0 {
            let rect = &g_rFlameRects[self.iDirection as usize][self.iFrame as usize];
            unsafe {
                gfx_drawpreview(
                    rm.spr_hazard_flame[1].get_surface(),
                    (((self.ix as i32) >> 1) + iOffsetX as i32) as i16,
                    (((self.iy as i32) >> 1) + iOffsetY as i32) as i16,
                    (rect.x >> 1) as i16,
                    (rect.y >> 1) as i16,
                    (rect.w >> 1) as i16,
                    (rect.h >> 1) as i16,
                    iOffsetX,
                    iOffsetY,
                    320,
                    240,
                    true,
                    None,
                );
            }
        }
    }

    fn set_new_timer(&mut self) {
        self.iTimer = (self.iFreq as i32 + RANDOM_INT(self.iFreq as i32)) as i16;
    }
}

impl CObjectTrait for IO_FlameCannon {
    crate::impl_cobject_plumbing!();

    fn update(&mut self) {
        if self.state == 0 {
            // No flame, waiting
            self.iTimer -= 1;
            if self.iTimer <= 0 {
                self.iTimer = 0;
                self.iCycle = 0;
                self.iFrame = 0;

                self.state = 1;
                unsafe {
                    if_sound_on_play(&mut rm.sfx_flamecannon);
                }
            }
        } else if self.state == 1 || self.state == 3 {
            // Start or end of flame but not deadly yet
            self.iTimer += 1;
            if self.iTimer >= 4 {
                self.iTimer = 0;

                self.iFrame += 1;
                if self.iFrame > 1 {
                    self.iFrame = 0;

                    self.iCycle += 1;
                    if self.iCycle >= 4 {
                        self.iFrame = 2;
                        self.iCycle = 0;

                        if self.state == 1 {
                            self.state = 2;
                        } else {
                            self.state = 0;
                            self.set_new_timer();
                        }
                    }
                }
            }
        } else if self.state == 2 {
            // Full flame
            self.iTimer += 1;
            if self.iTimer >= 4 {
                self.iTimer = 0;

                self.iFrame += 1;
                if self.iFrame > 3 {
                    self.iFrame = 2;

                    self.iCycle += 1;
                    if self.iCycle >= 8 {
                        self.state = 3;
                        self.iFrame = 0;
                        self.iCycle = 0;
                    }
                }
            }
        }
    }

    fn draw(&mut self) {
        if self.state > 0 {
            let rect = &g_rFlameRects[self.iDirection as usize][self.iFrame as usize];
            unsafe {
                rm.spr_hazard_flame[0].draw_src(self.ix as i32, self.iy as i32, rect);
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.state == 2 && !player.is_invincible() && !player.is_shielded() && !player.shyguy {
            return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        false
    }
}
