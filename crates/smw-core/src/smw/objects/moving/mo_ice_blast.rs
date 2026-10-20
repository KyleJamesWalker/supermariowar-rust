//! Ice wand blast: flies straight and freezes the first enemy or opposing player it hits.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::global_constants::ticks;
use crate::common::eyecandy::SpotlightId;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class Ice Blast
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct MO_IceBlast {
    pub io_moving_object: IO_MovingObject,

    /// Ticks left before the blast disappears.
    pub ttl: i16,
    /// Spotlight that follows the blast while spotlights are on.
    pub sSpotlight: SpotlightId,
}
impl_base!(MO_IceBlast => io_moving_object: IO_MovingObject);

impl MO_IceBlast {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, fVelyX: f32, iGlobalID: i16, teamID: i16, iColorID: i16) -> Self {
        let mut o = MO_IceBlast {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, 4, 8, 32, 32, 0, 0, 0, ((iColorID as i32 + 1) << 5) as i16, 32, 32),
            ttl: 0,
            sSpotlight: SpotlightId::NONE,
        };

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;

        o.movingObjectType = movingobject_iceblast;

        o.state = 1;

        o.velx = fVelyX;
        o.vely = 0.0f32;

        if o.velx > 0.0f32 {
            o.drawframe = 0;
        } else {
            o.drawframe = o.animationWidth - o.iw;
        }

        o.ttl = ticks(120) as i16;

        o.fObjectCollidesWithMap = false;

        o.sSpotlight = SpotlightId::NONE;
        o
    }
}

impl CObjectTrait for MO_IceBlast {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.animationtimer += 1;
        if self.animationtimer == self.animationspeed {
            self.animationtimer = 0;

            if self.velx > 0.0 {
                self.drawframe += self.iw;
                if self.drawframe >= self.animationWidth {
                    self.drawframe = 0;
                }
            } else {
                self.drawframe -= self.iw;
                if self.drawframe < 0 {
                    self.drawframe = self.animationWidth - self.iw;
                }
            }
        }

        let (fx, velx) = (self.fx, self.velx);
        self.set_xf(fx + velx);

        self.ttl -= 1;
        if self.ttl <= 0 {
            removeifprojectile(g, self.as_mo_ptr(), false, true);
        } else if g.game_values.spotlights {
            let sx = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
            let sy = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
            if self.sSpotlight.is_none() {
                self.sSpotlight = gp.spotlightManager.add_spotlight(g, sx, sy, 3);
            }

            if !self.sSpotlight.is_none() {
                g.spotlightManager.update_position(self.sSpotlight, sx, sy);
            }
        }
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        if self.iPlayerID != g.player(player).globalID && (g.game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != g.player(player).teamID) {
            if !g.player(player).is_shielded() && !g.player(player).is_invincible() && !g.player(player).shyguy {
                g.player(player).makefrozen(g, g.game_values.wandfreezetime);
                removeifprojectile(g, self.as_mo_ptr(), false, true);
            }
        }

        false
    }

    fn draw(&mut self, g: &mut Game) {
        io_moving_object_draw(g, self);
    }
}

impl IO_MovingObjectTrait for MO_IceBlast {
    fn harness_hash(&mut self, h: &mut crate::smw::rollback::Fnv) {
        crate::smw::objects::moving::moving_object::mo_harness_hash(self, h);
        h.i16s(&[self.ttl]);
    }
    crate::impl_io_moving_object_plumbing!();
}
