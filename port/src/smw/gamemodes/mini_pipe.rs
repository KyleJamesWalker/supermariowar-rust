//! Port of src/smw/gamemodes/MiniPipe.cpp

use crate::common::game_mode::{game_mode_pipe_minigame, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::score_cnt;
use crate::smw::objects::overmap::wo_pipe_bonus::OMO_PipeBonus;
use crate::smw::objects::overmap::wo_pipe_coin::OMO_PipeCoin;
use crate::smw::player::CPlayer;
use std::any::Any;

//Pipe Bonus Mini Game (used in world mode)
//Collect coins and powerups that come out of a pipe
pub struct CGM_Pipe_MiniGame {
    pub cgame_mode: CGameMode,

    iNextItemTimer: i16,
    iBonusTimer: i16,
    iBonusType: i16,
    iBonusTeam: i16,

    fSlowdown: bool,
}
impl_base!(CGM_Pipe_MiniGame => cgame_mode: CGameMode);

impl CGM_Pipe_MiniGame {
    pub fn new() -> Self {
        let mut this = CGM_Pipe_MiniGame { cgame_mode: CGameMode::new(), iNextItemTimer: 0, iBonusTimer: 0, iBonusType: 0, iBonusTeam: 0, fSlowdown: false };
        this.goal = 50;
        this.gamemode = game_mode_pipe_minigame;

        this.setup_mode_strings("Pipe Minigame", "Points", 0);
        this
    }

    pub fn set_bonus(&mut self, iType: i16, iTimer: i16, iTeamID: i16) {
        self.iBonusType = iType;

        //This is the random bonus
        if self.iBonusType == 5 {
            self.iBonusType = (RANDOM_INT(4) + 1) as i16;
        }

        if self.iBonusType == 4 {
            self.fSlowdown = true;
        }

        self.iBonusTimer = iTimer;
        self.iBonusTeam = iTeamID;

        unsafe {
            if self.iBonusType == 3 {
                if_sound_on_play(&mut rm.sfx_powerdown);
            } else {
                if_sound_on_play(&mut rm.sfx_collectpowerup);
            }
        }
    }

    pub fn is_slowdown(&self) -> bool {
        self.fSlowdown
    }
}

impl CGameModeTrait for CGM_Pipe_MiniGame {
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

        self.fReverseScoring = false;

        self.iNextItemTimer = 0;
        self.iBonusTimer = 0;
        self.iBonusType = 0;
        self.iBonusTeam = 0;
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
            return;
        }

        unsafe {
            self.iNextItemTimer -= 1;
            if self.iNextItemTimer <= 0 {
                let velx: f32 = (RANDOM_INT(21) - 10) as f32 / 2.0f32;
                let vely: f32 = -(RANDOM_INT(11) as f32 / 2.0f32 + 7.0f32);
                let vel = Vec2f::new(velx, vely);
                let pos = Vec2s::new(304, 256);

                if self.iBonusType == 0 || self.iBonusType == 2 || self.iBonusType == 4 {
                    if self.iBonusType == 2 {
                        self.iNextItemTimer = (RANDOM_INT(10) + 10) as i16;
                    } else {
                        self.iNextItemTimer = (RANDOM_INT(20) + 25) as i16;
                    }

                    let iRandPowerup: i16 = RANDOM_INT(50) as i16;
                    if self.iBonusType == 0 && iRandPowerup < 5 {
                        //bonuses
                        objectcontainer[1].add(Ptr::new_box(OMO_PipeBonus::new(Ptr::from_mut(&mut rm.spr_pipegamebonus), vel, pos, iRandPowerup, 620, 15)));
                    } else if iRandPowerup < 10 {
                        //fireballs
                        objectcontainer[1].add(Ptr::new_box(OMO_PipeBonus::new(Ptr::from_mut(&mut rm.spr_pipegamebonus), vel, pos, 5, 0, 15)));
                    } else {
                        //coins
                        let iRandCoin: i16 = RANDOM_INT(20) as i16;
                        objectcontainer[1].add(Ptr::new_box(OMO_PipeCoin::new(Ptr::from_mut(&mut rm.spr_coin), vel, pos, -1, if iRandCoin < 16 { 2 } else if iRandCoin < 19 { 0 } else { 1 }, 15)));
                    }
                } else if self.iBonusType == 1 {
                    self.iNextItemTimer = (RANDOM_INT(10) + 10) as i16;

                    let mut iRandTeam: i16 = RANDOM_INT(score_cnt as i32 + 2) as i16;

                    //Give an advantage to the team that got the item
                    if iRandTeam >= score_cnt {
                        iRandTeam = self.iBonusTeam;
                    }

                    let iRandPlayer: i16 = game_values.teamids[iRandTeam as usize][RANDOM_INT(game_values.teamcounts[iRandTeam as usize] as i32) as usize];

                    objectcontainer[1].add(Ptr::new_box(OMO_PipeCoin::new(Ptr::from_mut(&mut rm.spr_coin), vel, pos, iRandTeam, game_values.colorids[iRandPlayer as usize], 15)));
                } else if self.iBonusType == 3 {
                    self.iNextItemTimer = (RANDOM_INT(5) + 10) as i16;
                    objectcontainer[1].add(Ptr::new_box(OMO_PipeCoin::new(Ptr::from_mut(&mut rm.spr_coin), vel, pos, -1, 0, 15)));
                }
            }
        }

        if self.iBonusTimer > 0 {
            self.iBonusTimer -= 1;
            if self.iBonusTimer <= 0 {
                self.iBonusType = 0;
                self.fSlowdown = false;
            }
        }
    }

    fn playerkilledplayer(&mut self, _player: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        //other.Score().AdjustScore(-2);
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, _player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        //player.Score().AdjustScore(-2);
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(iType);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal > -1 {
            if player.score().score >= self.goal {
                player.score().set_score(self.goal);

                self.winningteam = player.get_team_id();
                self.gameover = true;

                remove_players_but_team(self.winningteam);
                setup_score_board(false);
                show_score_board();
            } else if player.score().score as i32 >= self.goal as i32 - 5 && !self.playedwarningsound {
                self.playwarningsound();
            }
        }

        PlayerKillType::Normal
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
