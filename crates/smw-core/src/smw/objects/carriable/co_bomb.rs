//! Port of src/smw/objects/carriable/CO_Bomb.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::Spotlight;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::movingobject_bomb;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};

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
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, vel: Vec2f, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16, timetolive: i16) -> Self {
        let mut o = CO_Bomb {
            mo_carried_object: MO_CarriedObject::new(g, nspr, pos, 5, aniSpeed, 24, 24, 4, 13),
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

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut player = player;
        if self.state == 1 && self.owner.is_none() {
            let item = self.as_carried_ptr();
            if g.player(player).accept_item(item) {
                self.owner = player;

                self.velx = 0.0;
                self.vely = 0.0;

                if self.iPlayerID == -1 {
                    self.iPlayerID = g.player(self.owner).globalID;
                    self.iTeamID = g.player(self.owner).teamID;
                    self.iColorOffsetY = ((g.player(self.owner).colorID as i32 + 1) * 38) as i16;
                }
            }
        }

        false
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.ttl -= 1;
        if self.ttl <= 0 {
            self.die(g);
        }

        if self.dead {
            return;
        }

        if !self.owner.is_none() {
            self.move_to_owner(g);
        } else {
            self.applyfriction();

            // Collision detect map
            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(g, self);
        }

        self.animate();

        if g.game_values.spotlights {
            let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
            let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
            if self.sSpotlight.is_null() {
                self.sSpotlight = gp.spotlightManager.add_spotlight(g, x, y, 3);
            }

            if !self.sSpotlight.is_null() {
                self.sSpotlight.update_position(x, y);
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let src = Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
        if !self.owner.is_none() && g.player(self.owner).iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(g.player(self.owner).get_warp_state() as i32) };
            g.rm.sprite(self.spr).draw_clip(g, x, y, &src, edge, g.player(self.owner).get_warp_plane() as i32);
        } else {
            g.rm.sprite(self.spr).draw_src(g, x, y, &src);
        }
    }
}

impl IO_MovingObjectTrait for CO_Bomb {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn die(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        if !self.owner.is_none() {
            g.player(self.owner).carriedItem = Ptr::null();
            self.owner = PlayerId::NONE;
        }

        let mut player = get_player_from_global_id(g, self.iPlayerID);

        if !player.is_none() {
            g.player(player).decrease_projectiles_count();
        }

        self.dead = true;
        let obj = Ptr::new_box(MO_Explosion::new(
            g,
            rm.spr_explosion,
            Vec2s::new((self.ix as i32 + (self.iw as i32 >> 2) - 96) as i16, (self.iy as i32 + (self.ih as i32 >> 2) - 64) as i16),
            2,
            4,
            self.iPlayerID,
            self.iTeamID,
            KillStyle::Bomb,
        ));
        gp.objectcontainer[2].add(g, obj);
        if_sound_on_play(g, &mut gp.rm.sfx_bobombsound);
    }
}

impl MO_CarriedObjectTrait for CO_Bomb {
    crate::impl_carried_object_plumbing!();
}
