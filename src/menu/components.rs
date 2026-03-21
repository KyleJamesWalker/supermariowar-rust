use std::path::PathBuf;

use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::game_modes::traits::GameModeType;

// ---------------------------------------------------------------------------
// Menu actions & navigation
// ---------------------------------------------------------------------------

/// Actions that can be triggered by selecting a menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    StartGame,
    OpenGameSettings,
    OpenPlayerSelect,
    OpenOptions,
    BackToMain,
    Resume,
    Restart,
    QuitToMenu,
    Quit,
    SelectMode,
    SelectMap,
    TeamSelect,
    NextValue,
    PrevValue,
    Confirm,
}

/// Identifies which submenu is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SubMenu {
    #[default]
    Main,
    GameSettings,
    PlayerSelect,
    Options,
}

/// Resource tracking the active submenu.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct CurrentSubMenu {
    pub submenu: SubMenu,
}

// ---------------------------------------------------------------------------
// Menu UI components
// ---------------------------------------------------------------------------

/// Marker for the main menu root entity.
#[derive(Component, Debug)]
pub struct MenuRoot;

/// Marker for a menu item that can be selected.
#[derive(Component, Debug)]
pub struct MenuItem {
    pub index: usize,
    pub label: String,
    pub enabled: bool,
    pub action: MenuAction,
}

/// Resource tracking which menu item is currently highlighted.
#[derive(Resource, Debug, Default)]
pub struct MenuSelection {
    pub selected_index: usize,
    pub item_count: usize,
}

/// Marker for the title text entity.
#[derive(Component, Debug)]
pub struct TitleText;

/// Marker for the version text entity.
#[derive(Component, Debug)]
pub struct VersionText;

// ---------------------------------------------------------------------------
// Game setup configuration
// ---------------------------------------------------------------------------

/// Control type for a player slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlType {
    Human,
    Cpu,
    None,
}

impl ControlType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ControlType::Human => "Human",
            ControlType::Cpu => "CPU",
            ControlType::None => "None",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            ControlType::Human => ControlType::Cpu,
            ControlType::Cpu => ControlType::None,
            ControlType::None => ControlType::Human,
        }
    }
}

/// CPU difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuDifficulty {
    Easy,
    Medium,
    Hard,
}

impl CpuDifficulty {
    pub fn display_name(&self) -> &'static str {
        match self {
            CpuDifficulty::Easy => "Easy",
            CpuDifficulty::Medium => "Medium",
            CpuDifficulty::Hard => "Hard",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            CpuDifficulty::Easy => CpuDifficulty::Medium,
            CpuDifficulty::Medium => CpuDifficulty::Hard,
            CpuDifficulty::Hard => CpuDifficulty::Easy,
        }
    }
}

/// Per-player slot configuration.
#[derive(Debug, Clone)]
pub struct PlayerSlotConfig {
    pub control_type: ControlType,
    pub team: u8,
    pub cpu_difficulty: CpuDifficulty,
    pub skin_index: usize,
}

impl Default for PlayerSlotConfig {
    fn default() -> Self {
        Self {
            control_type: ControlType::None,
            team: 0,
            cpu_difficulty: CpuDifficulty::Medium,
            skin_index: 0,
        }
    }
}

/// Resource holding the complete game setup (mode, limits, players).
#[derive(Resource, Debug, Clone)]
pub struct GameSetupConfig {
    pub selected_mode: GameModeType,
    pub kill_limit: u32,
    pub time_limit_secs: u32,
    pub point_limit: u32,
    pub lives: u32,
    pub num_players: usize,
    pub player_slots: [PlayerSlotConfig; MAX_PLAYERS],
    pub total_rounds: u32,
    /// Path to the map file to load for gameplay.
    pub map_path: PathBuf,
}

impl Default for GameSetupConfig {
    fn default() -> Self {
        Self {
            selected_mode: GameModeType::Classic,
            kill_limit: 10,
            time_limit_secs: 60,
            point_limit: 5,
            lives: 5,
            num_players: 2,
            player_slots: [
                PlayerSlotConfig {
                    control_type: ControlType::Human,
                    team: 0,
                    cpu_difficulty: CpuDifficulty::Medium,
                    skin_index: 0,
                },
                PlayerSlotConfig {
                    control_type: ControlType::Human,
                    team: 1,
                    cpu_difficulty: CpuDifficulty::Medium,
                    skin_index: 1,
                },
                PlayerSlotConfig {
                    control_type: ControlType::None,
                    team: 2,
                    cpu_difficulty: CpuDifficulty::Medium,
                    skin_index: 2,
                },
                PlayerSlotConfig {
                    control_type: ControlType::None,
                    team: 3,
                    cpu_difficulty: CpuDifficulty::Medium,
                    skin_index: 3,
                },
            ],
            total_rounds: 3,
            map_path: PathBuf::from("data/maps/0smw.map"),
        }
    }
}

// ---------------------------------------------------------------------------
// Game settings menu components
// ---------------------------------------------------------------------------

/// Marker for the game settings root node.
#[derive(Component, Debug)]
pub struct GameSettingsRoot;

/// Identifies a specific setting field for the game settings menu.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSettingField {
    Mode,
    KillLimit,
    TimeLimit,
    PointLimit,
    Lives,
    Rounds,
    Start,
    Back,
}

/// Resource tracking which field is selected in game settings.
#[derive(Resource, Debug, Default)]
pub struct GameSettingsSelection {
    pub selected_index: usize,
    pub field_count: usize,
}

// ---------------------------------------------------------------------------
// Player select components
// ---------------------------------------------------------------------------

/// Marker for the player select root node.
#[derive(Component, Debug)]
pub struct PlayerSelectRoot;

/// Identifies which player slot a UI element belongs to.
#[derive(Component, Debug)]
pub struct PlayerSlotMarker {
    pub player_index: usize,
}

/// Identifies what part of a player slot is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSelectField {
    ControlType,
    Team,
    CpuDifficulty,
}

/// Resource tracking selection state for the player select screen.
#[derive(Resource, Debug)]
pub struct PlayerSelectState {
    pub active_slot: usize,
    pub active_field: PlayerSelectField,
}

impl Default for PlayerSelectState {
    fn default() -> Self {
        Self {
            active_slot: 0,
            active_field: PlayerSelectField::ControlType,
        }
    }
}

// ---------------------------------------------------------------------------
// HUD components
// ---------------------------------------------------------------------------

/// Marker for the HUD root entity.
#[derive(Component, Debug)]
pub struct HudRoot;

/// Marker for a player score display in the HUD.
#[derive(Component, Debug)]
pub struct ScoreDisplay {
    pub player_index: usize,
}

/// Marker for a player lives display in the HUD.
#[derive(Component, Debug)]
pub struct LivesDisplay {
    pub player_index: usize,
}

/// Marker for the game timer display.
#[derive(Component, Debug)]
pub struct TimerDisplay;

/// Marker for a player powerup indicator.
#[derive(Component, Debug)]
pub struct PowerupDisplay {
    pub player_index: usize,
}

/// Marker for the round indicator text.
#[derive(Component, Debug)]
pub struct RoundDisplay;

/// Marker for the countdown text (3, 2, 1, GO!).
#[derive(Component, Debug)]
pub struct CountdownDisplay;

// ---------------------------------------------------------------------------
// Pause menu components
// ---------------------------------------------------------------------------

/// Marker for the pause menu root entity.
#[derive(Component, Debug)]
pub struct PauseMenuRoot;

/// Marker for a pause menu item.
#[derive(Component, Debug)]
pub struct PauseMenuItem {
    pub index: usize,
    pub action: MenuAction,
}

/// Resource tracking selection in the pause menu.
#[derive(Resource, Debug, Default)]
pub struct PauseMenuSelection {
    pub selected_index: usize,
    pub item_count: usize,
}

// ---------------------------------------------------------------------------
// Scoreboard components
// ---------------------------------------------------------------------------

/// Marker for the scoreboard root entity.
#[derive(Component, Debug)]
pub struct ScoreboardRoot;

/// Marker for a player entry row on the scoreboard.
#[derive(Component, Debug)]
pub struct ScoreboardEntry {
    pub player_index: usize,
}

/// Marker for the scoreboard prompt text ("Next Round" / "Back to Menu").
#[derive(Component, Debug)]
pub struct ScoreboardPrompt;

/// Timer resource for auto-advancing the scoreboard.
#[derive(Resource, Debug)]
pub struct ScoreboardTimer {
    pub timer: Timer,
}

impl Default for ScoreboardTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Once),
        }
    }
}
