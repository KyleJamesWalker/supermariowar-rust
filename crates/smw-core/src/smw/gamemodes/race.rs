//! Port of src/smw/gamemodes/Race.cpp

use crate::common::game_mode::{game_mode_race, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::score;
use crate::smw::object_container::CObjectContainer;
use crate::smw::objects::overmap::wo_race_goal::OMO_RaceGoal;
use crate::smw::player::CPlayer;
use std::any::Any;

fn remove_player_race_goals(container: &CObjectContainer, id: i16, iGoal: i16) {
    unsafe {
        if game_values.gamemodesettings.race.penalty == 0 && iGoal != -1 {
            return;
        }

        let n = container.list().len();
        for i in 0..n {
            let obj = container.list()[i];
            let Some(goal) = obj.get().as_any().downcast_mut::<OMO_RaceGoal>() else {
                continue;
            };

            if iGoal == -1 || 2 == game_values.gamemodesettings.race.penalty || (1 == game_values.gamemodesettings.race.penalty && goal.get_goal_id() == iGoal) {
                goal.reset(id);
            }
        }
    }
}

//Race
//Touch all the flying blocks in order
//Each successful curcuit you complete (before getting killed)
//Counts as one point
pub struct CGM_Race {
    pub cgame_mode: CGameMode,

    pub nextGoal: [i16; 4],
    pub quantity: i16,
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

    pub fn set_next_goal(&mut self, teamID: i16) {
        unsafe {
            self.nextGoal[teamID as usize] += 1;
            if self.nextGoal[teamID as usize] >= self.quantity {
                self.nextGoal[teamID as usize] = 0;
                remove_player_race_goals(&objectcontainer[2], teamID, -1);

                if !self.gameover {
                    score[teamID as usize].adjust_score(1);

                    //Don't end the game if the goal is infinite
                    if self.goal == -1 {
                        return;
                    }

                    if score[teamID as usize].score >= self.goal {
                        score[teamID as usize].set_score(self.goal);
                        self.winningteam = teamID;
                        self.gameover = true;

                        remove_players_but_team(self.winningteam);
                        setup_score_board(false);
                        show_score_board();
                    } else if score[teamID as usize].score as i32 >= self.goal as i32 - 1 && !self.playedwarningsound {
                        self.playwarningsound();
                    }
                }
            }
        }
    }

    //Player loses control of his areas
    pub fn penalize_race_goals(&mut self, player: Ptr<CPlayer>) {
        unsafe {
            remove_player_race_goals(&objectcontainer[2], player.get_team_id(), (self.nextGoal[player.get_team_id() as usize] as i32 - 1) as i16);
        }

        if 2 == self.penalty {
            self.nextGoal[player.get_team_id() as usize] = 0;
        } else if 1 == self.penalty {
            if self.nextGoal[player.get_team_id() as usize] > 0 {
                self.nextGoal[player.get_team_id() as usize] -= 1;
            }
        }
    }
}

impl CGameModeTrait for CGM_Race {
    fn gm(&self) -> &CGameMode {
        &self.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init(&mut self) {
        gm::cgamemode_init(self);

        unsafe {
            self.quantity = game_values.gamemodesettings.race.quantity;
            if self.quantity < 2 {
                self.quantity = 2;
                game_values.gamemodesettings.race.quantity = self.quantity;
            }

            self.penalty = game_values.gamemodesettings.race.penalty;
            if self.penalty < 0 || self.penalty > 2 {
                self.penalty = 0;
                game_values.gamemodesettings.race.penalty = self.penalty;
            }

            for iRaceGoal in 0..self.quantity {
                objectcontainer[2].add(Ptr::new_box(OMO_RaceGoal::new(Ptr::from_mut(&mut rm.spr_racegoal), iRaceGoal)));
            }
        }

        for iPlayer in 0..4 {
            self.nextGoal[iPlayer] = 0;
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        self.penalize_race_goals(other);
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        self.penalize_race_goals(player);
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(1 + if iType == 5 { 1 } else { 0 });

            //Don't end the game if the goal is infinite
            if self.goal == -1 {
                return;
            }

            if player.score().score >= self.goal {
                player.score().set_score(self.goal);
                self.winningteam = player.get_team_id();
                self.gameover = true;

                remove_players_but_team(self.winningteam);
                setup_score_board(false);
                show_score_board();
            } else if player.score().score as i32 >= self.goal as i32 - 1 && !self.playedwarningsound {
                self.playwarningsound();
            }
        }
    }
}
