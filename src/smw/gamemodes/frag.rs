//! Port of src/smw/gamemodes/Frag.cpp

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

    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_frag_playerextraguy(self, player, iType)
    }
    fn check_winner(&mut self, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_frag_check_winner(self, player)
    }
}

pub fn cgm_frag_playerkilledplayer<T: CGameModeTrait + ?Sized>(this: &mut T, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    if this.gm().gameover {
        return PlayerKillType::Normal;
    }

    unsafe {
        //Don't score if "sumo" style scoring is turned on
        if game_values.gamemode.gamemode != game_mode_frag || game_values.gamemodesettings.frag.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
            //Penalize killing your team mates
            if inflictor.get_team_id() == other.get_team_id() {
                inflictor.score().adjust_score(-1);
            } else {
                inflictor.score().adjust_score(1);
            }
        }

        let iRet = this.check_winner(inflictor);

        if game_values.gamemode.gamemode == game_mode_frag && game_values.gamemodesettings.frag.style == DeathStyle::Shield {
            if_sound_on_play(&mut rm.sfx_powerdown);
            other.shield().reset();
            return PlayerKillType::NonKill;
        }

        iRet
    }
}

pub fn cgm_frag_playerkilledself<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    cgamemode_playerkilledself(this, player, style);

    if !this.gm().gameover {
        player.score().adjust_score(-1);

        unsafe {
            if game_values.gamemode.gamemode == game_mode_frag && game_values.gamemodesettings.frag.style == DeathStyle::Shield {
                if_sound_on_play(&mut rm.sfx_powerdown);
                player.shield().reset();
                return PlayerKillType::NonKill;
            }
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_frag_playerextraguy<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gm().gameover {
        player.score().adjust_score(iType);
        this.check_winner(player);
    }
}

pub fn cgm_frag_check_winner<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>) -> PlayerKillType {
    let this = this.gm_mut();
    if this.goal > -1 {
        if player.score().score >= this.goal {
            player.score().set_score(this.goal);
            this.winningteam = player.get_team_id();
            this.gameover = true;

            remove_players_but_team(this.winningteam);
            setup_score_board(false);
            show_score_board();

            return PlayerKillType::Removed;
        } else if player.score().score as i32 >= this.goal as i32 - 2 && !this.playedwarningsound {
            this.playwarningsound();
        }
    }

    PlayerKillType::Normal
}
