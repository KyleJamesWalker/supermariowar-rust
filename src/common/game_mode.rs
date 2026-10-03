//! Port of src/common/GameMode.h

use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::{Aliased, Ptr};
use crate::smw::gamemodes::game_mode as gm;
use crate::smw::player::CPlayer;
use std::any::Any;
use std::ops::{Deref, DerefMut};

pub const GAMEMODE_NUM_OPTIONS: usize = 21;

pub type GameModeType = i32;
pub const game_mode_classic: GameModeType = 0;
pub const game_mode_frag: GameModeType = 1;
pub const game_mode_timelimit: GameModeType = 2;
pub const game_mode_jail: GameModeType = 3;
pub const game_mode_coins: GameModeType = 4;
pub const game_mode_stomp: GameModeType = 5;
pub const game_mode_eggs: GameModeType = 6;
pub const game_mode_ctf: GameModeType = 7;
pub const game_mode_chicken: GameModeType = 8;
pub const game_mode_tag: GameModeType = 9;
pub const game_mode_star: GameModeType = 10;
pub const game_mode_domination: GameModeType = 11;
pub const game_mode_koth: GameModeType = 12;
pub const game_mode_race: GameModeType = 13;
pub const game_mode_owned: GameModeType = 14;
pub const game_mode_frenzy: GameModeType = 15;
pub const game_mode_survival: GameModeType = 16;
pub const game_mode_greed: GameModeType = 17;
pub const game_mode_health: GameModeType = 18;
pub const game_mode_collection: GameModeType = 19;
pub const game_mode_chase: GameModeType = 20;
pub const game_mode_shyguytag: GameModeType = 21;
pub const GAMEMODE_LAST: GameModeType = 22;
pub const game_mode_bonus: GameModeType = 999;
pub const game_mode_pipe_minigame: GameModeType = 1000;
pub const game_mode_boss_minigame: GameModeType = 1001;
pub const game_mode_boxes_minigame: GameModeType = 1002;

#[derive(Clone, Debug, Default)]
pub struct SModeOption {
    pub szName: String,
    pub iValue: i16,
}

pub struct CGameMode {
    pub winningteam: i16,
    pub gameover: bool,
    pub gamemode: GameModeType,
    pub playedwarningsound: bool,
    pub goal: i16,

    pub szModeName: String,
    pub szGoalName: String,
    pub modeOptions: [SModeOption; GAMEMODE_NUM_OPTIONS],

    pub fReverseScoring: bool,

    pub _alias: Aliased,
}

impl CGameMode {
    pub fn getgamemode(&self) -> GameModeType {
        self.gamemode
    }
    pub fn get_mode_name(&self) -> &str {
        &self.szModeName
    }
    pub fn get_goal_name(&self) -> &str {
        &self.szGoalName
    }
    pub fn get_options(&mut self) -> &mut [SModeOption; GAMEMODE_NUM_OPTIONS] {
        &mut self.modeOptions
    }
    pub fn get_reverse_scoring(&self) -> bool {
        self.fReverseScoring
    }
}

/// Virtual interface of `CGameMode`. Base-class bodies live in `smw::gamemodes::game_mode`.
pub trait CGameModeTrait: Any {
    fn gm(&self) -> &CGameMode;
    fn gm_mut(&mut self) -> &mut CGameMode;
    fn as_any(&mut self) -> &mut dyn Any;

    fn init(&mut self) {
        gm::cgamemode_init(self)
    }
    fn think(&mut self) {
        gm::cgamemode_think(self)
    }
    fn draw_background(&mut self) {}
    fn draw_foreground(&mut self) {}
    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        gm::cgamemode_playerextraguy(self, player, iType)
    }
    fn check_winner(&mut self, _player: Ptr<CPlayer>) -> PlayerKillType {
        PlayerKillType::Normal
    }
    fn has_stored_powerups(&mut self) -> bool {
        true
    }
}

impl Deref for dyn CGameModeTrait {
    type Target = CGameMode;
    #[inline(always)]
    fn deref(&self) -> &CGameMode {
        self.gm()
    }
}

impl DerefMut for dyn CGameModeTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut CGameMode {
        self.gm_mut()
    }
}
