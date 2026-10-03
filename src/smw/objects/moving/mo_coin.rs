//! Port of src/smw/objects/moving/MO_Coin.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::objects::moving::moving_object::{
    io_moving_object_collision_detection_checksides, io_moving_object_draw, io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait,
};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class coin (for coin mode)
//------------------------------------------------------------------------------
pub struct MO_Coin {
    pub io_moving_object: IO_MovingObject,

    pub timer: i16,
    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,

    pub iType: i16,
    pub iTeam: i16,
    pub iUncollectableTime: i16,
}
impl_base!(MO_Coin => io_moving_object: IO_MovingObject);

impl MO_Coin {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, vel: Vec2f, pos: Vec2s, color: i16, team: i16, r#type: i16, uncollectabletime: i16, placecoin: bool) -> Self {
        let mut o = MO_Coin {
            io_moving_object: IO_MovingObject::new(nspr, pos, 4, 8, 30, 30, 1, 1, 0, ((color as i32) << 5) as i16, 32, 32),
            timer: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            iType: 0,
            iTeam: 0,
            iUncollectableTime: 0,
        };

        o.state = 1;
        o.objectType = object_moving;
        o.movingObjectType = movingobject_coin;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.iType = r#type;
        o.iTeam = team;

        o.iUncollectableTime = uncollectabletime;
        o.velx = vel.x;
        o.vely = vel.y;

        o.timer = 0;
        if placecoin {
            o.place_coin();
        }

        if o.iType == 0 {
            o.fObjectCollidesWithMap = false;
        } else {
            io_moving_object_collision_detection_checksides(&mut o);
        }
        o
    }

    pub fn place_coin(&mut self) {
        self.timer = 0;

        let mut x: i16 = 0;
        let mut y: i16 = 0;
        let mut iAttempts: i16 = 32;
        unsafe {
            while (!g_map.findspawnpoint(5, &mut x, &mut y, self.collisionWidth, self.collisionHeight, false)
                || objectcontainer[1].get_closest_moving_object(x, y, movingobject_coin) < 150.0f32)
                && {
                    let a = iAttempts;
                    iAttempts -= 1;
                    a > 0
                }
            {}
        }

        self.set_xi(x);
        self.set_yi(y);
    }
}

impl CObjectTrait for MO_Coin {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.iUncollectableTime > 0 || (self.iType == 1 && (!game_values.gamemodesettings.greed.owncoins && self.iTeam == player.get_team_id())) {
                return false;
            }

            if !game_values.gamemode.gameover {
                if self.iType == 2 {
                    player.score().subscore[0] += 1;
                } else {
                    player.score().adjust_score(1);
                }

                game_values.gamemode.check_winner(player);
            }

            eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_coinsparkle), self.ix, self.iy, 7, 4));

            if_sound_on_play(&mut rm.sfx_coin);
        }

        if self.iType == 0 {
            self.place_coin();
        } else {
            self.dead = true;
        }

        false
    }

    fn update(&mut self) {
        if self.iType != 1 {
            self.animate();

            self.sparkleanimationtimer += 1;
            if self.sparkleanimationtimer >= 4 {
                self.sparkleanimationtimer = 0;
                self.sparkledrawframe += 32;
                if self.sparkledrawframe as i32 >= App::screenHeight {
                    self.sparkledrawframe = 0;
                }
            }

            self.timer += 1;
            if self.timer > 1000 {
                self.place_coin();
            }
        }

        if self.iType != 0 {
            self.applyfriction();
            io_moving_object_update(self);

            self.iUncollectableTime -= 1;

            unsafe {
                if self.iType == 1 && (self.iUncollectableTime as i32) < -(game_values.gamemodesettings.greed.coinlife as i32) {
                    eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));
                    self.dead = true;
                }
            }
        }
    }

    fn draw(&mut self) {
        io_moving_object_draw(self);

        // Draw sparkles
        if self.iType != 1 {
            unsafe {
                rm.spr_shinesparkle.draw_src(
                    self.ix as i32 - self.collisionOffsetX as i32,
                    self.iy as i32 - self.collisionOffsetY as i32,
                    &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
                );
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_Coin {
    crate::impl_io_moving_object_plumbing!();
}
