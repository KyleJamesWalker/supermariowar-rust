pub mod components;
pub mod main_menu;

use bevy::prelude::*;

use crate::core::game_state::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), main_menu::setup_main_menu)
            .add_systems(
                Update,
                main_menu::menu_input.run_if(in_state(GameState::Menu)),
            )
            .add_systems(OnExit(GameState::Menu), main_menu::cleanup_main_menu);
    }
}
