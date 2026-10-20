//! Map flame cannon that periodically shoots a jet of fire.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::global_constants::ticks;
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
#[derive(Clone)]
pub struct IO_FlameCannon {
    pub cobject: CObject,

    /// Base interval between flames in 62.5 Hz ticks; each wait is one to two times this.
    pub iFreq: i16,
    /// Ticks left waiting while idle, or ticks into the current flame phase.
    pub iTimer: i16,
    /// Unused; reset at each flame start.
    pub iCycle: i16,
    /// Index into the flame frame rectangles for `iDirection`.
    pub iFrame: i16,
    /// Flame direction: 0 right, 1 left, 2 up, 3 down; indexes `g_rFlameRects`.
    pub iDirection: i16,
}
impl_base!(IO_FlameCannon => cobject: CObject);

impl IO_FlameCannon {
    /// `state` cycles 0 idle, 1 igniting, 2 full flame (deadly), 3 dying down.
    pub fn new(g: &mut Game, pos: Vec2s, freq: i16, direction: i16) -> Self {
        let mut this = IO_FlameCannon { cobject: CObject::new(g, SpriteId::NONE, pos), iFreq: freq, iTimer: 0, iCycle: 0, iFrame: 0, iDirection: direction };
        this.objectType = object_flamecannon;

        this.set_new_timer(g);

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

    /// Draws at half scale for the map preview, offset by `iOffsetX`, `iOffsetY` (pixels).
    pub fn draw_offset(&mut self, g: &mut Game, iOffsetX: i16, iOffsetY: i16) {
        let mut gp = g.ptr();
        if self.state > 0 {
            let rect = &g_rFlameRects[self.iDirection as usize][self.iFrame as usize];
            gfx_drawpreview(
                g,
                &gp.rm.sprite(gp.rm.spr_hazard_flame[1]),
                (((self.ix as i32) >> 1) + iOffsetX as i32) as i16,
                (((self.iy as i32) >> 1) + iOffsetY as i32) as i16,
                (rect.x >> 1) as i16,
                (rect.y >> 1) as i16,
                (rect.w >> 1) as i16,
                (rect.h >> 1) as i16,
                &Rect { x: iOffsetX as i32, y: iOffsetY as i32, w: 320, h: 240 },
                true,
                None,
            );
        }
    }

    /// Rolls the next idle wait.
    pub fn set_new_timer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.iTimer = ticks(self.iFreq as i32 + RANDOM_INT(gp.get(), self.iFreq as i32)) as i16;
    }
}

impl CObjectTrait for IO_FlameCannon {
    crate::impl_cobject_plumbing!();

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.state == 0 {
            // No flame, waiting
            self.iTimer -= 1;
            if self.iTimer <= 0 {
                self.iTimer = 0;
                self.iCycle = 0;
                self.iFrame = 0;

                self.state = 1;
                if_sound_on_play(g, &mut gp.rm.sfx_flamecannon);
            }
        } else if self.state == 1 || self.state == 3 {
            // Start or end of flame but not deadly yet
            let phase = ticks(32);
            self.iTimer += 1;
            if self.iTimer as i32 >= phase {
                self.iTimer = 0;
                self.iFrame = 2;

                if self.state == 1 {
                    self.state = 2;
                } else {
                    self.state = 0;
                    self.set_new_timer(g);
                }
            } else {
                self.iFrame = ((self.iTimer as i32 * 32 / phase / 4) % 2) as i16;
            }
        } else if self.state == 2 {
            // Full flame
            let phase = ticks(64);
            self.iTimer += 1;
            if self.iTimer as i32 >= phase {
                self.iTimer = 0;
                self.state = 3;
                self.iFrame = 0;
            } else {
                self.iFrame = (2 + (self.iTimer as i32 * 64 / phase / 4) % 2) as i16;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.state > 0 {
            let rect = &g_rFlameRects[self.iDirection as usize][self.iFrame as usize];
            gp.rm.sprite(gp.rm.spr_hazard_flame[0]).draw_src(g, self.ix as i32, self.iy as i32, rect);
        }
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        if self.state == 2 && !g.player(player).is_invincible() && !g.player(player).is_shielded() && !g.player(player).shyguy {
            return g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        false
    }
}
