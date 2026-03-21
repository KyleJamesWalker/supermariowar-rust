use bevy::prelude::*;

use crate::core::game_state::GameState;
use super::components::*;

/// Team colors used throughout the UI.
pub const TEAM_COLORS: [Color; 4] = [
    Color::srgb(1.0, 0.2, 0.2),   // Red
    Color::srgb(0.2, 0.5, 1.0),   // Blue
    Color::srgb(0.2, 0.9, 0.2),   // Green
    Color::srgb(1.0, 0.85, 0.0),  // Yellow
];

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
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.2)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                TitleText,
                Text::new("SUPER MARIO WAR"),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            // Subtitle
            parent.spawn((
                Text::new("Bevy/Rust Edition"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Menu items
            let items = [
                ("Start Game", MenuAction::StartGame, true),
                ("Game Settings", MenuAction::OpenGameSettings, true),
                ("Player Select", MenuAction::OpenPlayerSelect, true),
                ("Options", MenuAction::OpenOptions, true),
                ("Quit", MenuAction::Quit, true),
            ];

            for (i, (label, action, enabled)) in items.iter().enumerate() {
                parent.spawn((
                    MenuItem {
                        index: i,
                        label: label.to_string(),
                        enabled: *enabled,
                        action: *action,
                    },
                    Text::new(*label),
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

            // Version text at bottom
            parent.spawn((
                VersionText,
                Text::new("v0.1.0"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                },
            ));
        });

    commands.insert_resource(MenuSelection {
        selected_index: 0,
        item_count: 5,
    });
}

/// Handles keyboard/gamepad navigation in the main menu.
pub fn main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
    mut next_state: ResMut<NextState<GameState>>,
    mut current_sub: ResMut<CurrentSubMenu>,
    mut query: Query<(&MenuItem, &mut TextColor)>,
    mut exit_events: EventWriter<AppExit>,
) {
    if current_sub.submenu != SubMenu::Main {
        return;
    }

    // Navigation
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

    // Confirm selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Find the action for the selected item
        let mut selected_action = None;
        for (item, _) in query.iter() {
            if item.index == selection.selected_index && item.enabled {
                selected_action = Some(item.action);
                break;
            }
        }

        if let Some(action) = selected_action {
            match action {
                MenuAction::StartGame => {
                    next_state.set(GameState::Gameplay);
                }
                MenuAction::OpenGameSettings => {
                    current_sub.submenu = SubMenu::GameSettings;
                }
                MenuAction::OpenPlayerSelect => {
                    current_sub.submenu = SubMenu::PlayerSelect;
                }
                MenuAction::OpenOptions => {
                    current_sub.submenu = SubMenu::Options;
                }
                MenuAction::Quit => {
                    exit_events.send(AppExit::Success);
                }
                _ => {}
            }
        }
    }

    // Update visual selection
    for (item, mut color) in query.iter_mut() {
        if item.index == selection.selected_index {
            *color = TextColor(Color::srgb(1.0, 1.0, 0.0));
        } else if item.enabled {
            *color = TextColor(Color::srgba(0.6, 0.6, 0.6, 1.0));
        } else {
            *color = TextColor(Color::srgba(0.3, 0.3, 0.3, 0.5));
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

/// Despawns main menu UI when switching to a submenu (the submenu spawns its own).
pub fn hide_main_menu(
    mut commands: Commands,
    query: Query<Entity, With<MenuRoot>>,
    current_sub: Res<CurrentSubMenu>,
) {
    if current_sub.submenu != SubMenu::Main {
        for entity in query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Respawns main menu UI when returning from a submenu.
pub fn show_main_menu_on_return(
    commands: Commands,
    query: Query<Entity, With<MenuRoot>>,
    current_sub: Res<CurrentSubMenu>,
) {
    if current_sub.is_changed() && current_sub.submenu == SubMenu::Main && query.is_empty() {
        setup_main_menu(commands);
    }
}
