//! Port of src/smw/gamemodes/Jail.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::JailStyle;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::frag::{cgm_frag_check_winner, cgm_frag_playerkilledself, CGM_Frag};
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::main::{players, score, score_cnt};
use crate::smw::player::CPlayer;

//Similar to frag limit but:
//When a player is killed by another player, they get a "jail" marker
//Jailed players move slowly
//Jailed players can be freed by teammates
//If all players on a team are jailed, bonus kill goes to other team
//Similar to frag limit, but players get bonus frags for the number of players they have "owned"
pub struct CGM_Jail {
    pub cgm_frag: CGM_Frag,
}
impl_base!(CGM_Jail => cgm_frag: CGM_Frag);

//Owned:
//Frag limit death match, but players get bonus frags for the number of players they have "owned"
impl CGM_Jail {
    pub fn new() -> Self {
        let mut this = CGM_Jail { cgm_frag: CGM_Frag::new() };
        this.gamemode = game_mode_jail;
        this.goal = 20;
        this.szModeName = "Jail".to_string();
        this
    }
}

impl CGameModeTrait for CGM_Jail {
    crate::impl_cgamemode_plumbing!();

    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_jail_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_jail_playerextraguy(self, player, iType)
    }
    fn check_winner(&mut self, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_frag_check_winner(self, player)
    }
}

unsafe fn emplace_poof(player: Ptr<CPlayer>) {
    eyecandy[2].emplace(EC_SingleAnimation::new(
        Ptr::from_mut(&mut rm.spr_poof),
        (player.center_x() as i32 - 24) as i16,
        (player.center_y() as i32 - 24) as i16,
        4,
        5,
    ));
}

pub fn cgm_jail_playerkilledplayer(this: &mut CGM_Jail, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    if !this.gameover {
        unsafe {
            //Penalize killing your teammates
            if inflictor.get_team_id() == other.get_team_id() {
                inflictor.score().adjust_score(-1);
            } else {
                inflictor.score().adjust_score(1);
                let mut inflictor2 = inflictor;
                inflictor.jail().escape(&mut *inflictor2);

                other.jail().lock_in_by(&mut *inflictor2);

                //Apply rules for "Classic" jail
                if game_values.gamemodesettings.jail.style == JailStyle::Classic {
                    let mut jailedteams: [i16; 4] = [0; 4];

                    for i in 0..score_cnt {
                        jailedteams[i as usize] = game_values.teamcounts[i as usize];
                    }

                    //Figure out which teams have been jailed
                    for i in 0..players.len() {
                        let mut player = players[i];
                        if player.jail().is_active() {
                            jailedteams[player.get_team_id() as usize] -= 1;
                        }
                    }

                    //Determine if a single team is the only one not completely jailed
                    let mut iTeamPoint: i16 = -1;
                    for i in 0..score_cnt {
                        if jailedteams[i as usize] == 0 {
                            continue;
                        }

                        if iTeamPoint < 0 {
                            iTeamPoint = i;
                        } else {
                            iTeamPoint = -1;
                            break;
                        }
                    }

                    //if only a single team has not been jailed, award points
                    if iTeamPoint >= 0 {
                        let mut numjailedplayers: i16 = 0;

                        for i in 0..players.len() {
                            let mut player = players[i];
                            //If they weren't just the one killed and they were jailed, give them a transform cloud
                            if player != other && player.jail().is_active() {
                                emplace_poof(player);
                                if_sound_on_play(&mut rm.sfx_transform);
                            }

                            if player.jail().is_active() && player.get_team_id() != iTeamPoint {
                                numjailedplayers += 1;
                            }

                            player.jail().timer = 0;
                        }

                        //Give extra bonus score for being on the non-jailed team
                        if numjailedplayers > 1 {
                            score[iTeamPoint as usize].adjust_score(1);
                        }
                    }
                }
                //Apply rules for "Owned" jail
                else if game_values.gamemodesettings.jail.style == JailStyle::Owned {
                    let mut jailedteams: [i16; 4] = [-1, -1, -1, -1];

                    //Figure out which teams have been jailed
                    for i in 0..players.len() {
                        let mut player = players[i];
                        let piMarker = &mut jailedteams[player.get_team_id() as usize];

                        if *piMarker == -2 {
                            continue;
                        }

                        if !player.jail().is_active() {
                            *piMarker = -2; //Flag that the team is not completely jailed
                        } else if *piMarker == -1 {
                            *piMarker = player.jail().owner_teamID as i16;
                        } else if *piMarker != player.jail().owner_teamID as i16 {
                            *piMarker = -2; //Flag means team is not completely jailed or jailed by different teams
                        }
                    }

                    //Determine if a single team is the only one not completely jailed
                    let mut iTeamPoint: i16 = -1;
                    for i in 0..score_cnt {
                        let mut iJailOwner: i16 = -1;
                        for j in 0..score_cnt {
                            if i == j {
                                continue;
                            }

                            //Other team is not completely jailed or jailed by different teams
                            if jailedteams[j as usize] == -2 {
                                iJailOwner = -1;
                                break;
                            }

                            if iJailOwner == -1 {
                                iJailOwner = jailedteams[j as usize];
                            } else if iJailOwner != jailedteams[j as usize] {
                                //Not all teams were jailed by same team
                                iJailOwner = -1;
                                break;
                            }
                        }

                        if iJailOwner >= 0 {
                            iTeamPoint = iJailOwner;
                            break;
                        }
                    }

                    //if only a single team has not been jailed, award points
                    if iTeamPoint >= 0 {
                        let mut numjailedplayers: i16 = 0;

                        for i in 0..players.len() {
                            let mut player = players[i];
                            if player.jail().is_active() && player.get_team_id() != iTeamPoint {
                                numjailedplayers += 1;
                            }
                        }

                        //Give extra bonus score for being on the non-jailed team
                        if numjailedplayers > 1 {
                            score[iTeamPoint as usize].adjust_score(1);

                            //Release other teams if a bonus was awarded for locking them up
                            for i in 0..players.len() {
                                let mut player = players[i];
                                //Don't release players that were not jailed by this team
                                if player.jail().owner_teamID as i16 != iTeamPoint {
                                    continue;
                                }

                                //If they weren't just the one killed and they were jailed, give them a transform cloud
                                if player != other && player.jail().is_active() {
                                    emplace_poof(player);
                                    if_sound_on_play(&mut rm.sfx_transform);
                                }

                                player.jail().timer = 0;
                            }
                        }
                    }
                }
            }
        }

        //Don't end the game if the goal is infinite
        if this.goal == -1 {
            return PlayerKillType::Normal;
        }

        if inflictor.score().score >= this.goal {
            this.winningteam = inflictor.get_team_id();
            this.gameover = true;

            remove_players_but_team(this.winningteam);

            setup_score_board(false);
            show_score_board();

            return PlayerKillType::Removed;
        } else if inflictor.score().score as i32 >= this.goal as i32 - 3 && !this.playedwarningsound {
            this.playwarningsound();
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_jail_playerextraguy(this: &mut CGM_Jail, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gameover {
        player.score().adjust_score(iType);
        player.jail().timer = 0;

        //Don't end the game if the goal is infinite
        if this.goal == -1 {
            return;
        }

        if player.score().score >= this.goal {
            player.score().set_score(this.goal);
            this.winningteam = player.get_team_id();
            this.gameover = true;

            remove_players_but_team(this.winningteam);
            setup_score_board(false);
            show_score_board();
        } else if player.score().score as i32 >= this.goal as i32 - 3 && !this.playedwarningsound {
            this.playwarningsound();
        }
    }
}
