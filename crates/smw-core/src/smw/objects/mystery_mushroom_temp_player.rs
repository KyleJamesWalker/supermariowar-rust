//! Port of src/smw/objects/MysteryMushroomTempPlayer.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
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

    pub fn set_player(&mut self, g: &mut Game, mut player: PlayerId, iPowerup: i16) {
        self.fx = g.player(player).fx;
        self.fy = g.player(player).fy;

        self.fOldX = g.player(player).fOldX;
        self.fOldY = g.player(player).fOldY;

        self.velx = g.player(player).velx;
        self.vely = g.player(player).vely;

        // bobomb = player->bobomb;
        // powerup = player->powerup;

        self.burnupstarttimer = g.player(player).burnup.starttimer;
        self.burnuptimer = g.player(player).burnup.timer;

        self.inair = g.player(player).inair;
        self.onice = g.player(player).onice;
        // invincible = player->invincible;
        // invincibletimer = player->invincibletimer;

        self.platform = g.player(player).platform;
        // iCapeFrameX = player->iCapeFrameX;
        // iCapeFrameY = player->iCapeFrameY;
        // iCapeTimer = player->iCapeTimer;
        // iCapeYOffset = player->iCapeYOffset;

        self.gamepowerup = iPowerup;

        let offset = scorepowerupoffsets[(g.game_values.teamcounts[g.player(player).teamID as usize] - 1) as usize][g.player(player).subTeamID as usize];
        self.iOldPowerupX = (g.player(player).score().x as i32 + offset as i32) as i16;
        self.iOldPowerupY = (g.player(player).score().y as i32 + 25) as i16;
    }

    pub fn get_player(&mut self, g: &mut Game, mut player: PlayerId, iPowerup: &mut i16) {
        g.player(player).fNewSwapX = self.fx;
        g.player(player).fNewSwapY = self.fy;

        g.player(player).iOldPowerupX = self.iOldPowerupX;
        g.player(player).iOldPowerupY = self.iOldPowerupY;

        g.player(player).fOldX = self.fOldX;
        g.player(player).fOldY = self.fOldY;

        g.player(player).velx = self.velx;
        g.player(player).vely = self.vely;

        // player->bobomb = bobomb;
        // player->powerup = powerup;

        g.player(player).burnup.starttimer = self.burnupstarttimer;
        g.player(player).burnup.timer = self.burnuptimer;

        g.player(player).inair = self.inair;
        g.player(player).onice = self.onice;
        // player->invincible = invincible;
        // player->invincibletimer = invincibletimer;

        g.player(player).platform = self.platform;
        // player->iCapeFrameX = iCapeFrameX;
        // player->iCapeFrameY	= iCapeFrameY;
        // player->iCapeTimer = iCapeTimer;
        // player->iCapeYOffset = iCapeYOffset;

        *iPowerup = self.gamepowerup;

        if !g.carried_item(player).is_null() {
            g.carried_item(player).move_to_owner(g);
        }
    }
}
