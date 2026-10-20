//! Hammer projectile thrown in an arc; the super variant flies straight with no gravity.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::SpotlightId;
use crate::common::game::App;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::GRAVITATION;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::blocks::weapon_breakable_block::{B_WeaponBreakableBlock, WeaponDamageType};
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};

//------------------------------------------------------------------------------
// class hammer
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct MO_Hammer {
    pub io_moving_object: IO_MovingObject,

    /// Sprite row offset (pixels) for the thrower's color, 28 per color.
    pub colorOffset: i16,

    /// Ticks left before the hammer disappears.
    pub ttl: i16,
    /// Flies straight without gravity and disappears off the top of the screen.
    pub fSuper: bool,
    /// Spotlight that follows the hammer while spotlights are on.
    pub sSpotlight: SpotlightId,
}
impl_base!(MO_Hammer => io_moving_object: IO_MovingObject);

impl MO_Hammer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, vel: Vec2f, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16, superHammer: bool) -> Self {
        let iCollisionWidth = (g.rm.sprite(nspr).get_width() as i16 as i32 / iNumSpr as i32) as i16;
        let iCollisionHeight = ((g.rm.sprite(nspr).get_height() as i16 as i32) >> 2) as i16;
        let mut o = MO_Hammer {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, 0, 0, -1, -1, -1, -1),
            colorOffset: 0,
            ttl: 0,
            fSuper: false,
            sSpotlight: SpotlightId::NONE,
        };

        o.ih >>= 2;

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;
        o.colorOffset = (iColorID as i32 * 28) as i16;
        o.movingObjectType = movingobject_hammer;

        o.state = 1;

        o.velx = vel.x;
        o.vely = vel.y;
        o.ttl = g.game_values.hammerttl;

        o.fSuper = superHammer;

        if o.velx > 0.0f32 {
            o.drawframe = 0;
        } else {
            o.drawframe = o.animationWidth - o.iw;
        }

        o.fObjectCollidesWithMap = false;

        o.sSpotlight = SpotlightId::NONE;
        o
    }
}

impl CObjectTrait for MO_Hammer {
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
        let (fy, vely) = (self.fy, self.vely);
        self.set_yf(fy + vely);

        if !self.fSuper {
            self.vely += GRAVITATION;
        }

        if self.ix < 0 {
            let ix = (self.ix as i32 + App::screenWidth) as i16;
            self.set_xi(ix);
        } else if self.ix as i32 > App::screenWidth - 1 {
            let ix = (self.ix as i32 - App::screenWidth) as i16;
            self.set_xi(ix);
        }

        if self.iy as i32 > App::screenHeight
            || {
                self.ttl -= 1;
                self.ttl <= 0
            }
            || (self.fSuper && (self.iy as i32) < -(self.ih as i32))
        {
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

        // Detection collision with hammer breakable blocks
        let blocks = self.get_collision_blocks(g);
        for iBlock in 0..4usize {
            let mut block = blocks[iBlock];
            if block.is_null() {
                continue;
            }
            if let Some(weaponbreakableblock) = block.as_any().downcast_mut::<B_WeaponBreakableBlock>() {
                if weaponbreakableblock.r#type() == WeaponDamageType::Hammer {
                    weaponbreakableblock.trigger_behavior_player(g, self.iPlayerID, self.iTeamID);
                    removeifprojectile(g, self.as_mo_ptr(), false, false);
                    return;
                }
            }
        }
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        if self.iPlayerID != g.player(player).globalID && (g.game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != g.player(player).teamID) {
            if !g.player(player).is_shielded() {
                removeifprojectile(g, self.as_mo_ptr(), false, false);

                if !g.player(player).is_invincible() && !g.player(player).shyguy {
                    // Find the player that shot this hammer so we can attribute a kill
                    player_killed_player(g, self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Hammer, false, false);
                    return true;
                }
            }
        }

        false
    }

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &Rect { x: self.drawframe as i32, y: self.colorOffset as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }
}

impl IO_MovingObjectTrait for MO_Hammer {
    fn harness_hash(&mut self, h: &mut crate::smw::rollback::Fnv) {
        crate::smw::objects::moving::moving_object::mo_harness_hash(self, h);
        h.i16s(&[self.ttl]);
        h.bools(&[self.fSuper]);
    }
    crate::impl_io_moving_object_plumbing!();
}
