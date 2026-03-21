use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use crate::game_modes::PendingRoundReset;
use crate::game_modes::traits::RoundCountdown;
use crate::player::components::PauseEvent;
use super::components::*;

/// Listens for PauseEvent and toggles between Playing and Paused.
pub fn handle_pause_toggle(
    mut pause_events: EventReader<PauseEvent>,
    gameplay_state: Res<State<GameplayState>>,
    mut next_gameplay_state: ResMut<NextState<GameplayState>>,
) {
    if let Some(_event) = pause_events.read().next() {
        match gameplay_state.get() {
            GameplayState::Playing => {
                next_gameplay_state.set(GameplayState::Paused);
            }
            GameplayState::Paused => {
                next_gameplay_state.set(GameplayState::Playing);
            }
            _ => {}
        }
    }
}

/// Spawns the pause menu overlay when entering the Paused state.
pub fn setup_pause_menu(mut commands: Commands, mut selection: ResMut<PauseMenuSelection>) {
    // Semi-transparent overlay
    commands
        .spawn((
            PauseMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            // Pause title
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Menu items
            let items = [
                (0, "Resume", MenuAction::Resume),
                (1, "Restart", MenuAction::Restart),
                (2, "Quit to Menu", MenuAction::QuitToMenu),
            ];

            for (index, label, action) in items {
                parent.spawn((
                    PauseMenuItem { index, action },
                    Text::new(label),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0)),
                    Node {
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                ));
            }
        });

    *selection = PauseMenuSelection {
        selected_index: 0,
        item_count: 3,
    };
}

/// Handles input within the pause menu.
pub fn pause_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<PauseMenuSelection>,
    mut next_gameplay_state: ResMut<NextState<GameplayState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut pending_reset: ResMut<PendingRoundReset>,
    mut countdown: ResMut<RoundCountdown>,
    query: Query<&PauseMenuItem>,
) {
    // Navigation — guard against item_count == 0 (resource not yet initialised)
    if selection.item_count == 0 {
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if selection.selected_index > 0 {
            selection.selected_index -= 1;
        } else {
            selection.selected_index = selection.item_count - 1;
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        selection.selected_index = (selection.selected_index + 1) % selection.item_count;
    }

    // Confirm
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        let mut selected_action = None;
        for item in query.iter() {
            if item.index == selection.selected_index {
                selected_action = Some(item.action);
                break;
            }
        }

        if let Some(action) = selected_action {
            match action {
                MenuAction::Resume => {
                    next_gameplay_state.set(GameplayState::Playing);
                }
                MenuAction::Restart => {
                    // Signal mode state reset (same path as round-over)
                    pending_reset.0 = true;
                    *countdown = RoundCountdown::default();
                    next_gameplay_state.set(GameplayState::Countdown);
                }
                MenuAction::QuitToMenu => {
                    next_game_state.set(GameState::Menu);
                }
                _ => {}
            }
        }
    }

    // Escape also resumes
    if keyboard.just_pressed(KeyCode::Escape) {
        next_gameplay_state.set(GameplayState::Playing);
    }
}

/// Updates visual feedback for the pause menu selection.
pub fn update_pause_menu_display(
    selection: Res<PauseMenuSelection>,
    mut query: Query<(&PauseMenuItem, &mut TextColor)>,
) {
    for (item, mut color) in query.iter_mut() {
        if item.index == selection.selected_index {
            *color = TextColor(Color::srgb(1.0, 1.0, 0.0));
        } else {
            *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
        }
    }
}

/// Despawns the pause menu overlay when leaving the Paused state.
pub fn cleanup_pause_menu(
    mut commands: Commands,
    query: Query<Entity, With<PauseMenuRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
