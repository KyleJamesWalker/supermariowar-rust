//! Port of src/smw/gamemodes/Stomp.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::global_constants::NUMSTOMPENEMIES;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
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

    fn spawn_enemy(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        self.reset_spawn_timer(g);

        //If all weights were zero, then randomly choose an enemy
        if self.iEnemyWeightCount == 0 {
            self.iSelectedEnemy = RANDOM_INT(gp.get(), 9) as i16;
        } else {
            //Otherwise randomly choose an enemy from the weighted list
            let iRandEnemy: i32 = RANDOM_INT(gp.get(), self.iEnemyWeightCount as i32) + 1;
            self.iSelectedEnemy = 0;
            let mut iWeightCount: i32 = g.game_values.gamemodesettings.stomp.enemyweight[self.iSelectedEnemy as usize] as i32;

            while iWeightCount < iRandEnemy {
                self.iSelectedEnemy += 1;
                iWeightCount += g.game_values.gamemodesettings.stomp.enemyweight[self.iSelectedEnemy as usize] as i32;
            }
        }

        if 0 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Goomba::new(g, rm.spr_goomba, RANDOM_BOOL(gp.get()), false));
            gp.objectcontainer[0].add(g, obj);
        } else if 1 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Koopa::new(g, rm.spr_koopa, RANDOM_BOOL(gp.get()), false, false, true));
            gp.objectcontainer[0].add(g, obj);
        } else if 2 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_CheepCheep::new(g, rm.spr_cheepcheep));
            gp.objectcontainer[2].add(g, obj);
        } else if 3 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Koopa::new(g, rm.spr_redkoopa, RANDOM_BOOL(gp.get()), true, false, false));
            gp.objectcontainer[0].add(g, obj);
        } else if 4 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Spiny::new(g, rm.spr_spiny, RANDOM_BOOL(gp.get())));
            gp.objectcontainer[0].add(g, obj);
        } else if 5 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_BuzzyBeetle::new(g, rm.spr_buzzybeetle, RANDOM_BOOL(gp.get())));
            gp.objectcontainer[0].add(g, obj);
        } else if 6 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Goomba::new(g, rm.spr_paragoomba, RANDOM_BOOL(gp.get()), true));
            gp.objectcontainer[0].add(g, obj);
        } else if 7 == self.iSelectedEnemy {
            let obj = Ptr::new_box(MO_Koopa::new(g, rm.spr_parakoopa, RANDOM_BOOL(gp.get()), false, true, true));
            gp.objectcontainer[0].add(g, obj);
        } else {
            let obj = Ptr::new_box(MO_Koopa::new(g, rm.spr_redparakoopa, RANDOM_BOOL(gp.get()), true, true, true));
            gp.objectcontainer[0].add(g, obj);
        }
    }

    fn reset_spawn_timer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let rate = g.game_values.gamemodesettings.stomp.rate;
        self.spawntimer = ((RANDOM_INT(gp.get(), rate as i32) as i16) as i32 + rate as i32) as i16;
    }
}

pub fn net_spawn_enemy(g: &mut Game) {
    if let Some(this) = g.game_values.gamemode.get().as_any().downcast_mut::<CGM_Stomp>() {
        this.spawn_enemy(g);
    }
}

impl CGameModeTrait for CGM_Stomp {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgamemode_init(g, self);
        self.reset_spawn_timer(g);

        self.iEnemyWeightCount = 0;
        for iEnemy in 0..NUMSTOMPENEMIES as usize {
            self.iEnemyWeightCount += g.game_values.gamemodesettings.stomp.enemyweight[iEnemy];
        }

        //if (iEnemyWeightCount == 0)
        //  iEnemyWeightCount = 1;
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
        } else {
            for i in 0..g.players.len() {
                let player = g.players[i];
                self.check_winner(g, player);
            }
        }

        if !self.gameover {
            //Randomly spawn enemies
            self.spawntimer -= 1;
            if self.spawntimer <= 0 && !net_random::event(g, Ev::StompEnemy, &[]) {
                self.spawn_enemy(g);
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _player: PlayerId, _other: PlayerId, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, _player: PlayerId, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, iType);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if g.player(player).score(g).score >= self.goal {
            g.player(player).score(g).set_score(g, self.goal);
            self.winningteam = g.player(player).get_team_id();
            self.gameover = true;

            setup_score_board(g, false);
            show_score_board(g);
            remove_players_but_team(g, self.winningteam);
            return PlayerKillType::Removed;
        } else if g.player(player).score(g).score as i32 >= self.goal as i32 - 2 && !self.playedwarningsound {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }
}
