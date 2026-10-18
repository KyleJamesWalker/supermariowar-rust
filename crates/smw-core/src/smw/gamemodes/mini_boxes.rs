//! Port of src/smw/gamemodes/MiniBoxes.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_boxes_minigame, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::global_constants::{ticks, vel};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, remove_team, setup_score_board, show_score_board};

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
#[derive(Clone)]
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

    fn release_coin(&mut self, g: &mut Game, mut player: PlayerId) {
        let mut gp = g.ptr();
        if g.player(player).score(g).subscore[0] > 0 {
            g.player(player).score(g).subscore[0] -= 1;

            let pos = Vec2s::new((g.player(player).center_x() as i32 - 16) as i16, (g.player(player).center_y() as i32 - 16) as i16);

            let speed: f32 = vel(7.0f32 + RANDOM_INT(gp.get(), 9) as f32 / 2.0f32);
            let angle: f32 = -(RANDOM_INT(gp.get(), 314) as f32) / 100.0f32;
            let vel = Vec2f::new(speed * cosf(angle), speed * sinf(angle));

            if_sound_on_play(g, &mut gp.rm.sfx_coin);

            let obj = Ptr::new_box(MO_Coin::new(g, gp.get().rm.spr_coin, vel, pos, 2, -1, 2, ticks(30) as i16, false));
            gp.get().objectcontainer[1].add(g, obj);
        }
    }

    fn release_all_coins_from_team(&mut self, g: &mut Game, mut player: PlayerId) {
        while g.player(player).score(g).subscore[0] > 0 {
            self.release_coin(g, player);
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

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);

        self.fReverseScoring = false;

        for iScore in 0..g.score_cnt as usize {
            gp.score[iScore].set_score(g, self.goal);
            g.score[iScore].subscore[0] = 0;
        }
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: PlayerId, mut other: PlayerId, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        self.release_coin(g, other);

        g.player(other).score(g).adjust_score(g, -1);

        if !self.playedwarningsound {
            let mut countscore: i16 = 0;
            for k in 0..g.score_cnt as usize {
                if g.player(inflictor).score_slot == (k) as usize {
                    continue;
                }

                countscore = countscore.wrapping_add(g.score[k].score);
            }

            if countscore <= 2 {
                self.playwarningsound(g);
            }
        }

        if g.player(other).score(g).score <= 0 {
            self.release_all_coins_from_team(g, other);
            remove_team(g, g.player(other).get_team_id());
            return PlayerKillType::Removed;
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: PlayerId, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        self.release_coin(g, player);

        g.player(player).score(g).adjust_score(g, -1);

        if !self.playedwarningsound {
            let mut countscore: i16 = 0;
            let mut playwarning = false;
            for j in 0..g.score_cnt {
                for k in 0..g.score_cnt {
                    if j == k {
                        continue;
                    }

                    countscore = countscore.wrapping_add(g.score[k as usize].score);
                }

                if countscore <= 2 {
                    playwarning = true;
                    break;
                }

                countscore = 0;
            }

            if playwarning {
                self.playwarningsound(g);
            }
        }

        if g.player(player).score(g).score <= 0 {
            self.release_all_coins_from_team(g, player);
            remove_team(g, g.player(player).get_team_id());
            return PlayerKillType::Removed;
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if self.gameover {
            return;
        }

        g.player(player).score(g).adjust_score(g, iType);
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if g.player(player).score(g).subscore[0] >= 5 {
            g.player(player).score(g).subscore[0] = 5;

            self.winningteam = g.player(player).get_team_id();
            self.gameover = true;

            remove_players_but_team(g, self.winningteam);
            setup_score_board(g, false);
            show_score_board(g);
        } else if g.player(player).score(g).subscore[0] >= 4 && !self.playedwarningsound {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
