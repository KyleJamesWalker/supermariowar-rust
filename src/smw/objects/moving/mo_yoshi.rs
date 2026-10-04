//! Port of src/smw/objects/moving/MO_Yoshi.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::tile_types::*;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::carriable::co_egg::CO_Egg;
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class yoshi (for egg mode)
//------------------------------------------------------------------------------
pub struct MO_Yoshi {
    pub io_moving_object: IO_MovingObject,

    pub timer: i16,
    pub color: i16,
}
impl_base!(MO_Yoshi => io_moving_object: IO_MovingObject);

impl MO_Yoshi {
    pub fn new(nspr: Ptr<gfxSprite>, iColor: i16) -> Self {
        let mut o = MO_Yoshi {
            io_moving_object: IO_MovingObject::new(nspr, Vec2s::zero(), 2, 8, 52, 56, 0, 0, 0, (iColor as i32 * 56) as i16, 56, 52),
            timer: 0,
            color: 0,
        };

        o.objectType = object_moving;
        o.movingObjectType = movingobject_yoshi;
        o.state = 1;

        o.color = iColor;

        o.place_yoshi();

        o.fObjectCollidesWithMap = false;
        o
    }

    pub fn place_yoshi(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        unsafe {
            self.timer = 0;

            for _tries in 0..64 {
                self.ix = RANDOM_INT(App::screenWidth - self.iw as i32) as i16;
                self.iy = RANDOM_INT(App::screenHeight - self.ih as i32 - TILESIZE) as i16; // don't spawn too low

                let (ix, iy, iw, ih) = (self.ix as i32, self.iy as i32, self.iw as i32, self.ih as i32);
                let ixl: i16 = (ix / TILESIZE) as i16;
                let ixr: i16 = ((ix + iw) / TILESIZE) as i16;
                let iyt: i16 = (iy / TILESIZE) as i16;
                let iyb: i16 = ((iy + ih) / TILESIZE) as i16;

                let upperLeft: i32 = g_map.map(ixl as i32, iyt as i32);
                let upperRight: i32 = g_map.map(ixr as i32, iyt as i32);
                let lowerLeft: i32 = g_map.map(ixl as i32, iyb as i32);
                let lowerRight: i32 = g_map.map(ixr as i32, iyb as i32);

                if (upperLeft & tile_flag_solid) == 0
                    && (upperRight & tile_flag_solid) == 0
                    && (lowerLeft & tile_flag_solid) == 0
                    && (lowerRight & tile_flag_solid) == 0
                    && g_map.block(ixl, iyt).is_null()
                    && g_map.block(ixr, iyt).is_null()
                    && g_map.block(ixl, iyb).is_null()
                    && g_map.block(ixr, iyb).is_null()
                {
                    // spawn on ground, but not on spikes
                    let mut iDeathY: i16 = ((iy + ih) / TILESIZE) as i16;
                    let iDeathX1: i16 = (ix / TILESIZE) as i16;
                    let iDeathX2: i16 = ((ix + iw) / TILESIZE) as i16;

                    while (iDeathY as i32) < MAPHEIGHT {
                        let ttLeftTile: i32 = g_map.map(iDeathX1 as i32, iDeathY as i32);
                        let ttRightTile: i32 = g_map.map(iDeathX2 as i32, iDeathY as i32);

                        if (((ttLeftTile & tile_flag_solid) != 0 || (ttLeftTile & tile_flag_solid_on_top) != 0) && (ttLeftTile & tile_flag_death_on_top) == 0)
                            || (((ttRightTile & tile_flag_solid) != 0 || (ttRightTile & tile_flag_solid_on_top) != 0) && (ttRightTile & tile_flag_death_on_top) == 0)
                            || !g_map.block(iDeathX1, iDeathY).is_null()
                            || !g_map.block(iDeathX2, iDeathY).is_null()
                        {
                            let top: i16 = ((((iDeathY as i32) << 5) - ih) / TILESIZE) as i16;

                            if g_map.spawn(1, iDeathX1, top)
                                && g_map.spawn(1, iDeathX2, top)
                                && g_map.spawn(1, iDeathX1, iDeathY - 1)
                                && g_map.spawn(1, iDeathX2, iDeathY - 1)
                            {
                                let ix = self.ix;
                                self.set_xi(ix);
                                self.set_yi((((iDeathY as i32) << 5) - ih) as i16);
                                return;
                            }

                            break;
                        } else if (ttLeftTile & tile_flag_death_on_top) != 0 || (ttRightTile & tile_flag_death_on_top) != 0 {
                            break;
                        }

                        iDeathY += 1;
                    }
                }
            }

            self.ix = 320;
            self.iy = 240;
        }
    }

    pub fn get_color(&self) -> i16 {
        self.color
    }
}

impl CObjectTrait for MO_Yoshi {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        io_moving_object_draw(self);
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if !player.carriedItem.is_null() && player.carriedItem.get_moving_object_type() == movingobject_egg {
                let mut egg: Ptr<CO_Egg> = Ptr::from_mut(player.carriedItem.as_any().downcast_mut::<CO_Egg>().unwrap());

                if egg.color == self.color {
                    if !game_values.gamemode.gameover {
                        player.score().adjust_score(1);
                        game_values.gamemode.check_winner(player);
                    }

                    self.place_yoshi();

                    egg.place_egg();

                    if_sound_on_play(&mut rm.sfx_yoshi);
                }
            }

            false
        }
    }

    fn update(&mut self) {
        self.animate();

        self.timer += 1;
        if self.timer > 2000 {
            self.place_yoshi();
        }
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        unsafe {
            if object.get_moving_object_type() == movingobject_egg {
                let mut egg: Ptr<CO_Egg> = Ptr::from_mut(object.as_any().downcast_mut::<CO_Egg>().unwrap());

                if egg.color == self.color && !egg.owner_throw.is_null() {
                    let mut player: Ptr<CPlayer> = egg.owner_throw;

                    if !game_values.gamemode.gameover {
                        player.score().adjust_score(1);
                        game_values.gamemode.check_winner(player);
                    }

                    self.place_yoshi();
                    egg.place_egg();

                    if_sound_on_play(&mut rm.sfx_yoshi);
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_Yoshi {
    crate::impl_io_moving_object_plumbing!();
}
