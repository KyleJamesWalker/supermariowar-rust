pub mod components;
pub mod game_settings;
pub mod hud;
pub mod main_menu;
pub mod pause_menu;
pub mod player_select;
pub mod scoreboard;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use components::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // --- Resources ---
        app.init_resource::<GameSetupConfig>()
            .init_resource::<CurrentSubMenu>()
            .init_resource::<ScoreboardTimer>()
            .init_resource::<MenuSelection>()
            .init_resource::<GameSettingsSelection>()
            .init_resource::<PlayerSelectState>()
            .init_resource::<PauseMenuSelection>();

        // =====================================================================
        // Main Menu
        // =====================================================================
        app.add_systems(
                OnEnter(GameState::Menu),
                (reset_submenu_to_main, main_menu::setup_main_menu).chain(),
            )
            .add_systems(OnExit(GameState::Menu), (cleanup_all_menu_ui, cleanup_menu_resources));

        // Main menu input: only runs when the Main submenu is active
        app.add_systems(
            Update,
            main_menu::main_menu_input
                .run_if(in_state(GameState::Menu))
                .run_if(resource_equals(CurrentSubMenu {
                    submenu: SubMenu::Main,
                })),
        );

        // Submenu transition system: runs for ALL submenus so it can handle
        // transitions back to Main (e.g., from GameSettings or PlayerSelect)
        app.add_systems(
            Update,
            submenu_transition_system
                .run_if(in_state(GameState::Menu)),
        );

        // =====================================================================
        // Game Settings submenu
        // =====================================================================
        app.add_systems(
            Update,
            (
                game_settings::game_settings_input,
                game_settings::update_game_settings_display,
            )
                .run_if(in_state(GameState::Menu))
                .run_if(resource_equals(CurrentSubMenu {
                    submenu: SubMenu::GameSettings,
                })),
        );

        // =====================================================================
        // Player Select submenu
        // =====================================================================
        app.add_systems(
            Update,
            (
                player_select::player_select_input,
                player_select::update_player_select_display,
            )
                .run_if(in_state(GameState::Menu))
                .run_if(resource_equals(CurrentSubMenu {
                    submenu: SubMenu::PlayerSelect,
                })),
        );

        // =====================================================================
        // HUD (in-game)
        // =====================================================================
        app.add_systems(OnEnter(GameState::Gameplay), hud::setup_hud)
            .add_systems(OnExit(GameState::Gameplay), hud::cleanup_hud)
            .add_systems(
                Update,
                (
                    hud::update_hud_scores,
                    hud::update_hud_lives,
                    hud::update_hud_timer,
                    hud::update_hud_powerups,
                    hud::update_hud_round,
                    hud::update_hud_countdown,
                )
                    .run_if(in_state(GameState::Gameplay)),
            );

        // =====================================================================
        // Pause Menu
        // =====================================================================
        // Listen for pause events during Playing state
        app.add_systems(
            Update,
            pause_menu::handle_pause_toggle
                .run_if(in_state(GameState::Gameplay))
                .run_if(
                    in_state(GameplayState::Playing)
                        .or(in_state(GameplayState::Paused)),
                ),
        );

        app.add_systems(OnEnter(GameplayState::Paused), pause_menu::setup_pause_menu)
            .add_systems(OnExit(GameplayState::Paused), pause_menu::cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    pause_menu::pause_menu_input,
                    pause_menu::update_pause_menu_display,
                )
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Paused)),
            );

        // =====================================================================
        // Scoreboard
        // =====================================================================
        app.add_systems(
            OnEnter(GameplayState::RoundOver),
            scoreboard::setup_round_scoreboard,
        )
        .add_systems(
            OnExit(GameplayState::RoundOver),
            scoreboard::cleanup_scoreboard,
        )
        .add_systems(
            Update,
            scoreboard::scoreboard_round_input
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::RoundOver)),
        );

        app.add_systems(
            OnEnter(GameplayState::MatchOver),
            scoreboard::setup_match_scoreboard,
        )
        .add_systems(
            OnExit(GameplayState::MatchOver),
            scoreboard::cleanup_scoreboard,
        )
        .add_systems(
            Update,
            scoreboard::scoreboard_match_input
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::MatchOver)),
        );

        // =====================================================================
        // Apply GameSetupConfig to GameModeConfig when starting gameplay
        // =====================================================================
        app.add_systems(OnEnter(GameState::Gameplay), apply_game_setup_config);
    }
}

/// Despawns all menu-related UI when leaving the Menu state, regardless of
/// which submenu is currently active (Main, GameSettings, PlayerSelect, etc.).
fn cleanup_all_menu_ui(
    mut commands: Commands,
    main_menu_query: Query<Entity, With<MenuRoot>>,
    settings_query: Query<Entity, With<GameSettingsRoot>>,
    player_select_query: Query<Entity, With<PlayerSelectRoot>>,
) {
    for entity in main_menu_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in settings_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in player_select_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Resets menu-related resources when leaving the Menu state so they start
/// fresh next time the Menu state is entered.
fn cleanup_menu_resources(
    mut menu_sel: ResMut<MenuSelection>,
    mut game_sel: ResMut<GameSettingsSelection>,
    mut player_sel: ResMut<PlayerSelectState>,
) {
    *menu_sel = MenuSelection::default();
    *game_sel = GameSettingsSelection::default();
    *player_sel = PlayerSelectState::default();
}

/// Resets the active submenu to Main when (re-)entering the Menu state.
/// Uses `bypass_change_detection` so that the transition system does not
/// see this as a change and redundantly despawn/respawn the main menu UI
/// that `setup_main_menu` is about to create in the same OnEnter schedule.
fn reset_submenu_to_main(mut current_sub: ResMut<CurrentSubMenu>) {
    current_sub.bypass_change_detection().submenu = SubMenu::Main;
}

/// Handles transitions between submenus: despawns old UI and spawns new UI.
fn submenu_transition_system(
    mut commands: Commands,
    current_sub: Res<CurrentSubMenu>,
    // Existing UI queries for cleanup
    main_menu_query: Query<Entity, With<MenuRoot>>,
    settings_query: Query<Entity, With<GameSettingsRoot>>,
    player_select_query: Query<Entity, With<PlayerSelectRoot>>,
    game_setup: Res<GameSetupConfig>,
) {
    if !current_sub.is_changed() {
        return;
    }

    // Clean up all submenu UIs
    for entity in main_menu_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in settings_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in player_select_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Spawn the appropriate submenu
    match current_sub.submenu {
        SubMenu::Main => {
            main_menu::setup_main_menu(commands);
        }
        SubMenu::GameSettings => {
            game_settings::setup_game_settings(commands, game_setup);
        }
        SubMenu::PlayerSelect => {
            player_select::setup_player_select(commands, game_setup);
        }
        SubMenu::Options => {
            // Options menu is a placeholder for now; spawn main menu instead
            main_menu::setup_main_menu(commands);
        }
    }
}

/// When entering Gameplay, apply the GameSetupConfig to the GameModeConfig and RoundManager.
fn apply_game_setup_config(
    game_setup: Res<GameSetupConfig>,
    mut mode_config: ResMut<crate::game_modes::traits::GameModeConfig>,
    mut round_manager: ResMut<crate::game_modes::traits::RoundManager>,
) {
    mode_config.mode_type = game_setup.selected_mode;
    mode_config.kill_limit = game_setup.kill_limit;
    mode_config.time_limit_secs = game_setup.time_limit_secs;
    mode_config.point_limit = game_setup.point_limit;
    mode_config.lives = game_setup.lives;

    round_manager.total_rounds = game_setup.total_rounds;
    round_manager.current_round = 1;
    round_manager.match_finished = false;
    round_manager.round_wins = [0; crate::core::constants::MAX_PLAYERS];
    round_manager.team_round_wins = [0; crate::core::constants::MAX_PLAYERS];
}
