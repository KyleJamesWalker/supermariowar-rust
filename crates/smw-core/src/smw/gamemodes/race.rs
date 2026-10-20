//! Race mode: touch the numbered race goals in order; each full lap scores a point.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_race, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::object_container::CObjectContainer;
use crate::smw::objects::overmap::wo_race_goal::OMO_RaceGoal;
use crate::smw::player::CPlayer;
use std::any::Any;

/// Clears team `id`'s mark on race goals: all of them when `iGoal` is -1 (lap done) or the
/// penalty is 2, only goal `iGoal` when the penalty is 1, none when it is 0.
fn remove_player_race_goals(g: &mut Game, container: &CObjectContainer, id: i16, iGoal: i16) {
    if g.game_values.gamemodesettings.race.penalty == 0 && iGoal != -1 {
        return;
    }

    let n = container.list().len();
    for i in 0..n {
        let obj = container.list()[i];
        let Some(goal) = obj.get().as_any().downcast_mut::<OMO_RaceGoal>() else {
            continue;
        };

        if iGoal == -1 || 2 == g.game_values.gamemodesettings.race.penalty || (1 == g.game_values.gamemodesettings.race.penalty && goal.get_goal_id() == iGoal) {
            goal.reset(id);
        }
    }
}

/// Race mode. A death undoes progress according to the penalty setting.
#[derive(Clone)]
pub struct CGM_Race {
    pub cgame_mode: CGameMode,

    /// Per team, the index of the next race goal to touch.
    pub nextGoal: [i16; 4],
    /// Number of race goals on the map, at least 2.
    pub quantity: i16,
    /// Death penalty: 0 none, 1 lose the last goal touched, 2 restart the lap.
    pub penalty: i16,
}
impl_base!(CGM_Race => cgame_mode: CGameMode);

impl CGM_Race {
    pub fn new() -> Self {
        let mut this = CGM_Race { cgame_mode: CGameMode::new(), nextGoal: [0; 4], quantity: 0, penalty: 0 };
        this.goal = 10;
        this.gamemode = game_mode_race;
        this.quantity = 3;
        this.penalty = 0;

        this.setup_mode_strings("Race", "Laps", 2);
        this
    }

    pub fn get_next_goal(&self, teamID: i16) -> i16 {
        self.nextGoal[teamID as usize]
    }

    /// Advances the team to its next goal, scoring a lap and checking for a winner after the last one.
    pub fn set_next_goal(&mut self, g: &mut Game, teamID: i16) {
        let mut gp = g.ptr();
        self.nextGoal[teamID as usize] += 1;
        if self.nextGoal[teamID as usize] >= self.quantity {
            self.nextGoal[teamID as usize] = 0;
            remove_player_race_goals(g, &gp.objectcontainer[2], teamID, -1);

            if !self.gameover {
                gp.score[teamID as usize].adjust_score(g, 1);

                //Don't end the game if the goal is infinite
                if self.goal == -1 {
                    return;
                }

                if g.score[teamID as usize].score >= self.goal {
                    gp.score[teamID as usize].set_score(g, self.goal);
                    self.winningteam = teamID;
                    self.gameover = true;

                    remove_players_but_team(g, self.winningteam);
                    setup_score_board(g, false);
                    show_score_board(g);
                } else if g.score[teamID as usize].score as i32 >= self.goal as i32 - 1 && !self.playedwarningsound {
                    self.playwarningsound(g);
                }
            }
        }
    }

    /// Applies the death penalty to the player's team.
    pub fn penalize_race_goals(&mut self, g: &mut Game, player: PlayerId) {
        let mut gp = g.ptr();
        remove_player_race_goals(g, &gp.objectcontainer[2], g.player(player).get_team_id(), (self.nextGoal[g.player(player).get_team_id() as usize] as i32 - 1) as i16);

        if 2 == self.penalty {
            self.nextGoal[g.player(player).get_team_id() as usize] = 0;
        } else if 1 == self.penalty {
            if self.nextGoal[g.player(player).get_team_id() as usize] > 0 {
                self.nextGoal[g.player(player).get_team_id() as usize] -= 1;
            }
        }
    }
}

impl CGameModeTrait for CGM_Race {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.i16s(&self.nextGoal);
        h.i16s(&[self.quantity, self.penalty]);
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

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);

        self.quantity = g.game_values.gamemodesettings.race.quantity;
        if self.quantity < 2 {
            self.quantity = 2;
            g.game_values.gamemodesettings.race.quantity = self.quantity;
        }

        self.penalty = g.game_values.gamemodesettings.race.penalty;
        if self.penalty < 0 || self.penalty > 2 {
            self.penalty = 0;
            g.game_values.gamemodesettings.race.penalty = self.penalty;
        }

        for iRaceGoal in 0..self.quantity {
            let obj = Ptr::new_box(OMO_RaceGoal::new(g, gp.get().rm.spr_racegoal, iRaceGoal));
            gp.get().objectcontainer[2].add(g, obj);
        }

        for iPlayer in 0..4 {
            self.nextGoal[iPlayer] = 0;
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: PlayerId, other: PlayerId, _style: KillStyle) -> PlayerKillType {
        self.penalize_race_goals(g, other);
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(g, self, player, style);

        self.penalize_race_goals(g, player);
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, 1 + if iType == 5 { 1 } else { 0 });

            //Don't end the game if the goal is infinite
            if self.goal == -1 {
                return;
            }

            if g.player(player).score(g).score >= self.goal {
                g.player(player).score(g).set_score(g, self.goal);
                self.winningteam = g.player(player).get_team_id();
                self.gameover = true;

                remove_players_but_team(g, self.winningteam);
                setup_score_board(g, false);
                show_score_board(g);
            } else if g.player(player).score(g).score as i32 >= self.goal as i32 - 1 && !self.playedwarningsound {
                self.playwarningsound(g);
            }
        }
    }
}
