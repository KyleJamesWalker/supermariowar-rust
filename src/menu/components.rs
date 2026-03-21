use bevy::prelude::*;

/// Marker for the main menu root entity.
#[derive(Component, Debug)]
pub struct MenuRoot;

/// Marker for a menu item that can be selected.
#[derive(Component, Debug)]
pub struct MenuItem {
    pub index: usize,
    pub label: String,
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
