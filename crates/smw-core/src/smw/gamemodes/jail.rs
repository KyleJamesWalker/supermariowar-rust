//! Port of src/smw/gamemodes/Jail.cpp

use crate::game::Game;
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

    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_jail_playerkilledplayer(g, self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: Ptr<CPlayer>, iType: i16) {
        cgm_jail_playerextraguy(g, self, player, iType)
    }
    fn check_winner(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_frag_check_winner(g, self, player)
    }
}

fn emplace_poof(g: &mut Game, player: Ptr<CPlayer>) {
    g.eyecandy[2].emplace(EC_SingleAnimation::new(
        Ptr::from_mut(&mut g.rm.spr_poof),
        (player.center_x() as i32 - 24) as i16,
        (player.center_y() as i32 - 24) as i16,
        4,
        5,
    ));
}

pub fn cgm_jail_playerkilledplayer(g: &mut Game, this: &mut CGM_Jail, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    if !this.gameover {
        //Penalize killing your teammates
        if inflictor.get_team_id() == other.get_team_id() {
            inflictor.score().adjust_score(g, -1);
        } else {
            inflictor.score().adjust_score(g, 1);
            let mut inflictor2 = inflictor;
            inflictor.jail().escape(g, &mut *inflictor2);

            other.jail().lock_in_by(g, &mut *inflictor2);

            //Apply rules for "Classic" jail
            if g.game_values.gamemodesettings.jail.style == JailStyle::Classic {
                let mut jailedteams: [i16; 4] = [0; 4];

                for i in 0..g.score_cnt {
                    jailedteams[i as usize] = g.game_values.teamcounts[i as usize];
                }

                //Figure out which teams have been jailed
                for i in 0..g.players.len() {
                    let mut player = g.players[i];
                    if player.jail().is_active() {
                        jailedteams[player.get_team_id() as usize] -= 1;
                    }
                }

                //Determine if a single team is the only one not completely jailed
                let mut iTeamPoint: i16 = -1;
                for i in 0..g.score_cnt {
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

                    for i in 0..g.players.len() {
                        let mut player = g.players[i];
                        //If they weren't just the one killed and they were jailed, give them a transform cloud
                        if player != other && player.jail().is_active() {
                            emplace_poof(g, player);
                            if_sound_on_play(g, &mut gp.rm.sfx_transform);
                        }

                        if player.jail().is_active() && player.get_team_id() != iTeamPoint {
                            numjailedplayers += 1;
                        }

                        player.jail().timer = 0;
                    }

                    //Give extra bonus score for being on the non-jailed team
                    if numjailedplayers > 1 {
                        gp.score[iTeamPoint as usize].adjust_score(g, 1);
                    }
                }
            }
            //Apply rules for "Owned" jail
            else if g.game_values.gamemodesettings.jail.style == JailStyle::Owned {
                let mut jailedteams: [i16; 4] = [-1, -1, -1, -1];

                //Figure out which teams have been jailed
                for i in 0..g.players.len() {
                    let mut player = g.players[i];
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
                for i in 0..g.score_cnt {
                    let mut iJailOwner: i16 = -1;
                    for j in 0..g.score_cnt {
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

                    for i in 0..g.players.len() {
                        let mut player = g.players[i];
                        if player.jail().is_active() && player.get_team_id() != iTeamPoint {
                            numjailedplayers += 1;
                        }
                    }

                    //Give extra bonus score for being on the non-jailed team
                    if numjailedplayers > 1 {
                        gp.score[iTeamPoint as usize].adjust_score(g, 1);

                        //Release other teams if a bonus was awarded for locking them up
                        for i in 0..g.players.len() {
                            let mut player = g.players[i];
                            //Don't release players that were not jailed by this team
                            if player.jail().owner_teamID as i16 != iTeamPoint {
                                continue;
                            }

                            //If they weren't just the one killed and they were jailed, give them a transform cloud
                            if player != other && player.jail().is_active() {
                                emplace_poof(g, player);
                                if_sound_on_play(g, &mut gp.rm.sfx_transform);
                            }

                            player.jail().timer = 0;
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

            remove_players_but_team(g, this.winningteam);

            setup_score_board(g, false);
            show_score_board(g);

            return PlayerKillType::Removed;
        } else if inflictor.score().score as i32 >= this.goal as i32 - 3 && !this.playedwarningsound {
            this.playwarningsound(g);
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_jail_playerextraguy(g: &mut Game, this: &mut CGM_Jail, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gameover {
        player.score().adjust_score(g, iType);
        player.jail().timer = 0;

        //Don't end the game if the goal is infinite
        if this.goal == -1 {
            return;
        }

        if player.score().score >= this.goal {
            player.score().set_score(g, this.goal);
            this.winningteam = player.get_team_id();
            this.gameover = true;

            remove_players_but_team(g, this.winningteam);
            setup_score_board(g, false);
            show_score_board(g);
        } else if player.score().score as i32 >= this.goal as i32 - 3 && !this.playedwarningsound {
            this.playwarningsound(g);
        }
    }
}
