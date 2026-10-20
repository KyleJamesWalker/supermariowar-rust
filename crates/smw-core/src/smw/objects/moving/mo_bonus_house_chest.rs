//! Bonus house chest: opening it awards a world item or tournament points and ends the bonus stage.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class treasure chest powerup
//------------------------------------------------------------------------------
/// Chest opened with turbo; `state` 1 closed, 2 item rising, 3 done.
#[derive(Clone)]
pub struct MO_BonusHouseChest {
    pub io_moving_object: IO_MovingObject,

    /// Prize: a powerup index, then world items from `NUM_POWERUPS`, then score bonuses from `NUM_POWERUPS + NUM_WORLD_POWERUPS`.
    pub bonusitem: i16,
    /// Y (pixels) of the prize rising out of the open chest.
    pub drawbonusitemy: i16,
    /// Ticks left in the prize's rise.
    pub drawbonusitemtimer: i16,
}
impl_base!(MO_BonusHouseChest => io_moving_object: IO_MovingObject);

impl MO_BonusHouseChest {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iBonusItem: i16) -> Self {
        let mut o = MO_BonusHouseChest {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, 1, 0, 64, 64, 0, 0, -1, -1, -1, -1),
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

    fn update(&mut self, g: &mut Game) {
        // Draw rising powerup from chest
        if self.state == 2 {
            self.drawbonusitemtimer -= 1;
            let risen = vel(2.0) * (ticks(75) - self.drawbonusitemtimer as i32) as f32;
            self.drawbonusitemy = (self.iy as i32 + 32 - risen as i32) as i16;

            if self.drawbonusitemtimer <= 0 {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, 
                    g.rm.spr_fireballexplosion,
                    (self.ix as i32 + 16) as i16,
                    self.drawbonusitemy,
                    3,
                    8,
                ));
                self.state = 3;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let ix = self.ix as i32;
        let iy = self.iy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        if self.state < 2 {
            g.rm.sprite(self.spr).draw_src(g, ix, iy, &Rect { x: 0, y: 0, w: iw, h: ih });
        }

        if self.state >= 2 {
            g.rm.sprite(self.spr).draw_src(g, ix, iy, &Rect { x: 128, y: 0, w: iw, h: ih });
        }

        if self.state == 2 {
            let bonusitem = self.bonusitem as i32;
            if bonusitem >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                // Score bonuses
                let iBonus: i16 = (bonusitem - NUM_POWERUPS - NUM_WORLD_POWERUPS) as i16;
                let iBonusX: i16 = ((iBonus as i32 % 10) << 5) as i16;
                let iBonusY: i16 = (((iBonus as i32 / 10) << 5) + 32) as i16;
                gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, ix + 16, self.drawbonusitemy as i32, &Rect { x: iBonusX as i32, y: iBonusY as i32, w: 32, h: 32 });
            } else if bonusitem >= NUM_POWERUPS {
                // World Item
                gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, ix + 16, self.drawbonusitemy as i32, &Rect { x: (bonusitem - NUM_POWERUPS) << 5, y: 0, w: 32, h: 32 });
            } else {
                // Normal Powerup
                gp.rm.sprite(gp.rm.spr_storedpoweruplarge).draw_src(g, ix + 16, self.drawbonusitemy as i32, &Rect { x: bonusitem << 5, y: 0, w: 32, h: 32 });
            }
        }

        if self.state >= 2 {
            g.rm.sprite(self.spr).draw_src(g, ix, iy, &Rect { x: 64, y: 0, w: iw, h: ih });
        }
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        if self.state == 1 && !g.mode().gameover && g.player(player).keys(g).game_turbo().fPressed {
            let teamID = g.player(player).teamID as usize;
            if (self.bonusitem as i32) < NUM_POWERUPS + NUM_WORLD_POWERUPS {
                if g.game_values.worldpowerupcount[teamID] < 32 {
                    let idx = g.game_values.worldpowerupcount[teamID] as usize;
                    g.game_values.worldpowerupcount[teamID] += 1;
                    g.game_values.worldpowerups[teamID][idx] = self.bonusitem;
                } else {
                    g.game_values.worldpowerups[teamID][31] = self.bonusitem;
                }
            } else {
                let mut iBonus: i16 = (self.bonusitem as i32 - NUM_POWERUPS - NUM_WORLD_POWERUPS) as i16;
                if iBonus < 10 {
                    iBonus = iBonus + 1;
                } else {
                    iBonus = 9 - iBonus;
                }

                g.game_values.tournament_scores[teamID].total = (g.game_values.tournament_scores[teamID].total as i32 + iBonus as i32) as i16;

                if g.game_values.tournament_scores[teamID].total < 0 {
                    g.game_values.tournament_scores[teamID].total = 0;
                }
            }

            if_sound_on_play(g, &mut gp.rm.sfx_treasurechest);
            self.state = 2;

            self.drawbonusitemy = (self.iy as i32 + 32) as i16;
            self.drawbonusitemtimer = ticks(75) as i16;

            g.game_values.flags.forceexittimer = ticks(180) as i16;
            g.mode().gameover = true;
            g.mode().winningteam = g.player(player).teamID;
        }

        false
    }
}

impl IO_MovingObjectTrait for MO_BonusHouseChest {
    crate::impl_io_moving_object_plumbing!();
}
