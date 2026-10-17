//! Port of src/smw/gamemodes/Classic.cpp

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

//mariowar classic
pub struct CGM_Classic {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Classic => cgame_mode: CGameMode);

//mariowar (x lives - counting down)
impl CGM_Classic {
    pub fn new() -> Self {
        let mut this = CGM_Classic { cgame_mode: CGameMode::new() };
        this.goal = 10;
        this.gamemode = game_mode_classic;

        this.setup_mode_strings("Classic", "Lives", 5);
        this
    }
}

impl CGameModeTrait for CGM_Classic {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgm_classic_init(g, self)
    }
    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledplayer(g, self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        cgm_classic_playerextraguy(g, self, player, iType)
    }
}

pub fn cgm_classic_init(g: &mut Game, this: &mut CGM_Classic) {
        let mut gp = g.ptr();
    cgamemode_init(g, this);

    this.fReverseScoring = this.goal == -1;

    for iScore in 0..g.score_cnt {
        if this.fReverseScoring {
            gp.score[iScore as usize].set_score(g, 0);
        } else {
            gp.score[iScore as usize].set_score(g, this.goal);
        }
    }
}

pub fn cgm_classic_playerkilledplayer(g: &mut Game, this: &mut CGM_Classic, mut inflictor: PlayerId, mut other: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    if this.gameover {
        return PlayerKillType::Normal;
    }

    //If we are playing classic "sumo" mode, then only score hazard kills
    if g.mode().gamemode != game_mode_classic || g.game_values.gamemodesettings.classic.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
        if this.fReverseScoring {
            g.player(other).score(g).adjust_score(g, 1);
        } else {
            g.player(other).score(g).adjust_score(g, -1);

            if !this.playedwarningsound {
                let mut countscore: i16 = 0;
                for k in 0..g.score_cnt {
                    if Ptr::from_mut(g.player(inflictor).score(g)) == g.score[k as usize] {
                        continue;
                    }

                    countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
                }

                if countscore <= 2 {
                    this.playwarningsound(g);
                }
            }

            if g.player(other).score(g).score <= 0 {
                remove_team(g, g.player(other).get_team_id());
                return PlayerKillType::Removed;
            }
        }
    }

    if g.mode().gamemode == game_mode_classic && g.game_values.gamemodesettings.classic.style == DeathStyle::Shield {
        if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
        g.player(other).shield().reset(g);
        return PlayerKillType::NonKill;
    }

    PlayerKillType::Normal
}

pub fn cgm_classic_playerkilledself(g: &mut Game, this: &mut CGM_Classic, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    cgamemode_playerkilledself(g, this, player, style);

    if !this.gameover {
        if this.fReverseScoring {
            g.player(player).score(g).adjust_score(g, 1);
        } else {
            g.player(player).score(g).adjust_score(g, -1);

            if !this.playedwarningsound {
                let mut countscore: i16 = 0;
                let mut playwarning = false;
                for j in 0..g.score_cnt {
                    for k in 0..g.score_cnt {
                        if j == k {
                            continue;
                        }

                        countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
                    }

                    if countscore <= 2 {
                        playwarning = true;
                        break;
                    }

                    countscore = 0;
                }

                if playwarning {
                    this.playwarningsound(g);
                }
            }

            if g.player(player).score(g).score <= 0 {
                remove_team(g, g.player(player).get_team_id());
                return PlayerKillType::Removed;
            }
        }

        if g.mode().gamemode == game_mode_classic && g.game_values.gamemodesettings.classic.style == DeathStyle::Shield {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            g.player(player).shield().reset(g);
            return PlayerKillType::NonKill;
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_classic_playerextraguy(g: &mut Game, this: &mut CGM_Classic, mut player: PlayerId, iType: i16) {
    if !this.gameover {
        if this.fReverseScoring {
            g.player(player).score(g).adjust_score(g, -iType);
        } else {
            g.player(player).score(g).adjust_score(g, iType);
        }
    }
}
