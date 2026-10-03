//! Port of src/smw/objects/MysteryMushroomTempPlayer.cpp

use crate::common::movingplatform::MovingPlatform;
use crate::globals::*;
use crate::smw::gs_gameplay::scorepowerupoffsets;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::player::CPlayer;

pub struct MysteryMushroomTempPlayer {
    pub fUsed: bool,

    pub fx: f32,
    pub fy: f32,
    pub fOldX: f32,
    pub fOldY: f32,
    pub velx: f32,
    pub vely: f32,

    pub burnupstarttimer: i16,
    pub burnuptimer: i16,

    pub inair: bool,
    pub onice: bool,

    pub platform: Ptr<MovingPlatform>,

    pub gamepowerup: i16,

    pub iOldPowerupX: i16,
    pub iOldPowerupY: i16,
    pub _alias: Aliased,
}

impl Default for MysteryMushroomTempPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MysteryMushroomTempPlayer {
    pub fn new() -> Self {
        MysteryMushroomTempPlayer { _alias: Aliased::new(),
            fUsed: false,
            fx: 0.0,
            fy: 0.0,
            fOldX: 0.0,
            fOldY: 0.0,
            velx: 0.0,
            vely: 0.0,
            burnupstarttimer: 0,
            burnuptimer: 0,
            inair: false,
            onice: false,
            platform: Ptr::null(),
            gamepowerup: 0,
            iOldPowerupX: 0,
            iOldPowerupY: 0,
        }
    }

    pub fn set_player(&mut self, mut player: Ptr<CPlayer>, iPowerup: i16) {
        self.fx = player.fx;
        self.fy = player.fy;

        self.fOldX = player.fOldX;
        self.fOldY = player.fOldY;

        self.velx = player.velx;
        self.vely = player.vely;

        // bobomb = player->bobomb;
        // powerup = player->powerup;

        self.burnupstarttimer = player.burnup.starttimer;
        self.burnuptimer = player.burnup.timer;

        self.inair = player.inair;
        self.onice = player.onice;
        // invincible = player->invincible;
        // invincibletimer = player->invincibletimer;

        self.platform = player.platform;
        // iCapeFrameX = player->iCapeFrameX;
        // iCapeFrameY = player->iCapeFrameY;
        // iCapeTimer = player->iCapeTimer;
        // iCapeYOffset = player->iCapeYOffset;

        self.gamepowerup = iPowerup;

        unsafe {
            let offset = scorepowerupoffsets[(game_values.teamcounts[player.teamID as usize] - 1) as usize][player.subTeamID as usize];
            self.iOldPowerupX = (player.score().x as i32 + offset as i32) as i16;
            self.iOldPowerupY = (player.score().y as i32 + 25) as i16;
        }
    }

    pub fn get_player(&mut self, mut player: Ptr<CPlayer>, iPowerup: &mut i16) {
        player.fNewSwapX = self.fx;
        player.fNewSwapY = self.fy;

        player.iOldPowerupX = self.iOldPowerupX;
        player.iOldPowerupY = self.iOldPowerupY;

        player.fOldX = self.fOldX;
        player.fOldY = self.fOldY;

        player.velx = self.velx;
        player.vely = self.vely;

        // player->bobomb = bobomb;
        // player->powerup = powerup;

        player.burnup.starttimer = self.burnupstarttimer;
        player.burnup.timer = self.burnuptimer;

        player.inair = self.inair;
        player.onice = self.onice;
        // player->invincible = invincible;
        // player->invincibletimer = invincibletimer;

        player.platform = self.platform;
        // player->iCapeFrameX = iCapeFrameX;
        // player->iCapeFrameY	= iCapeFrameY;
        // player->iCapeTimer = iCapeTimer;
        // player->iCapeYOffset = iCapeYOffset;

        *iPowerup = self.gamepowerup;

        if !player.carriedItem.is_null() {
            player.carriedItem.move_to_owner();
        }
    }
}
