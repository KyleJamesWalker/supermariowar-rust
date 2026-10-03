//! Port of src/smw/objects/carriable/CO_Star.cpp

use crate::common::game::App;
use crate::common::game_mode::game_mode_star;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_star;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::star::CGM_Star;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_update, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class star (for star mode)
//------------------------------------------------------------------------------
pub struct CO_Star {
    pub mo_carried_object: MO_CarriedObject,

    pub timer: i16,
    pub iType: i16,
    pub iOffsetY: i16,
    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
    pub iID: i16,
}
impl_base!(CO_Star => mo_carried_object: MO_CarriedObject);

impl CO_Star {
    pub fn new(nspr: Ptr<gfxSprite>, r#type: i16, id: i16) -> Self {
        let mut o = CO_Star {
            mo_carried_object: MO_CarriedObject::new(nspr, Vec2s::zero(), 8, 8, 30, 30, 1, 1),
            timer: 0,
            iType: 0,
            iOffsetY: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            iID: 0,
        };

        o.iID = id;

        o.state = 1;
        o.iw = 32;
        o.ih = 32;
        o.movingObjectType = movingobject_star;

        o.iType = r#type;

        o.iOffsetY = if r#type == 1 { 32 } else { 0 };

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.dKickX = 3.0;
        o.dKickY = 6.0;

        o.fCarriedByKuriboShoe = true;

        o.place_star();

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -22;
        o.iOwnerUpOffset = 32;
        o
    }

    pub fn get_type(&self) -> i16 {
        self.iType
    }
    pub fn set_player_color(&mut self, iColor: i16) {
        self.iOffsetY = (64 + ((iColor as i32) << 5)) as i16;
    }

    pub fn place_star(&mut self) {
        unsafe {
            if game_values.gamemode.gamemode != game_mode_star {
                return;
            }

            let mut starmode: Ptr<CGM_Star> = Ptr::from_mut(game_values.gamemode.as_any().downcast_mut::<CGM_Star>().unwrap());

            self.timer = 0;

            let star = starmode.getstarplayer(self.iID);

            if !star.is_null() {
                self.set_xf(star.fx + HALFPW as f32 - 16.0);
                self.set_yf(star.fy + HALFPH as f32 - 17.0);

                self.velx = star.velx;
                self.vely = star.vely;
            }
        }

        MO_CarriedObjectTrait::drop(self);
    }
}

impl CObjectTrait for CO_Star {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        unsafe {
            if game_values.gamemode.gamemode != game_mode_star {
                return false;
            }

            let mut starmode: Ptr<CGM_Star> = Ptr::from_mut(game_values.gamemode.as_any().downcast_mut::<CGM_Star>().unwrap());

            self.timer = 0;
            if self.owner.is_null() && player.isready() {
                if player.throw_star == 0 && player.accept_item(self.as_carried_ptr()) {
                    self.owner = player;
                }
            }

            if (self.iType == 0 && player.is_invincible()) || player.is_shielded() || starmode.isplayerstar(player) || game_values.gamemode.gameover {
                return false;
            }

            let mut oldstar = starmode.swapplayer(self.iID, player);

            if self.owner == oldstar {
                oldstar.throw_star = 30;
                self.kick();
            }
        }

        false
    }

    fn update(&mut self) {
        if !self.owner.is_null() {
            self.move_to_owner();
            self.timer = 0;
        } else if {
            self.timer += 1;
            self.timer > 300
        } {
            self.place_star();
        } else {
            self.applyfriction();

            // Collision detect map
            io_moving_object_update(self);
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if !self.owner.is_null() {
            let src = SDL_Rect { x: 0, y: self.iOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
            if self.owner.iswarping() {
                let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
                self.spr.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
            } else {
                self.spr.draw_src(x, y, &src);
            }
        } else if self.velx != 0.0 {
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: self.iOffsetY as i32, w: self.iw as i32, h: self.ih as i32 }); // keep the star still while it's moving
        } else {
            self.spr.draw_src(x, y, &SDL_Rect { x: self.drawframe as i32, y: self.iOffsetY as i32, w: self.iw as i32, h: self.ih as i32 });
        }

        let src = SDL_Rect { x: self.sparkledrawframe as i32, y: if self.iType != 0 { 0 } else { 32 }, w: 32, h: 32 };
        unsafe {
            if !self.owner.is_null() && self.owner.iswarping() {
                let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32);
                rm.spr_shinesparkle.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
            } else {
                rm.spr_shinesparkle.draw_src(x, y, &src);
            }
        }
    }
}

impl IO_MovingObjectTrait for CO_Star {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_Star {
    crate::impl_carried_object_plumbing!();
}
