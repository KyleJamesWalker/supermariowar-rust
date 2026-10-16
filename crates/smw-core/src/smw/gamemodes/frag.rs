//! Port of src/smw/gamemodes/Frag.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{DeathStyle, ScoringStyle};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::player::CPlayer;

//Fraglimit
pub struct CGM_Frag {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Frag => cgame_mode: CGameMode);

//fraglimit:
impl CGM_Frag {
    pub fn new() -> Self {
        let mut this = CGM_Frag { cgame_mode: CGameMode::new() };
        this.goal = 20;
        this.gamemode = game_mode_frag;

        this.setup_mode_strings("Frag Limit", "Kills", 5);
        this
    }
}

impl CGameModeTrait for CGM_Frag {
    crate::impl_cgamemode_plumbing!();

    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledplayer(g, self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        cgm_frag_playerextraguy(g, self, player, iType)
    }
    fn check_winner(&mut self, g: &mut Game, player: PlayerId) -> PlayerKillType {
        cgm_frag_check_winner(g, self, player)
    }
}

pub fn cgm_frag_playerkilledplayer<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut inflictor: PlayerId, mut other: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    if this.gm().gameover {
        return PlayerKillType::Normal;
    }

    //Don't score if "sumo" style scoring is turned on
    if g.game_values.gamemode.gamemode != game_mode_frag || g.game_values.gamemodesettings.frag.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
        //Penalize killing your team mates
        if g.player(inflictor).get_team_id() == g.player(other).get_team_id() {
            g.player(inflictor).score().adjust_score(g, -1);
        } else {
            g.player(inflictor).score().adjust_score(g, 1);
        }
    }

    let iRet = this.check_winner(g, inflictor);

    if g.game_values.gamemode.gamemode == game_mode_frag && g.game_values.gamemodesettings.frag.style == DeathStyle::Shield {
        if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
        g.player(other).shield().reset(g);
        return PlayerKillType::NonKill;
    }

    iRet
}

pub fn cgm_frag_playerkilledself<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    cgamemode_playerkilledself(g, this, player, style);

    if !this.gm().gameover {
        g.player(player).score().adjust_score(g, -1);

        if g.game_values.gamemode.gamemode == game_mode_frag && g.game_values.gamemodesettings.frag.style == DeathStyle::Shield {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            g.player(player).shield().reset(g);
            return PlayerKillType::NonKill;
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_frag_playerextraguy<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, iType: i16) {
    if !this.gm().gameover {
        g.player(player).score().adjust_score(g, iType);
        this.check_winner(g, player);
    }
}

pub fn cgm_frag_check_winner<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId) -> PlayerKillType {
    let this = this.gm_mut();
    if this.goal > -1 {
        if g.player(player).score().score >= this.goal {
            g.player(player).score().set_score(g, this.goal);
            this.winningteam = g.player(player).get_team_id();
            this.gameover = true;

            remove_players_but_team(g, this.winningteam);
            setup_score_board(g, false);
            show_score_board(g);

            return PlayerKillType::Removed;
        } else if g.player(player).score().score as i32 >= this.goal as i32 - 2 && !this.playedwarningsound {
            this.playwarningsound(g);
        }
    }

    PlayerKillType::Normal
}
