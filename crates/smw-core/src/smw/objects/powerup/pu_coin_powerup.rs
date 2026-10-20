//! Coin powerup for coin and greed modes: adds its value to the collector's score.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_mode::{game_mode_coins, game_mode_greed};
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

/// Coin color; selects the 32-pixel sprite row.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CoinColor {
    Red,
    Green,
    Yellow,
    Blue,
}

//------------------------------------------------------------------------------
// class special extra coin powerup for coin or greed mode
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PU_CoinPowerup {
    pub mo_powerup: MO_Powerup,

    /// Sprite sheet row for the coin color, in pixels.
    pub iColorOffsetY: i16,
    /// Points added to the collector's score in coin and greed modes.
    pub iValue: i16,

    /// Tick counter for the sparkle overlay; it steps a frame every 4 ticks.
    pub sparkleanimationtimer: i16,
    /// X offset (pixels) of the current sparkle frame.
    pub sparkledrawframe: i16,
}
impl_base!(PU_CoinPowerup => mo_powerup: MO_Powerup);

impl PU_CoinPowerup {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, color: CoinColor, value: i16) -> Self {
        let mut o = PU_CoinPowerup {
            mo_powerup: MO_Powerup::new(g, nspr, pos, 4, 8, 30, 30, 1, 1),
            iColorOffsetY: (color as i32 * 32) as i16,
            iValue: value,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
        };
        o.velx = 0.0;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;
        o
    }
}

impl CObjectTrait for PU_CoinPowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self, g: &mut Game) {
        mo_powerup_update(g, self);

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if self.state == 0 {
            let iHeight = (32.0f32 - self.fy + self.desty) as i16;
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32, w: 32, h: iHeight as i32 });
            gp.rm.sprite(gp.rm.spr_shinesparkle).draw_src(g, x, y, &Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: iHeight as i32 });
        } else {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32, w: 32, h: 32 });
            gp.rm.sprite(gp.rm.spr_shinesparkle).draw_src(g, x, y, &Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 });
        }
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        let mut player = player;
        if self.state > 0 {
            if g.mode().gamemode == game_mode_coins || g.mode().gamemode == game_mode_greed {
                if !g.mode().gameover {
                    g.player(player).score(g).adjust_score(g, self.iValue);
                    gp.mode().check_winner(g, player);
                }
            }

            if_sound_on_play(g, &mut gp.rm.sfx_coin);
            self.dead = true;
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_CoinPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_CoinPowerup {
    crate::impl_powerup_plumbing!();
}
