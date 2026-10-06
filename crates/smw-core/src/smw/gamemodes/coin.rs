//! Port of src/smw/gamemodes/Coin.cpp

use crate::common::game_mode::*;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::CPlayer;

pub struct CGM_Coins {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Coins => cgame_mode: CGameMode);

//Coin mode:
//Collect randomly appearing coins on map
//First one to set amount wins
impl CGM_Coins {
    pub fn new() -> Self {
        let mut this = CGM_Coins { cgame_mode: CGameMode::new() };
        this.goal = 20;
        this.gamemode = game_mode_coins;

        this.setup_mode_strings("Coin Collection", "Coins", 5);
        this
    }
}

impl CGameModeTrait for CGM_Coins {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgamemode_init(self);

        unsafe {
            if game_values.gamemodesettings.coins.quantity < 1 {
                game_values.gamemodesettings.coins.quantity = 1;
            }

            for _iCoin in 0..game_values.gamemodesettings.coins.quantity {
                objectcontainer[1].add(Ptr::new_box(MO_Coin::new(Ptr::from_mut(&mut rm.spr_coin), Vec2f::zero(), Vec2s::zero(), 2, 0, 0, 0, true)));
            }
        }
    }

    fn playerkilledplayer(&mut self, _player: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        unsafe {
            if game_values.gamemodesettings.coins.penalty {
                other.score().adjust_score(-1);
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, mut player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        unsafe {
            if game_values.gamemodesettings.coins.penalty {
                player.score().adjust_score(-1);
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(((iType as i32) << 1) as i16);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if !self.gameover && self.goal > -1 {
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
}
