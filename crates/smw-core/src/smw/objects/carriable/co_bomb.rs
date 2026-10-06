//! Port of src/smw/objects/carriable/CO_Bomb.cpp

use crate::common::eyecandy::Spotlight;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::movingobject_bomb;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{objectcontainer, spotlightManager};
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class bomb
//------------------------------------------------------------------------------
pub struct CO_Bomb {
    pub mo_carried_object: MO_CarriedObject,

    pub iColorOffsetY: i16,
    pub ttl: i16,

    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(CO_Bomb => mo_carried_object: MO_CarriedObject);

impl CO_Bomb {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, vel: Vec2f, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16, timetolive: i16) -> Self {
        let mut o = CO_Bomb {
            mo_carried_object: MO_CarriedObject::new(nspr, pos, 5, aniSpeed, 24, 24, 4, 13),
            iColorOffsetY: 0,
            ttl: 0,
            sSpotlight: Ptr::null(),
        };

        o.iw = 28;
        o.ih = 38;

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;
        o.iColorOffsetY = ((iColorID as i32 + 1) * 38) as i16;

        o.movingObjectType = movingobject_bomb;
        o.state = 1;

        o.ttl = timetolive;

        o.velx = vel.x;
        o.vely = vel.y;

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -16;
        o.iOwnerUpOffset = 40;

        o.sSpotlight = Ptr::null();
        o
    }
}

impl CObjectTrait for CO_Bomb {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.state == 1 && self.owner.is_null() {
            let item = self.as_carried_ptr();
            if player.accept_item(item) {
                self.owner = player;

                self.velx = 0.0;
                self.vely = 0.0;

                if self.iPlayerID == -1 {
                    self.iPlayerID = self.owner.globalID;
                    self.iTeamID = self.owner.teamID;
                    self.iColorOffsetY = ((self.owner.colorID as i32 + 1) * 38) as i16;
                }
            }
        }

        false
    }

    fn update(&mut self) {
        self.ttl -= 1;
        if self.ttl <= 0 {
            self.die();
        }

        if self.dead {
            return;
        }

        if !self.owner.is_null() {
            self.move_to_owner();
        } else {
            self.applyfriction();

            // Collision detect map
            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(self);
        }

        self.animate();

        unsafe {
            if game_values.spotlights {
                let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight(x, y, 3);
                }

                if !self.sSpotlight.is_null() {
                    self.sSpotlight.update_position(x, y);
                }
            }
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let src = SDL_Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
        if !self.owner.is_null() && self.owner.iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
            self.spr.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
        } else {
            self.spr.draw_src(x, y, &src);
        }
    }
}

impl IO_MovingObjectTrait for CO_Bomb {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn die(&mut self) {
        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            self.owner = Ptr::null();
        }

        let mut player = get_player_from_global_id(self.iPlayerID);

        if !player.is_null() {
            player.decrease_projectiles_count();
        }

        self.dead = true;
        unsafe {
            objectcontainer[2].add(Ptr::new_box(MO_Explosion::new(
                Ptr::from_mut(&mut rm.spr_explosion),
                Vec2s::new((self.ix as i32 + (self.iw as i32 >> 2) - 96) as i16, (self.iy as i32 + (self.ih as i32 >> 2) - 64) as i16),
                2,
                4,
                self.iPlayerID,
                self.iTeamID,
                KillStyle::Bomb,
            )));
            if_sound_on_play(&mut rm.sfx_bobombsound);
        }
    }
}

impl MO_CarriedObjectTrait for CO_Bomb {
    crate::impl_carried_object_plumbing!();
}
