//! Snapshot of a player's movement state, used to swap players with the mystery mushroom.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::movingplatform::MovingPlatform;
use crate::globals::*;
use crate::smw::gs_gameplay::scorepowerupoffsets;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::player::CPlayer;

/// Position, velocity and powerup of one player, copied out with `set_player` and applied with `get_player`.
pub struct MysteryMushroomTempPlayer {
    /// Slot already taken in the current swap.
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

    pub platform: PlatformId,

    /// Stored powerup carried with the swap.
    pub gamepowerup: i16,

    /// Screen position (pixels) of the stored-powerup icon, for the swap animation.
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
            platform: PlatformId::NONE,
            gamepowerup: 0,
            iOldPowerupX: 0,
            iOldPowerupY: 0,
        }
    }

    /// Copies `player`'s state and `iPowerup` into this snapshot.
    pub fn set_player(&mut self, g: &mut Game, mut player: PlayerId, iPowerup: i16) {
        self.fx = g.player(player).fx;
        self.fy = g.player(player).fy;

        self.fOldX = g.player(player).fOldX;
        self.fOldY = g.player(player).fOldY;

        self.velx = g.player(player).velx;
        self.vely = g.player(player).vely;

        self.burnupstarttimer = g.player(player).burnup.starttimer;
        self.burnuptimer = g.player(player).burnup.timer;

        self.inair = g.player(player).inair;
        self.onice = g.player(player).onice;

        self.platform = g.player(player).platform;

        self.gamepowerup = iPowerup;

        let offset = scorepowerupoffsets[(g.game_values.teamcounts[g.player(player).teamID as usize] - 1) as usize][g.player(player).subTeamID as usize];
        self.iOldPowerupX = (g.player(player).score(g).x as i32 + offset as i32) as i16;
        self.iOldPowerupY = (g.player(player).score(g).y as i32 + 25) as i16;
    }

    /// Applies this snapshot to `player`: sets its swap target and returns the stored powerup in `iPowerup`.
    pub fn get_player(&mut self, g: &mut Game, mut player: PlayerId, iPowerup: &mut i16) {
        g.player(player).fNewSwapX = self.fx;
        g.player(player).fNewSwapY = self.fy;

        g.player(player).iOldPowerupX = self.iOldPowerupX;
        g.player(player).iOldPowerupY = self.iOldPowerupY;

        g.player(player).fOldX = self.fOldX;
        g.player(player).fOldY = self.fOldY;

        g.player(player).velx = self.velx;
        g.player(player).vely = self.vely;

        g.player(player).burnup.starttimer = self.burnupstarttimer;
        g.player(player).burnup.timer = self.burnuptimer;

        g.player(player).inair = self.inair;
        g.player(player).onice = self.onice;

        g.player(player).platform = self.platform;

        *iPowerup = self.gamepowerup;

        if !g.carried_item(player).is_null() {
            g.carried_item(player).move_to_owner(g);
        }
    }
}
