//! Survival mode: classic lives while dodging thwomps, podobos and Bowser fire.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::global_constants::{ticks, vel, NUMSURVIVALENEMIES};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::{cgm_classic_init, cgm_classic_playerextraguy, cgm_classic_playerkilledplayer, cgm_classic_playerkilledself, CGM_Classic};
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::overmap::wo_bowser_fire::OMO_BowserFire;
use crate::smw::objects::overmap::wo_thwomp::OMO_Thwomp;
use crate::smw::player::CPlayer;

#[derive(Clone)]

/// Survival mode: classic scoring with weighted-random hazards falling or flying in.
pub struct CGM_Survival {
    pub cgm_classic: CGM_Classic,

    /// Ticks until the next hazard spawn.
    pub timer: i16,
    /// Spawns since `rate` last dropped; it drops by one every `ticks(10)` spawns.
    pub ratetimer: i16,
    /// Base spawn interval, starting at `3 * density` and falling to `density`.
    pub rate: i16,
    /// Most recent hazard: 0 thwomp, 1 podobo, 2 Bowser fire.
    pub iSelectedEnemy: i16,
    /// Sum of the configured hazard weights, at least 1.
    pub iEnemyWeightCount: i16,
}
impl_base!(CGM_Survival => cgm_classic: CGM_Classic);

impl CGM_Survival {
    pub fn new() -> Self {
        let mut this = CGM_Survival { cgm_classic: CGM_Classic::new(), timer: 0, ratetimer: 0, rate: 0, iSelectedEnemy: 0, iEnemyWeightCount: 0 };
        this.goal = 20;
        this.gamemode = game_mode_survival;
        this.szModeName = "Survival".to_string();
        this
    }
}

impl CGM_Survival {
    fn spawn_enemy(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        self.ratetimer += 1;
        if self.ratetimer as i32 == ticks(10) {
            self.ratetimer = 0;

            self.rate -= 1;
            if self.rate < g.game_values.gamemodesettings.survival.density {
                self.rate = g.game_values.gamemodesettings.survival.density;
            }
        }

        //Randomly choose an enemy from the weighted list
        let iRandEnemy: i32 = RANDOM_INT(gp.get(), self.iEnemyWeightCount as i32) + 1;
        self.iSelectedEnemy = 0;
        let mut iWeightCount: i32 = g.game_values.gamemodesettings.survival.enemyweight[self.iSelectedEnemy as usize] as i32;

        while iWeightCount < iRandEnemy {
            self.iSelectedEnemy += 1;
            iWeightCount += g.game_values.gamemodesettings.survival.enemyweight[self.iSelectedEnemy as usize] as i32;
        }

        if 0 == self.iSelectedEnemy {
            let x = RANDOM_INT(gp.get(), (App::screenWidth as f32 * 0.92f32) as i32) as i16;
            let nspeed = vel(g.game_values.gamemodesettings.survival.speed as f32 / 2.0f32 + (RANDOM_INT(gp.get(), 20) as f32) / 10.0f32);
            let obj = Ptr::new_box(OMO_Thwomp::new(g, rm.spr_thwomp, x, nspeed));
            gp.objectcontainer[2].add(g, obj);
            self.timer = ticks(RANDOM_INT(gp.get(), 21) - 10 + self.rate as i32) as i16;
        } else if 1 == self.iSelectedEnemy {
            let x = RANDOM_INT(gp.get(), (App::screenWidth as f32 * 0.95f32) as i32) as i16;
            let nspeed = vel(-((RANDOM_INT(gp.get(), 9) as f32) / 2.0f32) - 8.0f32);
            let obj = Ptr::new_box(MO_Podobo::new(g, rm.spr_podobo, Vec2s::new(x, App::screenHeight as i16), nspeed, -1, -1, -1, false));
            gp.objectcontainer[2].add(g, obj);
            self.timer = ticks(RANDOM_INT(gp.get(), 21) - 10 + self.rate as i32 - 20) as i16;
        } else {
            let dSpeed: f32 = vel(((RANDOM_INT(gp.get(), 21) + 20) as f32) / 10.0f32);
            let dVel: f32 = if RANDOM_BOOL(gp.get()) { dSpeed } else { -dSpeed };

            let mut x: i16 = -54;
            if dVel < 0.0 {
                x = 694;
            }

            let y = RANDOM_INT(gp.get(), (App::screenHeight as f32 * 0.93f32) as i32) as i16;
            let obj = Ptr::new_box(OMO_BowserFire::new(g, rm.spr_bowserfire, Vec2s::new(x, y), Vec2f::new(dVel, 0.0f32), -1, -1, -1));
            gp.objectcontainer[2].add(g, obj);
            self.timer = ticks(RANDOM_INT(gp.get(), 21) - 10 + self.rate as i32) as i16;
        }
    }
}

impl CGameModeTrait for CGM_Survival {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.i16s(&[self.timer, self.ratetimer, self.rate, self.iSelectedEnemy, self.iEnemyWeightCount]);
    }
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        cgm_classic_init(g, &mut self.cgm_classic);

        self.rate = (3 * g.game_values.gamemodesettings.survival.density as i32) as i16;
        self.timer = ticks(RANDOM_INT(gp.get(), 21) - 10 + self.rate as i32) as i16;
        self.ratetimer = 0;

        self.iEnemyWeightCount = 0;
        for iEnemy in 0..NUMSURVIVALENEMIES as usize {
            self.iEnemyWeightCount += g.game_values.gamemodesettings.survival.enemyweight[iEnemy];
        }

        if self.iEnemyWeightCount == 0 {
            self.iEnemyWeightCount = 1;
        }
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        self.timer -= 1;
        if self.timer <= 0 {
            self.spawn_enemy(g);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledplayer(g, &mut self.cgm_classic, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_classic_playerkilledself(g, &mut self.cgm_classic, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        cgm_classic_playerextraguy(g, &mut self.cgm_classic, player, iType)
    }
}
