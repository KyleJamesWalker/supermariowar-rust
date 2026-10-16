//! Port of src/smw/gamemodes/Owned.cpp

use crate::game::Game;
use crate::common::game_mode::{game_mode_owned, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::player::CPlayer;
use std::any::Any;

//Owned:
//Players rack up points like domination for each player they have "owned"
pub struct CGM_Owned {
    pub cgame_mode: CGameMode,
    /// Frames toward the next point; never reset between games, like the C++ function-local static.
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
            unsafe {
                self.counter += 1;
                if self.counter >= g.game_values.pointspeed {
                    self.counter = 0;

                    let mut i = 0;
                    while i < g.players.len() {
                        let mut k = 0;
                        while k < g.players.len() {
                            if i != k && g.players[k].ownerPlayerID == g.players[i].get_global_id() {
                                gp.players[i].score().adjust_score(g, 1);
                            }
                            k += 1;
                        }

                        if self.goal > -1 {
                            if g.players[i].score().score >= self.goal {
                                gp.players[i].score().set_score(g, self.goal);

                                self.winningteam = g.players[i].get_team_id();
                                self.gameover = true;

                                remove_players_but_team(g, self.winningteam);
                                setup_score_board(g, false);
                                show_score_board(g);
                            } else if g.players[i].score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
                                self.playwarningsound(g);
                            }
                        }
                        i += 1;
                    }
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if !self.gameover {
            //Give a bonus to the killer if he already owned this player
            if other.ownerPlayerID == inflictor.get_global_id() {
                inflictor.score().adjust_score(g, 5);
            }

            //Release all players owned by the killed player
            unsafe {
                for &player in g.players.iter() {
                    let mut player = player;
                    if player.ownerPlayerID == other.get_global_id() {
                        player.ownerPlayerID = -1;
                    }
                }
            }

            //Assign owned status to the killed player
            if other.get_team_id() != inflictor.get_team_id() {
                other.ownerPlayerID = inflictor.get_global_id();
                other.ownerColorOffsetX = (inflictor.get_color_id() as i32 * 48) as i16;
            }

            return self.check_winner(g, inflictor);
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        unsafe {
            for &other in g.players.iter() {
                let mut other = other;
                if other.ownerPlayerID == player.get_global_id() {
                    other.ownerPlayerID = -1;
                }
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(g, (10 * iType as i32) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(g, self.goal);
            self.winningteam = player.get_team_id();
            self.gameover = true;

            setup_score_board(g, false);
            show_score_board(g);
            remove_players_but_team(g, self.winningteam);
            return PlayerKillType::Removed;
        } else if player.score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }
}
