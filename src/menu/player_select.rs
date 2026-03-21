use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use super::components::*;
use super::main_menu::TEAM_COLORS;

/// Spawns the player select screen UI.
pub fn setup_player_select(
    mut commands: Commands,
    game_setup: Res<GameSetupConfig>,
) {
    commands
        .spawn((
            PlayerSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.2)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("PLAYER SELECT"),
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

            // Instructions
            parent.spawn((
                Text::new("Up/Down: Select Slot | Left/Right: Change | Tab: Next Field | Enter: Confirm | Esc: Back"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(15.0)),
                    ..default()
                },
            ));

            // Player slots container
            parent
                .spawn(Node {
                    width: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|container| {
                    for i in 0..MAX_PLAYERS {
                        spawn_player_slot(container, i, &game_setup.player_slots[i]);
                    }
                });

            // Bottom buttons
            parent.spawn((
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(40.0),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    PlayerSelectConfirmButton,
                    Text::new("[ CONFIRM ]"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
                ));
                row.spawn((
                    PlayerSelectBackButton,
                    Text::new("[ BACK ]"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
                ));
            });
        });

    commands.insert_resource(PlayerSelectState::default());
}

/// Marker for confirm button on player select.
#[derive(Component, Debug)]
pub struct PlayerSelectConfirmButton;

/// Marker for back button on player select.
#[derive(Component, Debug)]
pub struct PlayerSelectBackButton;

/// Marker for a specific text element within a player slot.
#[derive(Component, Debug)]
pub struct PlayerSlotText {
    pub player_index: usize,
    pub field: PlayerSelectField,
}

fn spawn_player_slot(parent: &mut ChildBuilder, index: usize, slot: &PlayerSlotConfig) {
    let team_color = TEAM_COLORS[slot.team as usize % TEAM_COLORS.len()];

    parent
        .spawn((
            PlayerSlotMarker { player_index: index },
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5)),
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.6)),
        ))
        .with_children(|row| {
            // Player number
            row.spawn((
                Text::new(format!("P{}", index + 1)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(team_color),
                Node {
                    width: Val::Px(50.0),
                    ..default()
                },
            ));

            // Control type
            row.spawn((
                PlayerSlotText {
                    player_index: index,
                    field: PlayerSelectField::ControlType,
                },
                Text::new(slot.control_type.display_name()),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    width: Val::Px(100.0),
                    ..default()
                },
            ));

            // Team color indicator
            row.spawn((
                PlayerSlotText {
                    player_index: index,
                    field: PlayerSelectField::Team,
                },
                Text::new(format!("Team {}", slot.team + 1)),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(team_color),
                Node {
                    width: Val::Px(100.0),
                    ..default()
                },
            ));

            // CPU difficulty (only visible for CPU players)
            let diff_text = if slot.control_type == ControlType::Cpu {
                slot.cpu_difficulty.display_name().to_string()
            } else {
                "---".to_string()
            };
            row.spawn((
                PlayerSlotText {
                    player_index: index,
                    field: PlayerSelectField::CpuDifficulty,
                },
                Text::new(diff_text),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                Node {
                    width: Val::Px(100.0),
                    ..default()
                },
            ));

            // Skin preview (just index for now)
            row.spawn((
                Text::new(format!("Skin {}", slot.skin_index)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                Node {
                    width: Val::Px(80.0),
                    ..default()
                },
            ));
        });
}

/// Handles input on the player select screen.
pub fn player_select_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut select_state: ResMut<PlayerSelectState>,
    mut game_setup: ResMut<GameSetupConfig>,
    mut current_sub: ResMut<CurrentSubMenu>,
) {
    if current_sub.submenu != SubMenu::PlayerSelect {
        return;
    }

    // Up/Down: navigate between player slots
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if select_state.active_slot > 0 {
            select_state.active_slot -= 1;
        } else {
            select_state.active_slot = MAX_PLAYERS - 1;
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        select_state.active_slot = (select_state.active_slot + 1) % MAX_PLAYERS;
    }

    // Tab: cycle through fields within the active slot
    if keyboard.just_pressed(KeyCode::Tab) {
        select_state.active_field = match select_state.active_field {
            PlayerSelectField::ControlType => PlayerSelectField::Team,
            PlayerSelectField::Team => PlayerSelectField::CpuDifficulty,
            PlayerSelectField::CpuDifficulty => PlayerSelectField::Ready,
            PlayerSelectField::Ready => PlayerSelectField::ControlType,
        };
    }

    let slot_idx = select_state.active_slot;

    // Left/Right: change the value of the currently selected field
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA) {
        match select_state.active_field {
            PlayerSelectField::ControlType => {
                // Cycle backwards: None -> Cpu -> Human -> None
                game_setup.player_slots[slot_idx].control_type =
                    match game_setup.player_slots[slot_idx].control_type {
                        ControlType::Human => ControlType::None,
                        ControlType::Cpu => ControlType::Human,
                        ControlType::None => ControlType::Cpu,
                    };
            }
            PlayerSelectField::Team => {
                let t = &mut game_setup.player_slots[slot_idx].team;
                if *t == 0 {
                    *t = 3;
                } else {
                    *t -= 1;
                }
            }
            PlayerSelectField::CpuDifficulty => {
                let d = &mut game_setup.player_slots[slot_idx].cpu_difficulty;
                *d = match *d {
                    CpuDifficulty::Easy => CpuDifficulty::Hard,
                    CpuDifficulty::Medium => CpuDifficulty::Easy,
                    CpuDifficulty::Hard => CpuDifficulty::Medium,
                };
            }
            PlayerSelectField::Ready => {}
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD) {
        match select_state.active_field {
            PlayerSelectField::ControlType => {
                game_setup.player_slots[slot_idx].control_type =
                    game_setup.player_slots[slot_idx].control_type.cycle_next();
            }
            PlayerSelectField::Team => {
                let t = &mut game_setup.player_slots[slot_idx].team;
                *t = (*t + 1) % 4;
            }
            PlayerSelectField::CpuDifficulty => {
                game_setup.player_slots[slot_idx].cpu_difficulty =
                    game_setup.player_slots[slot_idx].cpu_difficulty.cycle_next();
            }
            PlayerSelectField::Ready => {}
        }
    }

    // Enter: confirm and go back (require at least 2 active players)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        let active_count = game_setup
            .player_slots
            .iter()
            .filter(|s| s.control_type != ControlType::None)
            .count();
        if active_count >= 2 {
            game_setup.num_players = active_count;
            current_sub.submenu = SubMenu::Main;
        } else {
            info!("Need at least 2 active players to start a game");
        }
    }

    // Escape: go back without confirming
    if keyboard.just_pressed(KeyCode::Escape) {
        current_sub.submenu = SubMenu::Main;
    }
}

/// Updates the visual display of the player select screen.
pub fn update_player_select_display(
    current_sub: Res<CurrentSubMenu>,
    select_state: Res<PlayerSelectState>,
    game_setup: Res<GameSetupConfig>,
    mut slot_query: Query<(&PlayerSlotMarker, &mut BorderColor)>,
    mut text_query: Query<(&PlayerSlotText, &mut Text, &mut TextColor)>,
    mut confirm_query: Query<
        &mut TextColor,
        (With<PlayerSelectConfirmButton>, Without<PlayerSlotText>, Without<PlayerSelectBackButton>),
    >,
    mut back_query: Query<
        &mut TextColor,
        (With<PlayerSelectBackButton>, Without<PlayerSlotText>, Without<PlayerSelectConfirmButton>),
    >,
) {
    if current_sub.submenu != SubMenu::PlayerSelect {
        return;
    }

    // Highlight active slot border
    for (marker, mut border) in slot_query.iter_mut() {
        if marker.player_index == select_state.active_slot {
            *border = BorderColor(Color::srgb(1.0, 1.0, 0.0));
        } else {
            *border = BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5));
        }
    }

    // Update slot text values and highlight active field
    for (slot_text, mut text, mut color) in text_query.iter_mut() {
        let slot = &game_setup.player_slots[slot_text.player_index];
        let is_active_slot = slot_text.player_index == select_state.active_slot;
        let is_active_field = is_active_slot && slot_text.field == select_state.active_field;

        match slot_text.field {
            PlayerSelectField::ControlType => {
                *text = Text::new(slot.control_type.display_name());
            }
            PlayerSelectField::Team => {
                *text = Text::new(format!("Team {}", slot.team + 1));
                let team_color = TEAM_COLORS[slot.team as usize % TEAM_COLORS.len()];
                if !is_active_field {
                    *color = TextColor(team_color);
                    continue;
                }
            }
            PlayerSelectField::CpuDifficulty => {
                if slot.control_type == ControlType::Cpu {
                    *text = Text::new(slot.cpu_difficulty.display_name());
                } else {
                    *text = Text::new("---");
                }
            }
            PlayerSelectField::Ready => {}
        }

        if is_active_field {
            *color = TextColor(Color::srgb(1.0, 1.0, 0.0));
        } else if is_active_slot {
            *color = TextColor(Color::WHITE);
        } else {
            *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
        }
    }

    // Button colors
    for mut color in confirm_query.iter_mut() {
        *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
    }
    for mut color in back_query.iter_mut() {
        *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
    }
}

/// Despawns the player select screen.
pub fn cleanup_player_select(
    mut commands: Commands,
    query: Query<Entity, With<PlayerSelectRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
