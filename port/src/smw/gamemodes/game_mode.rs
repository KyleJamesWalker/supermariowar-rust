//! Port of src/smw/gamemodes/GameMode.cpp
// PLACEHOLDER(foundation): signatures for the CGameMode base-class bodies; the gamemodes porter fills them in.

use crate::common::game_mode::CGameModeTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::Ptr;
use crate::smw::player::CPlayer;

pub fn cgamemode_init<T: CGameModeTrait + ?Sized>(_this: &mut T) {
    todo!("port CGameMode::init")
}

pub fn cgamemode_think<T: CGameModeTrait + ?Sized>(_this: &mut T) {
    todo!("port CGameMode::think")
}

pub fn cgamemode_playerkilledplayer<T: CGameModeTrait + ?Sized>(_this: &mut T, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    todo!("port CGameMode::playerkilledplayer")
}

pub fn cgamemode_playerkilledself<T: CGameModeTrait + ?Sized>(_this: &mut T, _player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    todo!("port CGameMode::playerkilledself")
}

pub fn cgamemode_playerextraguy<T: CGameModeTrait + ?Sized>(_this: &mut T, _player: Ptr<CPlayer>, _iType: i16) {
    todo!("port CGameMode::playerextraguy")
}
