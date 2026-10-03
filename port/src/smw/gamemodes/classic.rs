//! Port of src/smw/gamemodes/Classic.cpp

use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{DeathStyle, ScoringStyle};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::main::{score, score_cnt};
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

    fn init(&mut self) {
        cgm_classic_init(self)
    }
    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_classic_playerextraguy(self, player, iType)
    }
}

pub fn cgm_classic_init(this: &mut CGM_Classic) {
    cgamemode_init(this);

    this.fReverseScoring = this.goal == -1;

    unsafe {
        for iScore in 0..score_cnt {
            if this.fReverseScoring {
                score[iScore as usize].set_score(0);
            } else {
                score[iScore as usize].set_score(this.goal);
            }
        }
    }
}

pub fn cgm_classic_playerkilledplayer(this: &mut CGM_Classic, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    if this.gameover {
        return PlayerKillType::Normal;
    }

    unsafe {
        //If we are playing classic "sumo" mode, then only score hazard kills
        if game_values.gamemode.gamemode != game_mode_classic || game_values.gamemodesettings.classic.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
            if this.fReverseScoring {
                other.score().adjust_score(1);
            } else {
                other.score().adjust_score(-1);

                if !this.playedwarningsound {
                    let mut countscore: i16 = 0;
                    for k in 0..score_cnt {
                        if Ptr::from_mut(inflictor.score()) == score[k as usize] {
                            continue;
                        }

                        countscore = (countscore as i32 + score[k as usize].score as i32) as i16;
                    }

                    if countscore <= 2 {
                        this.playwarningsound();
                    }
                }

                if other.score().score <= 0 {
                    remove_team(other.get_team_id());
                    return PlayerKillType::Removed;
                }
            }
        }

        if game_values.gamemode.gamemode == game_mode_classic && game_values.gamemodesettings.classic.style == DeathStyle::Shield {
            if_sound_on_play(&mut rm.sfx_powerdown);
            other.shield().reset();
            return PlayerKillType::NonKill;
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_classic_playerkilledself(this: &mut CGM_Classic, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    cgamemode_playerkilledself(this, player, style);

    if !this.gameover {
        unsafe {
            if this.fReverseScoring {
                player.score().adjust_score(1);
            } else {
                player.score().adjust_score(-1);

                if !this.playedwarningsound {
                    let mut countscore: i16 = 0;
                    let mut playwarning = false;
                    for j in 0..score_cnt {
                        for k in 0..score_cnt {
                            if j == k {
                                continue;
                            }

                            countscore = (countscore as i32 + score[k as usize].score as i32) as i16;
                        }

                        if countscore <= 2 {
                            playwarning = true;
                            break;
                        }

                        countscore = 0;
                    }

                    if playwarning {
                        this.playwarningsound();
                    }
                }

                if player.score().score <= 0 {
                    remove_team(player.get_team_id());
                    return PlayerKillType::Removed;
                }
            }

            if game_values.gamemode.gamemode == game_mode_classic && game_values.gamemodesettings.classic.style == DeathStyle::Shield {
                if_sound_on_play(&mut rm.sfx_powerdown);
                player.shield().reset();
                return PlayerKillType::NonKill;
            }
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_classic_playerextraguy(this: &mut CGM_Classic, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gameover {
        if this.fReverseScoring {
            player.score().adjust_score(-iType);
        } else {
            player.score().adjust_score(iType);
        }
    }
}
