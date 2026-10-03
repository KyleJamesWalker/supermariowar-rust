//! Port of src/smw/player.cpp
// PLACEHOLDER(foundation): only the fields and methods the common/ ports touch. The player porter
// replaces this file with the full CPlayer, keeping these C++ names and types.

use crate::common::movingplatform::MovingPlatform;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::score::CScore;
use crate::globals::{Aliased, Ptr};
use crate::smw::player_components::player_kuribo_shoe::PlayerKuriboShoe;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerState {
    #[default]
    Waiting,
    Spawning,
    Dead,
    Ready,
    EnteringWarpUp,
    EnteringWarpRight,
    EnteringWarpDown,
    EnteringWarpLeft,
    LeavingWarpDown,
    LeavingWarpLeft,
    LeavingWarpUp,
    LeavingWarpRight,
}

#[derive(Default)]
pub struct CPlayer {
    pub shyguy: bool,

    pub score: Ptr<CScore>,
    pub killsinrowinair: i16,

    pub ix: i16,
    pub iy: i16,
    pub fx: f32,
    pub fy: f32,
    pub velx: f32,
    pub vely: f32,
    pub fOldX: f32,
    pub fOldY: f32,
    pub fPrecalculatedY: f32,
    pub oldvelx: f32,

    pub inair: bool,
    pub onice: bool,

    pub extrajumps: i16,

    pub kuriboshoe: PlayerKuriboShoe,

    pub state: PlayerState,

    pub platform: Ptr<MovingPlatform>,
    pub iHorizontalPlatformCollision: i16,
    pub iVerticalPlatformCollision: i16,
    pub iPlatformCollisionPlayerId: i16,

    pub fallthrough: bool,

    pub iSuicideCreditPlayerID: i16,
    pub iSuicideCreditTimer: i16,

    pub _alias: Aliased,
}

impl CPlayer {
    pub fn isready(&self) -> bool {
        self.state == PlayerState::Ready
    }

    pub fn is_invincible(&self) -> bool {
        todo!("port CPlayer::isInvincible")
    }

    pub fn is_shielded(&self) -> bool {
        todo!("port CPlayer::isShielded")
    }

    pub fn set_xf(&mut self, xf: f32) {
        self.fx = xf;
        self.ix = self.fx as i16;
    }
    pub fn set_xi(&mut self, xi: i16) {
        self.ix = xi;
        self.fx = self.ix as f32;
    }
    pub fn set_yf(&mut self, yf: f32) {
        self.fy = yf;
        self.iy = self.fy as i16;
    }
    pub fn set_yi(&mut self, yi: i16) {
        self.iy = yi;
        self.fy = self.iy as f32;
    }

    pub fn flipsidesifneeded(&mut self) {
        todo!("port CPlayer::flipsidesifneeded")
    }

    /// `KillPlayerMapHazard(bool fForce, KillStyle ks, bool fKillCarriedItem, short iPlayerId = -1)`
    pub fn kill_player_map_hazard(&mut self, _fForce: bool, _ks: KillStyle, _fKillCarriedItem: bool, _iPlayerId: i16) -> PlayerKillType {
        todo!("port CPlayer::KillPlayerMapHazard")
    }
}
