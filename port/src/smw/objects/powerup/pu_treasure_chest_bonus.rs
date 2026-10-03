//! Port of src/smw/objects/powerup/PU_TreasureChestBonus.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class treasure chest powerup
//------------------------------------------------------------------------------
pub struct PU_TreasureChestBonus {
    pub mo_powerup: MO_Powerup,

    pub bonusitem: i16,
    pub numbounces: i16,
    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
    pub drawbonusitemx: i16,
    pub drawbonusitemy: i16,
    pub drawbonusitemtimer: i16,
}
impl_base!(PU_TreasureChestBonus => mo_powerup: MO_Powerup);

impl PU_TreasureChestBonus {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, iNumSpr: i16, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16, iBonusItem: i16) -> Self {
        let mut o = PU_TreasureChestBonus {
            mo_powerup: MO_Powerup::new(nspr, Vec2s::zero(), iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY),
            bonusitem: iBonusItem,
            numbounces: 5,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            drawbonusitemx: 0,
            drawbonusitemy: 0,
            drawbonusitemtimer: 0,
        };
        o.velx = 0.0;
        o.bounce = -VELPOWERUPBOUNCE * 2.0;
        o.state = 2;

        let mut ix = o.ix;
        let mut iy = o.iy;
        let collisionWidth = o.collisionWidth;
        let collisionHeight = o.collisionHeight;
        let mut iAttempts: i16 = 10;
        unsafe {
            while !g_map.findspawnpoint(5, &mut ix, &mut iy, collisionWidth, collisionHeight, false) && {
                let a = iAttempts;
                iAttempts -= 1;
                a > 0
            } {}
        }
        o.ix = ix;
        o.iy = iy;
        o.fx = o.ix as f32;
        o.fy = o.iy as f32;

        o.fObjectDiesOnSuperDeathTiles = false;
        o
    }
}

impl CObjectTrait for PU_TreasureChestBonus {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        mo_powerup_update(self);

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        // Draw rising powerup from chest
        if self.state == 3 {
            self.drawbonusitemy -= 2;

            self.drawbonusitemtimer -= 1;
            if self.drawbonusitemtimer <= 0 {
                self.state = 4;
            }
        } else if self.state == 4 {
            unsafe {
                eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.drawbonusitemy, 3, 8));
            }
            self.dead = true;
        }
    }

    fn draw(&mut self) {
        if self.state < 3 {
            mo_powerup_draw(self);

            // Draw sparkles
            unsafe {
                rm.spr_shinesparkle.draw_src(
                    self.ix as i32 - self.collisionOffsetX as i32,
                    self.iy as i32 - self.collisionOffsetY as i32,
                    &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
                );
            }
        } else {
            unsafe {
                if self.bonusitem as i32 >= NUM_POWERUPS {
                    rm.spr_worlditems.draw_src(
                        self.drawbonusitemx as i32,
                        self.drawbonusitemy as i32,
                        &SDL_Rect { x: (self.bonusitem as i32 - NUM_POWERUPS) << 5, y: 0, w: 32, h: 32 },
                    );
                } else {
                    rm.spr_storedpoweruplarge.draw_src(
                        self.drawbonusitemx as i32,
                        self.drawbonusitemy as i32,
                        &SDL_Rect { x: (self.bonusitem as i32) << 5, y: 0, w: 32, h: 32 },
                    );
                }
            }
        }
    }

    fn collide_player(&mut self, _player: Ptr<CPlayer>) -> bool {
        if self.state == 1 {
            unsafe {
                if_sound_on_play(&mut rm.sfx_treasurechest);
                // if (game_values.worldpowerupcount[player->teamID] < 32)
                //     game_values.worldpowerups[player->teamID][game_values.worldpowerupcount[player->teamID]++] = bonusitem;
                // else
                //	game_values.worldpowerups[player->teamID][31] = bonusitem;

                self.state = 3;

                self.drawbonusitemx = self.ix;
                self.drawbonusitemy = self.iy;
                self.drawbonusitemtimer = 60;

                eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));

                game_values.flags.noexit = false;
            }
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_TreasureChestBonus {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }

    fn bottom_bounce(&mut self) -> f32 {
        if self.state == 2 {
            self.numbounces -= 1;
            if self.numbounces <= 0 {
                self.numbounces = 0;
                self.state = 1;
                self.bounce = GRAVITATION;
            } else if self.vely > 0.0f32 {
                self.bounce = -self.vely / 2.0f32;
            } else {
                self.bounce /= 2.0f32;
            }
        }

        self.bounce
    }
}

impl MO_PowerupTrait for PU_TreasureChestBonus {
    crate::impl_powerup_plumbing!();
}
