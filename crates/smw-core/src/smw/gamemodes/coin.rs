//! Coin collection mode.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::CPlayer;

#[derive(Clone)]

/// Coin collection: collect randomly appearing coins; first team to the goal wins.
pub struct CGM_Coins {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Coins => cgame_mode: CGameMode);

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

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        cgamemode_init(g, self);

        if g.game_values.gamemodesettings.coins.quantity < 1 {
            g.game_values.gamemodesettings.coins.quantity = 1;
        }

        for _iCoin in 0..g.game_values.gamemodesettings.coins.quantity {
            let obj = Ptr::new_box(MO_Coin::new(g, gp.get().rm.spr_coin, Vec2f::zero(), Vec2s::zero(), 2, 0, 0, 0, true));
            gp.get().objectcontainer[1].add(g, obj);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _player: PlayerId, mut other: PlayerId, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if g.game_values.gamemodesettings.coins.penalty {
            g.player(other).score(g).adjust_score(g, -1);
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: PlayerId, _style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if g.game_values.gamemodesettings.coins.penalty {
            g.player(player).score(g).adjust_score(g, -1);
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, ((iType as i32) << 1) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if !self.gameover && self.goal > -1 {
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
}
