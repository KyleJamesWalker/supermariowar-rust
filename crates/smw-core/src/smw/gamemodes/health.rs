//! Port of src/smw/gamemodes/Health.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::{cgm_classic_init, cgm_classic_playerkilledplayer, cgm_classic_playerkilledself, CGM_Classic};

use crate::smw::player::CPlayer;

#[derive(Clone)]

pub struct CGM_Health {
    pub cgm_classic: CGM_Classic,
}
impl_base!(CGM_Health => cgm_classic: CGM_Classic);

//mariowar (x lives - counting down)
impl CGM_Health {
    pub fn new() -> Self {
        let mut this = CGM_Health { cgm_classic: CGM_Classic::new() };
        this.goal = 5;
        this.gamemode = game_mode_health;

        this.setup_mode_strings("Health", "Lives", 1);
        this
    }
}

impl CGameModeTrait for CGM_Health {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgm_classic_init(g, &mut self.cgm_classic);

        for iScore in 0..g.score_cnt {
            let mut s = g.score[iScore as usize].as_ptr();
            s.subscore[0] = g.game_values.gamemodesettings.health.startlife;
            s.subscore[1] = s.subscore[0];
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, mut other: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if style == KillStyle::Bobomb || style == KillStyle::Bomb || style == KillStyle::IceBlast {
            g.player(other).score(g).subscore[0] -= 2;
        } else {
            g.player(other).score(g).subscore[0] -= 1;
        }

        if g.player(other).score(g).subscore[0] <= 0 {
            g.player(other).score(g).subscore[0] = 0;
            let iRet = cgm_classic_playerkilledplayer(g, &mut self.cgm_classic, inflictor, other, style);

            if iRet == PlayerKillType::Normal {
                g.player(other).score(g).subscore[0] = g.player(other).score(g).subscore[1];
            }

            return iRet;
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            g.player(other).shield().reset(g);
        }

        PlayerKillType::NonKill
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if style == KillStyle::Bobomb || style == KillStyle::Bomb || style == KillStyle::Environment {
            g.player(player).score(g).subscore[0] -= 2;
        } else {
            g.player(player).score(g).subscore[0] -= 1;
        }

        if g.player(player).score(g).subscore[0] <= 0 {
            g.player(player).score(g).subscore[0] = 0;
            let iRet = cgm_classic_playerkilledself(g, &mut self.cgm_classic, player, style);

            if iRet == PlayerKillType::Normal {
                g.player(player).score(g).subscore[0] = g.player(player).score(g).subscore[1];
            }

            return iRet;
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            g.player(player).shield().reset(g);
        }

        PlayerKillType::NonKill
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).subscore[0] += iType;

            if g.player(player).score(g).subscore[0] > g.player(player).score(g).subscore[1] {
                g.player(player).score(g).subscore[0] = g.player(player).score(g).subscore[1];
            }
        }
    }
}
