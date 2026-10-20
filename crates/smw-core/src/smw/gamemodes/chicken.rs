//! Chicken mode: the chicken scores over time and the player who kills it becomes the chicken.

use crate::smw::players::PlayerId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{ticks, PHOFFSET, PWOFFSET, VELJUMP, VELMOVING_CHICKEN};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::player::CPlayer;

/// Chicken mode: the first kill, or killing the chicken, makes the killer the chicken. The chicken
/// scores a point every `ticks(pointspeed)` ticks and 5 per kill, and moves at most `VELMOVING_CHICKEN`.
#[derive(Clone)]
pub struct CGM_Chicken {
    pub cgame_mode: CGameMode,

    /// The current chicken, or `PlayerId::NONE` when nobody is the chicken.
    pub m_chicken: PlayerId,
    /// Ticks toward the chicken's next point; `init` does not reset it.
    counter: i16,
}
impl_base!(CGM_Chicken => cgame_mode: CGameMode);

/// Maps a warp state (0-3) to the edge the warping sprite is clipped against.
pub(crate) fn clip_edge_from(v: i16) -> ClipEdge {
    match v as u8 {
        0 => ClipEdge::Top,
        1 => ClipEdge::Right,
        2 => ClipEdge::Bottom,
        _ => ClipEdge::Left,
    }
}

impl CGM_Chicken {
    pub fn new() -> Self {
        let mut this = CGM_Chicken { cgame_mode: CGameMode::new(), m_chicken: PlayerId::NONE, counter: 0 };
        this.goal = 200;
        this.gamemode = game_mode_chicken;

        this.setup_mode_strings("Chicken", "Points", 50);
        this
    }

    pub fn chicken(&self) -> PlayerId {
        self.m_chicken
    }

    pub fn clear_chicken(&mut self) {
        self.m_chicken = PlayerId::NONE;
    }
}

impl CGameModeTrait for CGM_Chicken {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.player(self.m_chicken);
        h.i16s(&[self.counter]);
    }
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgamemode_init(g, self);
        self.clear_chicken();
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        if !self.m_chicken.is_none() {
            let mut chicken = self.m_chicken;
            if g.player(chicken).get_vel_x() > VELMOVING_CHICKEN {
                g.player(chicken).velx = VELMOVING_CHICKEN;
            } else if g.player(chicken).get_vel_x() < -VELMOVING_CHICKEN {
                g.player(chicken).velx = -VELMOVING_CHICKEN;
            }

            if g.player(chicken).isready() && !g.player(chicken).is_tanooki_statue() {
                self.counter += 1;
                if self.counter as i32 >= ticks(g.game_values.pointspeed as i32) {
                    self.counter = 0;
                    g.player(chicken).score(g).adjust_score(g, 1);
                    self.check_winner(g, chicken);
                }
            }
        }
    }

    fn draw_foreground(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Draw the chicken indicator around the chicken
        if g.game_values.gamemodesettings.chicken.usetarget && !self.gameover && !self.m_chicken.is_none() {
            let chicken = self.m_chicken;
            if g.player(chicken).iswarping() {
                gp.rm.sprite(gp.rm.spr_chicken).draw_clip(
                    g,
                    g.player(chicken).left_x() as i32 - PWOFFSET - 16,
                    g.player(chicken).top_y() as i32 - PHOFFSET - 16,
                    &Rect { x: 0, y: 0, w: 64, h: 64 },
                    clip_edge_from(g.player(chicken).get_warp_state()),
                    g.player(chicken).get_warp_plane() as i32,
                );
            } else if g.player(chicken).isready() {
                gp.rm.sprite(gp.rm.spr_chicken).draw(g, g.player(chicken).center_x() as i32 - 32, g.player(chicken).center_y() as i32 - 32);
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: PlayerId, mut other: PlayerId, _style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if self.m_chicken.is_none() || other == self.m_chicken {
            self.m_chicken = inflictor;
            g.eyecandy[2].emplace(EC_GravText::new(
                Ptr::from_mut(&mut g.rm.game_font_large),
                g.player(inflictor).center_x(),
                g.player(inflictor).bottom_y(),
                "Chicken!".to_string(),
                (-(VELJUMP as f64) * 1.5) as f32,
            ));
            //eyecandy[2].emplace<EC_SingleAnimation>(&rm->spr_fireballexplosion, inflictor.centerX() - 16, inflictor.centerY() - 16, 3, 8));
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_poof,
                (g.player(inflictor).center_x() as i32 - 24) as i16,
                (g.player(inflictor).center_y() as i32 - 24) as i16,
                4,
                5,
            ));
            if_sound_on_play(g, &mut gp.rm.sfx_transform);

            if other == self.m_chicken {
                g.player(other).set_corpse_type(1); //flag to use chicken corpse sprite
            }
        } else if inflictor == self.m_chicken {
            if !self.gameover {
                g.player(inflictor).score(g).adjust_score(g, 5);
                return self.check_winner(g, inflictor);
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(g, self, player, style);

        if self.m_chicken == player {
            g.player(player).set_corpse_type(1); //flag to use chicken corpse sprite

            if !self.gameover {
                self.m_chicken = PlayerId::NONE;
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, (10 * iType as i32) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if g.player(player).score(g).score >= self.goal {
            g.player(player).score(g).set_score(g, self.goal);
            self.winningteam = g.player(player).get_team_id();
            self.gameover = true;

            setup_score_board(g, false);
            show_score_board(g);
            remove_players_but_team(g, self.winningteam);
            return PlayerKillType::Removed;
        } else if g.player(player).score(g).score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }
}
