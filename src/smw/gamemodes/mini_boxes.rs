//! Port of src/smw/gamemodes/MiniBoxes.cpp

use crate::common::game_mode::{game_mode_boxes_minigame, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, remove_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{score, score_cnt};
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::CPlayer;
use std::any::Any;

/*
TODO
1) Create new box object that breaks on contact and either gives a coin/bonus/penalty
2) Create levels approprate for distributing boxes onto
3) Add level hazards like thwomps, podobos, fireballs
4) Add hurry up that kicks in after 3 minutes of play that adds more coins or lowers bar to win
*/

//Boxes Bonus Mini Game (used in world mode)
//Try to collect all coins from boxes and players
pub struct CGM_Boxes_MiniGame {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Boxes_MiniGame => cgame_mode: CGameMode);

impl CGM_Boxes_MiniGame {
    pub fn new() -> Self {
        let mut this = CGM_Boxes_MiniGame { cgame_mode: CGameMode::new() };
        this.goal = 10;
        this.gamemode = game_mode_boxes_minigame;

        this.setup_mode_strings("Boxes Minigame", "Lives", 5);
        this
    }

    fn release_coin(&mut self, mut player: Ptr<CPlayer>) {
        if player.score().subscore[0] > 0 {
            player.score().subscore[0] -= 1;

            let pos = Vec2s::new((player.center_x() as i32 - 16) as i16, (player.center_y() as i32 - 16) as i16);

            let speed: f32 = 7.0f32 + RANDOM_INT(9) as f32 / 2.0f32;
            let angle: f32 = -(RANDOM_INT(314) as f32) / 100.0f32;
            let vel = Vec2f::new(speed * angle.cos(), speed * angle.sin());

            unsafe {
                if_sound_on_play(&mut rm.sfx_coin);

                objectcontainer[1].add(Ptr::new_box(MO_Coin::new(Ptr::from_mut(&mut rm.spr_coin), vel, pos, 2, -1, 2, 30, false)));
            }
        }
    }

    fn release_all_coins_from_team(&mut self, mut player: Ptr<CPlayer>) {
        while player.score().subscore[0] > 0 {
            self.release_coin(player);
        }
    }
}

impl CGameModeTrait for CGM_Boxes_MiniGame {
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

        unsafe {
            for iScore in 0..score_cnt as usize {
                score[iScore].set_score(self.goal);
                score[iScore].subscore[0] = 0;
            }
        }
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
        }
    }

    fn playerkilledplayer(&mut self, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        self.release_coin(other);

        other.score().adjust_score(-1);

        if !self.playedwarningsound {
            let mut countscore: i16 = 0;
            unsafe {
                for k in 0..score_cnt as usize {
                    if Ptr::from_mut(inflictor.score()) == score[k] {
                        continue;
                    }

                    countscore = countscore.wrapping_add(score[k].score);
                }
            }

            if countscore <= 2 {
                self.playwarningsound();
            }
        }

        if other.score().score <= 0 {
            self.release_all_coins_from_team(other);
            remove_team(other.get_team_id());
            return PlayerKillType::Removed;
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, mut player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        self.release_coin(player);

        player.score().adjust_score(-1);

        if !self.playedwarningsound {
            let mut countscore: i16 = 0;
            let mut playwarning = false;
            unsafe {
                for j in 0..score_cnt {
                    for k in 0..score_cnt {
                        if j == k {
                            continue;
                        }

                        countscore = countscore.wrapping_add(score[k as usize].score);
                    }

                    if countscore <= 2 {
                        playwarning = true;
                        break;
                    }

                    countscore = 0;
                }
            }

            if playwarning {
                self.playwarningsound();
            }
        }

        if player.score().score <= 0 {
            self.release_all_coins_from_team(player);
            remove_team(player.get_team_id());
            return PlayerKillType::Removed;
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if self.gameover {
            return;
        }

        player.score().adjust_score(iType);
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if player.score().subscore[0] >= 5 {
            player.score().subscore[0] = 5;

            self.winningteam = player.get_team_id();
            self.gameover = true;

            remove_players_but_team(self.winningteam);
            setup_score_board(false);
            show_score_board();
        } else if player.score().subscore[0] >= 4 && !self.playedwarningsound {
            self.playwarningsound();
        }

        PlayerKillType::Normal
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
