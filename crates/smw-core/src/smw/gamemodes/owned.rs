//! Owned mode: kill opponents to own them; owned players earn their owner points over time.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_owned, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::global_constants::ticks;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::player::CPlayer;
use std::any::Any;

/// Owned mode. Each owned player is worth a point every `ticks(pointspeed)` ticks; killing a
/// player you already own is worth 5, and dying frees everyone you own.
#[derive(Clone)]
pub struct CGM_Owned {
    pub cgame_mode: CGameMode,
    /// Ticks toward the next point payout; `init` does not reset it.
    counter: i16,
}
impl_base!(CGM_Owned => cgame_mode: CGameMode);

impl CGM_Owned {
    pub fn new() -> Self {
        let mut this = CGM_Owned { cgame_mode: CGameMode::new(), counter: 0 };
        this.goal = 200;
        this.gamemode = game_mode_owned;

        this.setup_mode_strings("Owned", "Points", 50);
        this
    }
}

impl CGameModeTrait for CGM_Owned {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.i16s(&[self.counter]);
    }
    fn gm(&self) -> &CGameMode {
        &self.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn think(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.gameover {
            self.displayplayertext(g);
        } else {
            self.counter += 1;
            if self.counter as i32 >= ticks(g.game_values.pointspeed as i32) {
                self.counter = 0;

                let mut i = 0;
                while i < g.players.len() {
                    let mut k = 0;
                    while k < g.players.len() {
                        if i != k && g.player(g.players[k]).ownerPlayerID == g.player(g.players[i]).get_global_id() {
                            g.player(gp.players[i]).score(g).adjust_score(g, 1);
                        }
                        k += 1;
                    }

                    if self.goal > -1 {
                        if g.player(g.players[i]).score(g).score >= self.goal {
                            g.player(gp.players[i]).score(g).set_score(g, self.goal);

                            self.winningteam = g.player(g.players[i]).get_team_id();
                            self.gameover = true;

                            remove_players_but_team(g, self.winningteam);
                            setup_score_board(g, false);
                            show_score_board(g);
                        } else if g.player(g.players[i]).score(g).score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
                            self.playwarningsound(g);
                        }
                    }
                    i += 1;
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: PlayerId, mut other: PlayerId, _style: KillStyle) -> PlayerKillType {
        if !self.gameover {
            //Give a bonus to the killer if he already owned this player
            if g.player(other).ownerPlayerID == g.player(inflictor).get_global_id() {
                g.player(inflictor).score(g).adjust_score(g, 5);
            }

            //Release all players owned by the killed player
            for &player in g.players.iter() {
                let mut player = player;
                if g.player(player).ownerPlayerID == g.player(other).get_global_id() {
                    g.player(player).ownerPlayerID = -1;
                }
            }

            //Assign owned status to the killed player
            if g.player(other).get_team_id() != g.player(inflictor).get_team_id() {
                g.player(other).ownerPlayerID = g.player(inflictor).get_global_id();
                g.player(other).ownerColorOffsetX = (g.player(inflictor).get_color_id() as i32 * 48) as i16;
            }

            return self.check_winner(g, inflictor);
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(g, self, player, style);

        for &other in g.players.iter() {
            let mut other = other;
            if g.player(other).ownerPlayerID == g.player(player).get_global_id() {
                g.player(other).ownerPlayerID = -1;
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
