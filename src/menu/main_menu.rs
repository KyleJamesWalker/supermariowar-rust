use bevy::prelude::*;

use crate::core::game_state::GameState;
use crate::graphics::sprites::SpriteSheets;
use super::components::*;

/// Team colors used throughout the UI.
pub const TEAM_COLORS: [Color; 4] = [
    Color::srgb(1.0, 0.2, 0.2),   // Red
    Color::srgb(0.2, 0.5, 1.0),   // Blue
    Color::srgb(0.2, 0.9, 0.2),   // Green
    Color::srgb(1.0, 0.85, 0.0),  // Yellow
];

/// Spawns the main menu UI when entering the Menu state.
pub fn setup_main_menu(mut commands: Commands, sprite_sheets: Res<SpriteSheets>) {
    // Background image (menu_background.png: 640x480)
    let bg_handle = sprite_sheets.menu_images.get("menu_background").cloned();

    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.2)),
        ))
        .with_children(|parent| {
            // Background image overlay (absolute positioned behind everything)
            if let Some(bg) = bg_handle {
                parent.spawn((
                    ImageNode::new(bg),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }

            // Title logo (menu_smw.png: 372x140)
            if let Some(logo) = sprite_sheets.menu_images.get("menu_smw").cloned() {
                parent.spawn((
                    TitleText,
                    ImageNode::new(logo),
                    Node {
                        width: Val::Px(372.0),
                        height: Val::Px(140.0),
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                ));
            } else {
                // Fallback: text title if image not loaded
                parent.spawn((
                    TitleText,
                    Text::new("SUPER MARIO WAR"),
                    TextFont {
                        font_size: 52.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.85, 0.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                ));
            }

            // Subtitle
            parent.spawn((
                Text::new("Bevy/Rust Edition"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
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
                // Each menu item is a container with a background + text
                parent
                    .spawn((
                        MenuItem {
                            index: i,
                            label: label.to_string(),
                            enabled: *enabled,
                            action: *action,
                        },
                        Node {
                            width: Val::Px(400.0),
                            height: Val::Px(32.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                        BorderRadius::all(Val::Px(4.0)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            MenuItemText { index: i },
                            Text::new(*label),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
                        ));
                    });
            }

            // Version text at bottom
            if let Some(ver_img) = sprite_sheets.menu_images.get("menu_version").cloned() {
                parent.spawn((
                    VersionText,
                    ImageNode::new(ver_img),
                    Node {
                        margin: UiRect::top(Val::Px(30.0)),
                        height: Val::Px(16.0),
                        ..default()
                    },
                ));
            } else {
                parent.spawn((
                    VersionText,
                    Text::new("v0.1.0"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.6)),
                    Node {
                        margin: UiRect::top(Val::Px(30.0)),
                        ..default()
                    },
                ));
            }
        });

    commands.insert_resource(MenuSelection {
        selected_index: 0,
        item_count: 5,
    });
}

/// Marker for a menu item's text child (for color updates).
#[derive(Component, Debug)]
pub struct MenuItemText {
    pub index: usize,
}

/// Handles keyboard/gamepad navigation in the main menu.
pub fn main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
    mut next_state: ResMut<NextState<GameState>>,
    mut current_sub: ResMut<CurrentSubMenu>,
    item_query: Query<(&MenuItem, &Children)>,
    mut text_color_query: Query<(&MenuItemText, &mut TextColor)>,
    mut bg_query: Query<(&MenuItem, &mut BackgroundColor), Without<MenuItemText>>,
    mut exit_events: EventWriter<AppExit>,
) {
    if current_sub.submenu != SubMenu::Main {
        return;
    }

    // Navigation
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        if selection.item_count == 0 {
            // Not yet initialized
        } else if selection.selected_index > 0 {
            selection.selected_index -= 1;
        } else {
            selection.selected_index = selection.item_count - 1;
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        if selection.item_count > 0 {
            selection.selected_index = (selection.selected_index + 1) % selection.item_count;
        }
    }

    // Confirm selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        let mut selected_action = None;
        for (item, _children) in item_query.iter() {
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

    // Update visual selection - background colors on item containers
    for (item, mut bg) in bg_query.iter_mut() {
        if item.index == selection.selected_index {
            *bg = BackgroundColor(Color::srgba(0.15, 0.4, 0.15, 0.7));
        } else {
            *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35));
        }
    }

    // Update text colors
    for (text_marker, mut color) in text_color_query.iter_mut() {
        if text_marker.index == selection.selected_index {
            *color = TextColor(Color::srgb(1.0, 1.0, 1.0));
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
    sprite_sheets: Res<SpriteSheets>,
    query: Query<Entity, With<MenuRoot>>,
    current_sub: Res<CurrentSubMenu>,
) {
    if current_sub.is_changed() && current_sub.submenu == SubMenu::Main && query.is_empty() {
        setup_main_menu(commands, sprite_sheets);
    }
}
