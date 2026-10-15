//! Port of src/smw/gamemodes/Health.cpp

use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::{cgm_classic_init, cgm_classic_playerkilledplayer, cgm_classic_playerkilledself, CGM_Classic};

use crate::smw::player::CPlayer;

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

        unsafe {
            for iScore in 0..game().score_cnt {
                let mut s = g.score[iScore as usize];
                s.subscore[0] = g.game_values.gamemodesettings.health.startlife;
                s.subscore[1] = s.subscore[0];
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if style == KillStyle::Bobomb || style == KillStyle::Bomb || style == KillStyle::IceBlast {
            other.score().subscore[0] -= 2;
        } else {
            other.score().subscore[0] -= 1;
        }

        if other.score().subscore[0] <= 0 {
            other.score().subscore[0] = 0;
            let iRet = cgm_classic_playerkilledplayer(g, &mut self.cgm_classic, inflictor, other, style);

            if iRet == PlayerKillType::Normal {
                other.score().subscore[0] = other.score().subscore[1];
            }

            return iRet;
        } else {
            unsafe {
                if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            }
            other.shield().reset(g);
        }

        PlayerKillType::NonKill
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if self.gameover {
            return PlayerKillType::Normal;
        }

        if style == KillStyle::Bobomb || style == KillStyle::Bomb || style == KillStyle::Environment {
            player.score().subscore[0] -= 2;
        } else {
            player.score().subscore[0] -= 1;
        }

        if player.score().subscore[0] <= 0 {
            player.score().subscore[0] = 0;
            let iRet = cgm_classic_playerkilledself(g, &mut self.cgm_classic, player, style);

            if iRet == PlayerKillType::Normal {
                player.score().subscore[0] = player.score().subscore[1];
            }

            return iRet;
        } else {
            unsafe {
                if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            }
            player.shield().reset(g);
        }

        PlayerKillType::NonKill
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().subscore[0] += iType;

            if player.score().subscore[0] > player.score().subscore[1] {
                player.score().subscore[0] = player.score().subscore[1];
            }
        }
    }
}
