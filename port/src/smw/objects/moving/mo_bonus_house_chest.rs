//! Port of src/smw/objects/moving/MO_BonusHouseChest.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class treasure chest powerup
//------------------------------------------------------------------------------
pub struct MO_BonusHouseChest {
    pub io_moving_object: IO_MovingObject,

    pub bonusitem: i16,
    pub drawbonusitemy: i16,
    pub drawbonusitemtimer: i16,
}
impl_base!(MO_BonusHouseChest => io_moving_object: IO_MovingObject);

impl MO_BonusHouseChest {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iBonusItem: i16) -> Self {
        let mut o = MO_BonusHouseChest {
            io_moving_object: IO_MovingObject::new(nspr, pos, 1, 0, 64, 64, 0, 0, -1, -1, -1, -1),
            bonusitem: 0,
            drawbonusitemy: 0,
            drawbonusitemtimer: 0,
        };

        o.iw = 64;
        o.ih = 64;

        o.state = 1;
        o.bonusitem = iBonusItem;

        o.drawbonusitemy = 0;
        o.drawbonusitemtimer = 0;

        o.movingObjectType = movingobject_treasurechest;

        o.fObjectDiesOnSuperDeathTiles = false;
        o.fObjectCollidesWithMap = false;
        o
    }
}

impl CObjectTrait for MO_BonusHouseChest {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        // Draw rising powerup from chest
        if self.state == 2 {
            self.drawbonusitemy -= 2;

            self.drawbonusitemtimer -= 1;
            if self.drawbonusitemtimer <= 0 {
                unsafe {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 + 16) as i16,
                        self.drawbonusitemy,
                        3,
                        8,
                    ));
                }
                self.state = 3;
            }
        }
    }

    fn draw(&mut self) {
        let ix = self.ix as i32;
        let iy = self.iy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        if self.state < 2 {
            self.spr.draw_src(ix, iy, &SDL_Rect { x: 0, y: 0, w: iw, h: ih });
        }

        if self.state >= 2 {
            self.spr.draw_src(ix, iy, &SDL_Rect { x: 128, y: 0, w: iw, h: ih });
        }

        if self.state == 2 {
            let bonusitem = self.bonusitem as i32;
            unsafe {
                if bonusitem >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                    // Score bonuses
                    let iBonus: i16 = (bonusitem - NUM_POWERUPS - NUM_WORLD_POWERUPS) as i16;
                    let iBonusX: i16 = ((iBonus as i32 % 10) << 5) as i16;
                    let iBonusY: i16 = (((iBonus as i32 / 10) << 5) + 32) as i16;
                    rm.spr_worlditems.draw_src(ix + 16, self.drawbonusitemy as i32, &SDL_Rect { x: iBonusX as i32, y: iBonusY as i32, w: 32, h: 32 });
                } else if bonusitem >= NUM_POWERUPS {
                    // World Item
                    rm.spr_worlditems.draw_src(ix + 16, self.drawbonusitemy as i32, &SDL_Rect { x: (bonusitem - NUM_POWERUPS) << 5, y: 0, w: 32, h: 32 });
                } else {
                    // Normal Powerup
                    rm.spr_storedpoweruplarge.draw_src(ix + 16, self.drawbonusitemy as i32, &SDL_Rect { x: bonusitem << 5, y: 0, w: 32, h: 32 });
                }
            }
        }

        if self.state >= 2 {
            self.spr.draw_src(ix, iy, &SDL_Rect { x: 64, y: 0, w: iw, h: ih });
        }
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.state == 1 && !game_values.gamemode.gameover && player.playerKeys.game_turbo().fPressed {
                let teamID = player.teamID as usize;
                if (self.bonusitem as i32) < NUM_POWERUPS + NUM_WORLD_POWERUPS {
                    if game_values.worldpowerupcount[teamID] < 32 {
                        let idx = game_values.worldpowerupcount[teamID] as usize;
                        game_values.worldpowerupcount[teamID] += 1;
                        game_values.worldpowerups[teamID][idx] = self.bonusitem;
                    } else {
                        game_values.worldpowerups[teamID][31] = self.bonusitem;
                    }
                } else {
                    let mut iBonus: i16 = (self.bonusitem as i32 - NUM_POWERUPS - NUM_WORLD_POWERUPS) as i16;
                    if iBonus < 10 {
                        iBonus = iBonus + 1;
                    } else {
                        iBonus = 9 - iBonus;
                    }

                    game_values.tournament_scores[teamID].total = (game_values.tournament_scores[teamID].total as i32 + iBonus as i32) as i16;

                    if game_values.tournament_scores[teamID].total < 0 {
                        game_values.tournament_scores[teamID].total = 0;
                    }
                }

                if_sound_on_play(&mut rm.sfx_treasurechest);
                self.state = 2;

                self.drawbonusitemy = (self.iy as i32 + 32) as i16;
                self.drawbonusitemtimer = 75;

                game_values.flags.forceexittimer = 180;
                game_values.gamemode.gameover = true;
                game_values.gamemode.winningteam = player.teamID;
            }
        }

        false
    }
}

impl IO_MovingObjectTrait for MO_BonusHouseChest {
    crate::impl_io_moving_object_plumbing!();
}
