use bevy::prelude::*;

use crate::core::game_state::GameState;
use super::components::*;

/// Spawns the main menu UI when entering the Menu state.
pub fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                TitleText,
                Text::new("Super Mario War"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Menu items
            let items = ["Start Game", "Options", "Quit"];
            for (i, label) in items.iter().enumerate() {
                parent.spawn((
                    MenuItem {
                        index: i,
                        label: label.to_string(),
                    },
                    Text::new(*label),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                ));
            }
        });

    commands.insert_resource(MenuSelection {
        selected_index: 0,
        item_count: 3,
    });
}

/// Handles keyboard navigation in the main menu.
pub fn menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
    mut next_state: ResMut<NextState<GameState>>,
    mut query: Query<(&MenuItem, &mut TextColor)>,
) {
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

    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        match selection.selected_index {
            0 => next_state.set(GameState::Gameplay),
            2 => {
                // Quit handled by Bevy's AppExit event
                std::process::exit(0);
            }
            _ => {}
        }
    }

    // Update visual selection
    for (item, mut color) in query.iter_mut() {
        if item.index == selection.selected_index {
            *color = TextColor(Color::srgba(1.0, 1.0, 0.0, 1.0));
        } else {
            *color = TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0));
        }
    }
}

/// Despawns the main menu when leaving the Menu state.
pub fn cleanup_main_menu(
    mut commands: Commands,
    query: Query<Entity, With<MenuRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
