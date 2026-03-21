use bevy::prelude::*;

use crate::game_modes::traits::GameModeType;
use super::components::*;

/// All game mode types in order, for cycling through.
const ALL_MODES: [GameModeType; 22] = [
    GameModeType::Classic,
    GameModeType::Frag,
    GameModeType::TimeLimit,
    GameModeType::Jail,
    GameModeType::CoinCollection,
    GameModeType::Stomp,
    GameModeType::Eggs,
    GameModeType::CaptureTheFlag,
    GameModeType::Chicken,
    GameModeType::Tag,
    GameModeType::Star,
    GameModeType::Domination,
    GameModeType::KingOfTheHill,
    GameModeType::Race,
    GameModeType::Owned,
    GameModeType::Frenzy,
    GameModeType::Survival,
    GameModeType::Greed,
    GameModeType::Health,
    GameModeType::CardCollection,
    GameModeType::Chase,
    GameModeType::ShyGuyTag,
];

/// Returns a user-friendly name for a game mode.
pub fn mode_display_name(mode: GameModeType) -> &'static str {
    match mode {
        GameModeType::Classic => "Classic",
        GameModeType::Frag => "Frag Limit",
        GameModeType::TimeLimit => "Time Limit",
        GameModeType::Jail => "Jail",
        GameModeType::CoinCollection => "Coin Collection",
        GameModeType::Stomp => "Stomp",
        GameModeType::Eggs => "Eggs",
        GameModeType::CaptureTheFlag => "Capture The Flag",
        GameModeType::Chicken => "Chicken",
        GameModeType::Tag => "Tag",
        GameModeType::Star => "Star",
        GameModeType::Domination => "Domination",
        GameModeType::KingOfTheHill => "King of the Hill",
        GameModeType::Race => "Race",
        GameModeType::Owned => "Owned",
        GameModeType::Frenzy => "Frenzy",
        GameModeType::Survival => "Survival",
        GameModeType::Greed => "Greed",
        GameModeType::Health => "Health",
        GameModeType::CardCollection => "Card Collection",
        GameModeType::Chase => "Chase",
        GameModeType::ShyGuyTag => "Shy Guy Tag",
    }
}

/// Returns a short description for each game mode.
pub fn mode_description(mode: GameModeType) -> &'static str {
    match mode {
        GameModeType::Classic => "Stomp or kill opponents to reach the kill limit.",
        GameModeType::Frag => "Kill opponents with any method to reach the frag limit.",
        GameModeType::TimeLimit => "Score the most kills before time runs out.",
        GameModeType::Jail => "Send opponents to jail. Free teammates by touching them.",
        GameModeType::CoinCollection => "Collect coins scattered around the map.",
        GameModeType::Stomp => "Only stomps count! Chain stomps for bonus points.",
        GameModeType::Eggs => "Collect Yoshi eggs and return them to your nest.",
        GameModeType::CaptureTheFlag => "Grab the enemy flag and return it to your base.",
        GameModeType::Chicken => "The chicken scores points. Kill the chicken to become it!",
        GameModeType::Tag => "You're it! Tag someone else before time runs out.",
        GameModeType::Star => "Collect stars that appear around the map.",
        GameModeType::Domination => "Control zones on the map to score points over time.",
        GameModeType::KingOfTheHill => "Hold the hill zone to score points.",
        GameModeType::Race => "Race around the map, passing through checkpoints.",
        GameModeType::Owned => "Tag tiles by walking on them. Own the most tiles!",
        GameModeType::Frenzy => "Powerups rain from the sky! Survive the chaos.",
        GameModeType::Survival => "Last player standing wins. Limited lives.",
        GameModeType::Greed => "Collect coins dropped by other players when killed.",
        GameModeType::Health => "Players have HP. Reduce opponents to zero.",
        GameModeType::CardCollection => "Collect matching cards for bonus points.",
        GameModeType::Chase => "Chase the phantom! Touch it to score.",
        GameModeType::ShyGuyTag => "Avoid the Shy Guys! Last player untagged wins.",
    }
}

/// Finds the index of a mode in the ALL_MODES array.
fn mode_index(mode: GameModeType) -> usize {
    ALL_MODES
        .iter()
        .position(|m| *m == mode)
        .unwrap_or(0)
}

/// Cycles to the next game mode.
fn next_mode(current: GameModeType) -> GameModeType {
    let idx = mode_index(current);
    ALL_MODES[(idx + 1) % ALL_MODES.len()]
}

/// Cycles to the previous game mode.
fn prev_mode(current: GameModeType) -> GameModeType {
    let idx = mode_index(current);
    if idx == 0 {
        ALL_MODES[ALL_MODES.len() - 1]
    } else {
        ALL_MODES[idx - 1]
    }
}

/// The ordered list of fields in the game settings menu.
const SETTINGS_FIELDS: [GameSettingField; 8] = [
    GameSettingField::Mode,
    GameSettingField::KillLimit,
    GameSettingField::TimeLimit,
    GameSettingField::PointLimit,
    GameSettingField::Lives,
    GameSettingField::Rounds,
    GameSettingField::Start,
    GameSettingField::Back,
];

/// Spawns the game settings submenu UI.
pub fn setup_game_settings(
    mut commands: Commands,
    game_setup: Res<GameSetupConfig>,
) {
    commands
        .spawn((
            GameSettingsRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.2)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("GAME SETTINGS"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Mode setting
            spawn_setting_row(
                parent,
                GameSettingField::Mode,
                "Mode",
                &format!("< {} >", mode_display_name(game_setup.selected_mode)),
            );

            // Kill limit
            spawn_setting_row(
                parent,
                GameSettingField::KillLimit,
                "Kill Limit",
                &format!("< {} >", game_setup.kill_limit),
            );

            // Time limit
            spawn_setting_row(
                parent,
                GameSettingField::TimeLimit,
                "Time Limit",
                &format!("< {}s >", game_setup.time_limit_secs),
            );

            // Point limit
            spawn_setting_row(
                parent,
                GameSettingField::PointLimit,
                "Point Limit",
                &format!("< {} >", game_setup.point_limit),
            );

            // Lives
            spawn_setting_row(
                parent,
                GameSettingField::Lives,
                "Lives",
                &format!("< {} >", game_setup.lives),
            );

            // Rounds
            spawn_setting_row(
                parent,
                GameSettingField::Rounds,
                "Rounds",
                &format!("< {} >", game_setup.total_rounds),
            );

            // Mode description
            parent.spawn((
                Text::new(mode_description(game_setup.selected_mode)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
                Node {
                    margin: UiRect::vertical(Val::Px(10.0)),
                    max_width: Val::Px(500.0),
                    ..default()
                },
                ModeDescriptionText,
            ));

            // Start button
            spawn_button_row(parent, GameSettingField::Start, "[ START GAME ]");

            // Back button
            spawn_button_row(parent, GameSettingField::Back, "[ BACK ]");
        });

    commands.insert_resource(GameSettingsSelection {
        selected_index: 0,
        field_count: SETTINGS_FIELDS.len(),
    });
}

/// Marker for the mode description text (updated when mode changes).
#[derive(Component, Debug)]
pub struct ModeDescriptionText;

/// Marker for a setting value text (updated when values change).
#[derive(Component, Debug)]
pub struct SettingValueText {
    pub field: GameSettingField,
}

fn spawn_setting_row(parent: &mut ChildBuilder, field: GameSettingField, label: &str, value: &str) {
    parent
        .spawn((
            field,
            Node {
                width: Val::Px(450.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(10.0)),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                SettingValueText { field },
                Text::new(value),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_button_row(parent: &mut ChildBuilder, field: GameSettingField, label: &str) {
    parent.spawn((
        field,
        Text::new(label),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
        Node {
            margin: UiRect::top(Val::Px(6.0)),
            ..default()
        },
    ));
}

/// Handles input in the game settings submenu.
pub fn game_settings_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<GameSettingsSelection>,
    mut game_setup: ResMut<GameSetupConfig>,
    mut current_sub: ResMut<CurrentSubMenu>,
    mut next_state: ResMut<NextState<crate::core::game_state::GameState>>,
) {
    if current_sub.submenu != SubMenu::GameSettings {
        return;
    }

    // Navigation — guard against field_count == 0 (resource not yet initialised)
    if selection.field_count == 0 {
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if selection.selected_index > 0 {
            selection.selected_index -= 1;
        } else {
            selection.selected_index = selection.field_count - 1;
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        selection.selected_index = (selection.selected_index + 1) % selection.field_count;
    }

    let current_field = SETTINGS_FIELDS[selection.selected_index];

    // Left/Right to adjust values
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA) {
        match current_field {
            GameSettingField::Mode => {
                game_setup.selected_mode = prev_mode(game_setup.selected_mode);
            }
            GameSettingField::KillLimit => {
                game_setup.kill_limit = game_setup.kill_limit.saturating_sub(1).max(1);
            }
            GameSettingField::TimeLimit => {
                game_setup.time_limit_secs = game_setup.time_limit_secs.saturating_sub(10).max(10);
            }
            GameSettingField::PointLimit => {
                game_setup.point_limit = game_setup.point_limit.saturating_sub(1).max(1);
            }
            GameSettingField::Lives => {
                game_setup.lives = game_setup.lives.saturating_sub(1).max(1);
            }
            GameSettingField::Rounds => {
                game_setup.total_rounds = game_setup.total_rounds.saturating_sub(1).max(1);
            }
            _ => {}
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
        match current_field {
            GameSettingField::Mode => {
                game_setup.selected_mode = next_mode(game_setup.selected_mode);
            }
            GameSettingField::KillLimit => {
                game_setup.kill_limit = (game_setup.kill_limit + 1).min(50);
            }
            GameSettingField::TimeLimit => {
                game_setup.time_limit_secs = (game_setup.time_limit_secs + 10).min(600);
            }
            GameSettingField::PointLimit => {
                game_setup.point_limit = (game_setup.point_limit + 1).min(50);
            }
            GameSettingField::Lives => {
                game_setup.lives = (game_setup.lives + 1).min(20);
            }
            GameSettingField::Rounds => {
                game_setup.total_rounds = (game_setup.total_rounds + 1).min(10);
            }
            _ => {}
        }
    }

    // Confirm/Enter
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        match current_field {
            GameSettingField::Start => {
                next_state.set(crate::core::game_state::GameState::Gameplay);
            }
            GameSettingField::Back => {
                current_sub.submenu = SubMenu::Main;
            }
            _ => {}
        }
    }

    // Escape -> back
    if keyboard.just_pressed(KeyCode::Escape) {
        current_sub.submenu = SubMenu::Main;
    }
}

/// Updates visual highlights and value text in the game settings menu.
pub fn update_game_settings_display(
    selection: Res<GameSettingsSelection>,
    game_setup: Res<GameSetupConfig>,
    current_sub: Res<CurrentSubMenu>,
    mut field_query: Query<(&GameSettingField, &mut BackgroundColor), Without<SettingValueText>>,
    mut value_query: Query<(&SettingValueText, &mut Text)>,
    mut desc_query: Query<&mut Text, (With<ModeDescriptionText>, Without<SettingValueText>)>,
    mut text_color_query: Query<(&GameSettingField, &mut TextColor), Without<SettingValueText>>,
) {
    if current_sub.submenu != SubMenu::GameSettings || selection.field_count == 0 {
        return;
    }

    let selected_field = SETTINGS_FIELDS[selection.selected_index];

    // Update highlight colors on field rows
    for (field, mut bg) in field_query.iter_mut() {
        if *field == selected_field {
            *bg = BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.4));
        } else {
            *bg = BackgroundColor(Color::NONE);
        }
    }

    // Update button text colors (Start, Back)
    for (field, mut color) in text_color_query.iter_mut() {
        if *field == selected_field {
            *color = TextColor(Color::srgb(1.0, 1.0, 0.0));
        } else {
            *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
        }
    }

    // Update value texts
    for (setting_value, mut text) in value_query.iter_mut() {
        let new_text = match setting_value.field {
            GameSettingField::Mode => {
                format!("< {} >", mode_display_name(game_setup.selected_mode))
            }
            GameSettingField::KillLimit => format!("< {} >", game_setup.kill_limit),
            GameSettingField::TimeLimit => format!("< {}s >", game_setup.time_limit_secs),
            GameSettingField::PointLimit => format!("< {} >", game_setup.point_limit),
            GameSettingField::Lives => format!("< {} >", game_setup.lives),
            GameSettingField::Rounds => format!("< {} >", game_setup.total_rounds),
            _ => continue,
        };
        *text = Text::new(new_text);
    }

    // Update mode description
    for mut text in desc_query.iter_mut() {
        *text = Text::new(mode_description(game_setup.selected_mode));
    }
}

/// Despawns the game settings submenu.
pub fn cleanup_game_settings(
    mut commands: Commands,
    query: Query<Entity, With<GameSettingsRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
