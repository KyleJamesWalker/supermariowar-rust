//! Port of src/smw/gamemodes/Survival.cpp

use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::global_constants::NUMSURVIVALENEMIES;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::{cgm_classic_init, cgm_classic_playerextraguy, cgm_classic_playerkilledplayer, cgm_classic_playerkilledself, CGM_Classic};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::overmap::wo_bowser_fire::OMO_BowserFire;
use crate::smw::objects::overmap::wo_thwomp::OMO_Thwomp;
use crate::smw::player::CPlayer;

pub struct CGM_Survival {
    pub cgm_classic: CGM_Classic,

    pub timer: i16,
    pub ratetimer: i16,
    pub rate: i16,
    pub iSelectedEnemy: i16,
    pub iEnemyWeightCount: i16,
}
impl_base!(CGM_Survival => cgm_classic: CGM_Classic);

//Survival Mode! - just like mario war classic, but you have
// to dodge thwomps from the sky.  Idea from ziotok.
impl CGM_Survival {
    pub fn new() -> Self {
        let mut this = CGM_Survival { cgm_classic: CGM_Classic::new(), timer: 0, ratetimer: 0, rate: 0, iSelectedEnemy: 0, iEnemyWeightCount: 0 };
        this.goal = 20;
        this.gamemode = game_mode_survival;
        this.szModeName = "Survival".to_string();
        this
    }
}

impl CGameModeTrait for CGM_Survival {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgm_classic_init(&mut self.cgm_classic);

        unsafe {
            self.rate = (3 * game_values.gamemodesettings.survival.density as i32) as i16;
            self.timer = (RANDOM_INT(21) - 10 + self.rate as i32) as i16;
            self.ratetimer = 0;

            self.iEnemyWeightCount = 0;
            for iEnemy in 0..NUMSURVIVALENEMIES as usize {
                self.iEnemyWeightCount += game_values.gamemodesettings.survival.enemyweight[iEnemy];
            }

            if self.iEnemyWeightCount == 0 {
                self.iEnemyWeightCount = 1;
            }
        }
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
            return;
        }

        unsafe {
            self.timer -= 1;
            if self.timer <= 0 {
                self.ratetimer += 1;
                if self.ratetimer == 10 {
                    self.ratetimer = 0;

                    self.rate -= 1;
                    if self.rate < game_values.gamemodesettings.survival.density {
                        self.rate = game_values.gamemodesettings.survival.density;
                    }
                }

                //Randomly choose an enemy from the weighted list
                let iRandEnemy: i32 = RANDOM_INT(self.iEnemyWeightCount as i32) + 1;
                self.iSelectedEnemy = 0;
                let mut iWeightCount: i32 = game_values.gamemodesettings.survival.enemyweight[self.iSelectedEnemy as usize] as i32;

                while iWeightCount < iRandEnemy {
                    self.iSelectedEnemy += 1;
                    iWeightCount += game_values.gamemodesettings.survival.enemyweight[self.iSelectedEnemy as usize] as i32;
                }

                if 0 == self.iSelectedEnemy {
                    let x = RANDOM_INT((App::screenWidth as f32 * 0.92f32) as i32) as i16;
                    let nspeed = game_values.gamemodesettings.survival.speed as f32 / 2.0f32 + (RANDOM_INT(20) as f32) / 10.0f32;
                    objectcontainer[2].add(Ptr::new_box(OMO_Thwomp::new(Ptr::from_mut(&mut rm.spr_thwomp), x, nspeed)));
                    self.timer = (RANDOM_INT(21) - 10 + self.rate as i32) as i16;
                } else if 1 == self.iSelectedEnemy {
                    let x = RANDOM_INT((App::screenWidth as f32 * 0.95f32) as i32) as i16;
                    let nspeed = -((RANDOM_INT(9) as f32) / 2.0f32) - 8.0f32;
                    objectcontainer[2].add(Ptr::new_box(MO_Podobo::new(Ptr::from_mut(&mut rm.spr_podobo), Vec2s::new(x, App::screenHeight as i16), nspeed, -1, -1, -1, false)));
                    self.timer = (RANDOM_INT(21) - 10 + self.rate as i32 - 20) as i16;
                } else {
                    let dSpeed: f32 = ((RANDOM_INT(21) + 20) as f32) / 10.0f32;
                    let dVel: f32 = if RANDOM_BOOL() { dSpeed } else { -dSpeed };

                    let mut x: i16 = -54;
                    if dVel < 0.0 {
                        x = 694;
                    }

                    let y = RANDOM_INT((App::screenHeight as f32 * 0.93f32) as i32) as i16;
                    objectcontainer[2].add(Ptr::new_box(OMO_BowserFire::new(Ptr::from_mut(&mut rm.spr_bowserfire), Vec2s::new(x, y), Vec2f::new(dVel, 0.0f32), -1, -1, -1)));
                    self.timer = (RANDOM_INT(21) - 10 + self.rate as i32) as i16;
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledplayer(&mut self.cgm_classic, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledself(&mut self.cgm_classic, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_classic_playerextraguy(&mut self.cgm_classic, player, iType)
    }
}
