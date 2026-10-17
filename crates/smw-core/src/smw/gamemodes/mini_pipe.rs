//! Port of src/smw/gamemodes/MiniPipe.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_pipe_minigame, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::objects::overmap::wo_pipe_bonus::OMO_PipeBonus;
use crate::smw::objects::overmap::wo_pipe_coin::OMO_PipeCoin;
use crate::smw::player::CPlayer;
use std::any::Any;

//Pipe Bonus Mini Game (used in world mode)
//Collect coins and powerups that come out of a pipe
#[derive(Clone)]
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

    pub fn set_bonus(&mut self, g: &mut Game, iType: i16, iTimer: i16, iTeamID: i16) {
        let mut gp = g.ptr();
        self.iBonusType = iType;

        //This is the random bonus
        if self.iBonusType == 5 {
            self.iBonusType = (RANDOM_INT(gp.get(), 4) + 1) as i16;
        }

        if self.iBonusType == 4 {
            self.fSlowdown = true;
        }

        self.iBonusTimer = iTimer;
        self.iBonusTeam = iTeamID;

        if self.iBonusType == 3 {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_collectpowerup);
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

    fn init(&mut self, g: &mut Game) {
        gm::cgamemode_init(g, self);

        self.fReverseScoring = false;

        self.iNextItemTimer = 0;
        self.iBonusTimer = 0;
        self.iBonusType = 0;
        self.iBonusTeam = 0;
    }

    fn think(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        self.iNextItemTimer -= 1;
        if self.iNextItemTimer <= 0 {
            let velx: f32 = (RANDOM_INT(gp.get(), 21) - 10) as f32 / 2.0f32;
            let vely: f32 = -(RANDOM_INT(gp.get(), 11) as f32 / 2.0f32 + 7.0f32);
            let vel = Vec2f::new(velx, vely);
            let pos = Vec2s::new(304, 256);

            if self.iBonusType == 0 || self.iBonusType == 2 || self.iBonusType == 4 {
                if self.iBonusType == 2 {
                    self.iNextItemTimer = (RANDOM_INT(gp.get(), 10) + 10) as i16;
                } else {
                    self.iNextItemTimer = (RANDOM_INT(gp.get(), 20) + 25) as i16;
                }

                let iRandPowerup: i16 = RANDOM_INT(gp.get(), 50) as i16;
                if self.iBonusType == 0 && iRandPowerup < 5 {
                    //bonuses
                    let obj = Ptr::new_box(OMO_PipeBonus::new(g, gp.get().rm.spr_pipegamebonus, vel, pos, iRandPowerup, 620, 15));
                    gp.get().objectcontainer[1].add(g, obj);
                } else if iRandPowerup < 10 {
                    //fireballs
                    let obj = Ptr::new_box(OMO_PipeBonus::new(g, gp.get().rm.spr_pipegamebonus, vel, pos, 5, 0, 15));
                    gp.get().objectcontainer[1].add(g, obj);
                } else {
                    //coins
                    let iRandCoin: i16 = RANDOM_INT(gp.get(), 20) as i16;
                    let obj = Ptr::new_box(OMO_PipeCoin::new(g, gp.get().rm.spr_coin, vel, pos, -1, if iRandCoin < 16 { 2 } else if iRandCoin < 19 { 0 } else { 1 }, 15));
                    gp.get().objectcontainer[1].add(g, obj);
                }
            } else if self.iBonusType == 1 {
                self.iNextItemTimer = (RANDOM_INT(gp.get(), 10) + 10) as i16;

                let mut iRandTeam: i16 = RANDOM_INT(gp.get(), g.score_cnt as i32 + 2) as i16;

                //Give an advantage to the team that got the item
                if iRandTeam >= g.score_cnt {
                    iRandTeam = self.iBonusTeam;
                }

                let iRandPlayer: i16 = g.game_values.teamids[iRandTeam as usize][RANDOM_INT(gp.get(), g.game_values.teamcounts[iRandTeam as usize] as i32) as usize];

                let obj = Ptr::new_box(OMO_PipeCoin::new(g, gp.get().rm.spr_coin, vel, pos, iRandTeam, gp.get().game_values.colorids[iRandPlayer as usize], 15));
                gp.get().objectcontainer[1].add(g, obj);
            } else if self.iBonusType == 3 {
                self.iNextItemTimer = (RANDOM_INT(gp.get(), 5) + 10) as i16;
                let obj = Ptr::new_box(OMO_PipeCoin::new(g, gp.get().rm.spr_coin, vel, pos, -1, 0, 15));
                gp.get().objectcontainer[1].add(g, obj);
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

    fn playerkilledplayer(&mut self, g: &mut Game, _player: PlayerId, _other: PlayerId, _style: KillStyle) -> PlayerKillType {
        //other.Score().AdjustScore(-2);
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, _player: PlayerId, _style: KillStyle) -> PlayerKillType {
        //player.Score().AdjustScore(-2);
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, iType);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if self.goal > -1 {
            if g.player(player).score(g).score >= self.goal {
                g.player(player).score(g).set_score(g, self.goal);

                self.winningteam = g.player(player).get_team_id();
                self.gameover = true;

                remove_players_but_team(g, self.winningteam);
                setup_score_board(g, false);
                show_score_board(g);
            } else if g.player(player).score(g).score as i32 >= self.goal as i32 - 5 && !self.playedwarningsound {
                self.playwarningsound(g);
            }
        }

        PlayerKillType::Normal
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
