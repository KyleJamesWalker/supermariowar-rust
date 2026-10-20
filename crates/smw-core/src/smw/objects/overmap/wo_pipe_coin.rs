//! Coin launched from the pipe in the coin pipe minigame.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::GRAVITATION;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::object_base::{object_pipe_coin, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::overmap::over_map_object::{IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class pipe coin (for coin pipe minigame)
//------------------------------------------------------------------------------
/// Falling coin. A team coin scores only for its team; a neutral coin's color sets its value.
#[derive(Clone)]
pub struct OMO_PipeCoin {
    pub io_over_map_object: IO_OverMapObject,

    /// Team the coin scores for, or -1 for a neutral coin.
    pub iTeamID: i16,
    /// Neutral coin value by color: 0 costs a point, 1 gives 5, 2 gives 1.
    pub iColorID: i16,

    /// Tick counter for the sparkle overlay; it steps a frame every 4 ticks.
    pub sparkleanimationtimer: i16,
    /// X offset (pixels) of the current sparkle frame.
    pub sparkledrawframe: i16,

    /// Ticks before it can be collected; it is clipped to the pipe mouth meanwhile.
    pub iUncollectableTime: i16,
}
impl_base!(OMO_PipeCoin => io_over_map_object: IO_OverMapObject);

impl OMO_PipeCoin {
    pub fn new(g: &mut Game, nspr: SpriteId, vel: Vec2f, pos: Vec2s, teamid: i16, colorid: i16, uncollectabletime: i16) -> Self {
        let mut this = OMO_PipeCoin {
            io_over_map_object: IO_OverMapObject::new(g, nspr, pos, 4, 8, 30, 30, 1, 1, 0, ((colorid as i32) << 5) as i16, 32, 32),
            iTeamID: teamid,
            iColorID: colorid,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            iUncollectableTime: 0,
        };
        this.state = 1;
        this.objectType = object_pipe_coin;

        this.velx = vel.x;

        if g.pipegamemode.is_slowdown() {
            this.vely = vel.y / 2.0f32;
        } else {
            this.vely = vel.y;
        }

        this.iUncollectableTime = uncollectabletime;
        this
    }

    pub fn get_color(&self) -> i16 {
        self.iColorID
    }

    pub fn get_team(&self) -> i16 {
        self.iTeamID
    }
}

impl CObjectTrait for OMO_PipeCoin {
    crate::impl_cobject_plumbing!();

    fn update(&mut self, g: &mut Game) {
        let x = self.fx + self.velx;
        self.set_xf(x);
        let y = self.fy + self.vely;
        self.set_yf(y);

        if self.iTeamID == -1 {
            self.animate();
        }

        if self.iy as i32 >= App::screenHeight {
            self.dead = true;
        }

        if g.pipegamemode.is_slowdown() {
            self.vely += GRAVITATION / 1.5f32;
        } else {
            self.vely += GRAVITATION;
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        if self.iUncollectableTime > 0 {
            self.iUncollectableTime -= 1;
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let srcRect = Rect { x: self.drawframe as i32, y: self.animationOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
        let sparkleRect = Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 };

        if self.iUncollectableTime > 0 {
            g.rm.sprite(self.spr).draw_clip(g, x, y, &srcRect, ClipEdge::Bottom, 256);

            // Draw sparkles
            if self.iTeamID == -1 {
                gp.rm.sprite(gp.rm.spr_shinesparkle).draw_clip(g, x, y, &sparkleRect, ClipEdge::Bottom, 256);
            }
        } else {
            g.rm.sprite(self.spr).draw_src(g, x, y, &srcRect);

            // Draw sparkles
            if self.iTeamID == -1 {
                gp.rm.sprite(gp.rm.spr_shinesparkle).draw_src(g, x, y, &sparkleRect);
            }
        }
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        let mut gp = g.ptr();
        if self.iUncollectableTime > 0 {
            return false;
        }

        if !g.mode().gameover {
            if self.iTeamID != -1 {
                if g.player(player).teamID == self.iTeamID {
                    g.player(player).score(g).adjust_score(g, 1);
                    if_sound_on_play(g, &mut gp.rm.sfx_coin);
                }
            } else if self.iColorID == 2 {
                g.player(player).score(g).adjust_score(g, 1);
                if_sound_on_play(g, &mut gp.rm.sfx_coin);
            } else if self.iColorID == 0 {
                g.player(player).score(g).adjust_score(g, -1);
                if_sound_on_play(g, &mut gp.rm.sfx_stun);
            } else if self.iColorID == 1 {
                g.player(player).score(g).adjust_score(g, 5);
                if_sound_on_play(g, &mut gp.rm.sfx_extraguysound);
            }

            gp.mode().check_winner(g, player);
        }

        g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, g.rm.spr_coinsparkle, self.ix, self.iy, 7, 4));

        self.dead = true;
        false
    }
}

impl IO_OverMapObjectTrait for OMO_PipeCoin {
    crate::impl_over_map_object_plumbing!();
}
