//! Port of src/smw/objects/moving/MO_BulletBill.cpp

use crate::common::eyecandy::EC_FallingObject;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_drawpreview;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, lookup_team_id, objectcontainer};
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class bulletbill
//------------------------------------------------------------------------------
pub struct MO_BulletBill {
    pub io_moving_object: IO_MovingObject,

    pub spr_dead: Ptr<gfxSprite>,

    pub iColorID: i16,

    pub iColorOffsetY: i16,
    pub iDirectionOffsetY: i16,

    pub fIsSpawned: bool,
    pub iHiddenDirection: i16,
    pub iHiddenPlane: i16,
}
impl_base!(MO_BulletBill => io_moving_object: IO_MovingObject);

impl MO_BulletBill {
    pub fn new(nspr: Ptr<gfxSprite>, nsprdead: Ptr<gfxSprite>, pos: Vec2s, nspeed: f32, playerID: i16, isspawned: bool) -> Self {
        let mut o = MO_BulletBill {
            io_moving_object: IO_MovingObject::new(nspr, pos, 4, 8, 30, 28, 1, 2, -1, -1, -1, -1),
            spr_dead: Ptr::null(),
            iColorID: 0,
            iColorOffsetY: 0,
            iDirectionOffsetY: 0,
            fIsSpawned: false,
            iHiddenDirection: 0,
            iHiddenPlane: 0,
        };

        o.spr_dead = nsprdead;

        o.velx = nspeed;
        o.vely = 0.0;

        o.movingObjectType = movingobject_bulletbill;
        o.state = 1;

        o.fIsSpawned = isspawned;

        o.ih = 32;
        o.iw = 32;

        o.inair = true;

        if o.fIsSpawned {
            o.iPlayerID = -1;
            o.iColorID = 0;
            o.iTeamID = -1;

            o.animationspeed = 0;

            if o.velx < 0.0 {
                o.iHiddenDirection = 1;
                o.iHiddenPlane = o.ix;
            } else {
                o.iHiddenDirection = 3;
                o.iHiddenPlane = (o.ix as i32 + TILESIZE) as i16;
            }
        } else {
            if o.velx < 0.0 {
                let x = (App::screenWidth + o.iw as i32) as i16;
                o.set_xi(x);
            } else {
                let x = -o.iw;
                o.set_xi(x);
            }

            o.iPlayerID = playerID;
            o.iColorID = unsafe { game_values.colorids[o.iPlayerID as usize] };
            o.iTeamID = lookup_team_id(o.iPlayerID);
        }

        o.iColorOffsetY = (64 * o.iColorID as i32) as i16;
        o.set_direction_offset();

        o.fObjectCollidesWithMap = false;
        o
    }

    // For preview drawing
    pub fn draw_offset(&mut self, iOffsetX: i16, iOffsetY: i16) {
        let surface = self.spr.get_surface();
        let dstX = ((self.ix as i32 >> 1) + iOffsetX as i32) as i16;
        let dstY = ((self.iy as i32 >> 1) + iOffsetY as i32) as i16;
        let srcX = (self.drawframe as i32 >> 1) as i16;
        let srcY = ((self.iColorOffsetY as i32 + self.iDirectionOffsetY as i32) >> 1) as i16;
        let w = (self.iw as i32 >> 1) as i16;
        let h = (self.ih as i32 >> 1) as i16;
        if self.fIsSpawned {
            let edge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.iHiddenDirection as i32) };
            gfx_drawpreview(surface, dstX, dstY, srcX, srcY, w, h, iOffsetX, iOffsetY, 320, 240, false, Some((edge, (self.iHiddenPlane as i32 >> 1) + iOffsetX as i32)));
        } else {
            gfx_drawpreview(surface, dstX, dstY, srcX, srcY, w, h, iOffsetX, iOffsetY, 320, 240, false, None);
        }
    }

    pub fn hittop(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            let mut p = player;
            p.set_yi((self.iy as i32 - PH - 1) as i16);
            p.bouncejump();
            p.get().collisions.checktop(player.get());
            p.platform = Ptr::null();

            p.add_killer_award(Ptr::null(), KillStyle::BulletBill);

            if_sound_on_play(&mut rm.sfx_mip);
        }

        self.die();

        false
    }

    pub fn hitother(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            if player.is_shielded() || player.globalID == self.iPlayerID {
                return false;
            }

            if game_values.teamcollision != TeamCollisionStyle::On && self.iTeamID == player.teamID {
                return false;
            }

            // Find the player that owns this bullet bill so we can attribute a kill
            player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::BulletBill, false, false);
        }

        true
    }

    pub fn set_direction_offset(&mut self) {
        self.iDirectionOffsetY = if self.velx < 0.0 { 0 } else { 32 };
    }
}

impl CObjectTrait for MO_BulletBill {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        let fx = self.fx + self.velx;
        self.set_xf(fx);
        // setYf(fy + vely);

        self.animate();

        if (self.velx < 0.0 && (self.ix as i32) < -(self.iw as i32)) || (self.velx > 0.0 && self.ix as i32 > App::screenWidth) {
            self.dead = true;
        }
    }

    fn draw(&mut self) {
        let src = SDL_Rect {
            x: self.drawframe as i32,
            y: self.iColorOffsetY as i32 + self.iDirectionOffsetY as i32,
            w: self.iw as i32,
            h: self.ih as i32,
        };
        if self.fIsSpawned {
            let edge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.iHiddenDirection as i32) };
            self.spr.draw_clip(self.ix as i32, self.iy as i32, &src, edge, self.iHiddenPlane as i32);
        } else {
            self.spr.draw_src(self.ix as i32, self.iy as i32, &src);
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.dead {
            return false;
        }

        let ix = self.ix as i32;
        let iw = self.iw as i32;

        // if the bullet bill is off the screen, don't wrap it to collide
        if (ix < 0 && self.velx < 0.0 && player.ix as i32 > ix + iw && (player.ix as i32 + PW) < App::screenWidth)
            || (ix + iw >= App::screenWidth && self.velx > 0.0 && (player.ix as i32 + PW) < ix && player.ix >= 0)
        {
            return false;
        }

        if player.is_invincible() || player.shyguy {
            player.add_killer_award(Ptr::null(), KillStyle::BulletBill);
            unsafe {
                if_sound_on_play(&mut rm.sfx_kicksound);
            }

            self.die();
        } else {
            if player.fOldY + PH as f32 <= self.iy as f32 && player.iy as i32 + PH >= self.iy as i32 {
                return self.hittop(player);
            } else {
                return self.hitother(player);
            }
        }

        false
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        unsafe {
            removeifprojectile(object, true, false);

            let r#type: MovingObjectType = object.get_moving_object_type();

            if r#type == movingobject_bulletbill {
                let bulletbill = object.as_any().downcast_mut::<MO_BulletBill>().unwrap();

                // Same team bullet bills don't kill each other
                if bulletbill.iTeamID == self.iTeamID {
                    return;
                }

                bulletbill.dead = true;
                self.dead = true;

                let ix = self.ix as i32;
                let iy = self.iy as i32;
                let bix = bulletbill.ix as i32;
                let biy = bulletbill.iy as i32;

                let mut iOffsetX: i16 = 0;
                if ix + (self.iw as i32) < bix {
                    iOffsetX = App::screenWidth as i16;
                } else if bix + (bulletbill.iw as i32) < ix {
                    iOffsetX = -App::screenWidth as i16;
                }

                let iCenterX: i16 = (((ix + iOffsetX as i32 - bix) >> 1) + (bix + (bulletbill.iw as i32 >> 1))) as i16;
                let iCenterY: i16 = (((iy - biy) >> 1) + (biy + (bulletbill.ih as i32 >> 1))) as i16;

                objectcontainer[2].add(Ptr::new_box(MO_Explosion::new(
                    Ptr::from_mut(&mut rm.spr_explosion),
                    Vec2s::new((iCenterX as i32 - 96) as i16, (iCenterY as i32 - 64) as i16),
                    2,
                    4,
                    -1,
                    -1,
                    KillStyle::BulletBill,
                )));
                if_sound_on_play(&mut rm.sfx_bobombsound);
            } else if r#type == movingobject_shell
                || r#type == movingobject_throwblock
                || r#type == movingobject_throwbox
                || r#type == movingobject_attackzone
                || r#type == movingobject_explosion
            {
                // Don't kill things with shells that are sitting still
                if r#type == movingobject_shell && object.get_state() == 2 {
                    return;
                }

                // Don't kill things with boxesx that aren't moving fast enough
                if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                    return;
                }

                if r#type != movingobject_explosion {
                    object.die();
                }

                if_sound_on_play(&mut rm.sfx_kicksound);
                self.die();
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_BulletBill {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self) {
        self.dead = true;
        unsafe {
            eyecandy[2].emplace(EC_FallingObject::new(
                self.spr_dead,
                self.ix,
                self.iy,
                0.0,
                -VELJUMP / 2.0f32,
                1,
                0,
                if self.velx > 0.0 { 0 } else { 32 },
                (self.iColorID as i32 * 32) as i16,
                32,
                32,
            ));
        }
    }
}
