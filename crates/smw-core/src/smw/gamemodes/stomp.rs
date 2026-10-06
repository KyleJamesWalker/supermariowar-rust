//! Port of src/smw/gamemodes/Stomp.cpp

use crate::common::game_mode::*;
use crate::common::global_constants::NUMSTOMPENEMIES;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::moving::mo_cheep_cheep::MO_CheepCheep;
use crate::smw::objects::walkingenemy::we_buzzy_beetle::MO_BuzzyBeetle;
use crate::smw::objects::walkingenemy::we_goomba::MO_Goomba;
use crate::smw::objects::walkingenemy::we_koopa::MO_Koopa;
use crate::smw::objects::walkingenemy::we_spiny::MO_Spiny;
use crate::smw::player::CPlayer;

//Similar to coin mode but you have to smash the most goombas/cheeps/koopas
pub struct CGM_Stomp {
    pub cgame_mode: CGameMode,

    pub spawntimer: i16,
    pub iSelectedEnemy: i16,
    pub iEnemyWeightCount: i16,
}
impl_base!(CGM_Stomp => cgame_mode: CGameMode);

//Stomp mode:
//Kill randomly appearing goomobas on map
//First one to kill the limit wins
impl CGM_Stomp {
    pub fn new() -> Self {
        let mut this = CGM_Stomp { cgame_mode: CGameMode::new(), spawntimer: 0, iSelectedEnemy: 0, iEnemyWeightCount: 0 };
        this.gamemode = game_mode_stomp;
        this.setup_mode_strings("Stomp", "Kills", 10);
        this
    }

    fn spawn_enemy(&mut self) {
        unsafe {
            self.reset_spawn_timer();

            //If all weights were zero, then randomly choose an enemy
            if self.iEnemyWeightCount == 0 {
                self.iSelectedEnemy = RANDOM_INT(9) as i16;
            } else {
                //Otherwise randomly choose an enemy from the weighted list
                let iRandEnemy: i32 = RANDOM_INT(self.iEnemyWeightCount as i32) + 1;
                self.iSelectedEnemy = 0;
                let mut iWeightCount: i32 = game_values.gamemodesettings.stomp.enemyweight[self.iSelectedEnemy as usize] as i32;

                while iWeightCount < iRandEnemy {
                    self.iSelectedEnemy += 1;
                    iWeightCount += game_values.gamemodesettings.stomp.enemyweight[self.iSelectedEnemy as usize] as i32;
                }
            }

            if 0 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Goomba::new(Ptr::from_mut(&mut rm.spr_goomba), RANDOM_BOOL(), false)));
            } else if 1 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(Ptr::from_mut(&mut rm.spr_koopa), RANDOM_BOOL(), false, false, true)));
            } else if 2 == self.iSelectedEnemy {
                objectcontainer[2].add(Ptr::new_box(MO_CheepCheep::new(Ptr::from_mut(&mut rm.spr_cheepcheep))));
            } else if 3 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(Ptr::from_mut(&mut rm.spr_redkoopa), RANDOM_BOOL(), true, false, false)));
            } else if 4 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Spiny::new(Ptr::from_mut(&mut rm.spr_spiny), RANDOM_BOOL())));
            } else if 5 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_BuzzyBeetle::new(Ptr::from_mut(&mut rm.spr_buzzybeetle), RANDOM_BOOL())));
            } else if 6 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Goomba::new(Ptr::from_mut(&mut rm.spr_paragoomba), RANDOM_BOOL(), true)));
            } else if 7 == self.iSelectedEnemy {
                objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(Ptr::from_mut(&mut rm.spr_parakoopa), RANDOM_BOOL(), false, true, true)));
            } else {
                objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(Ptr::from_mut(&mut rm.spr_redparakoopa), RANDOM_BOOL(), true, true, true)));
            }
        }
    }

    fn reset_spawn_timer(&mut self) {
        unsafe {
            let rate = game_values.gamemodesettings.stomp.rate;
            self.spawntimer = ((RANDOM_INT(rate as i32) as i16) as i32 + rate as i32) as i16;
        }
    }
}

pub fn net_spawn_enemy() {
    unsafe {
        if let Some(this) = game_values.gamemode.as_any().downcast_mut::<CGM_Stomp>() {
            this.spawn_enemy();
        }
    }
}

impl CGameModeTrait for CGM_Stomp {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgamemode_init(self);
        self.reset_spawn_timer();

        self.iEnemyWeightCount = 0;
        unsafe {
            for iEnemy in 0..NUMSTOMPENEMIES as usize {
                self.iEnemyWeightCount += game_values.gamemodesettings.stomp.enemyweight[iEnemy];
            }
        }

        //if (iEnemyWeightCount == 0)
        //  iEnemyWeightCount = 1;
    }

    fn think(&mut self) {
        unsafe {
            if self.gameover {
                self.displayplayertext();
            } else {
                for i in 0..players.len() {
                    let player = players[i];
                    self.check_winner(player);
                }
            }

            if !self.gameover {
                //Randomly spawn enemies
                self.spawntimer -= 1;
                if self.spawntimer <= 0 && !net_random::event(Ev::StompEnemy, &[]) {
                    self.spawn_enemy();
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, _player: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, _player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(iType);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(self.goal);
            self.winningteam = player.get_team_id();
            self.gameover = true;

            setup_score_board(false);
            show_score_board();
            remove_players_but_team(self.winningteam);
            return PlayerKillType::Removed;
        } else if player.score().score as i32 >= self.goal as i32 - 2 && !self.playedwarningsound {
            self.playwarningsound();
        }

        PlayerKillType::Normal
    }
}
