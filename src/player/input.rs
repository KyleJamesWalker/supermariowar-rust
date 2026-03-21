use bevy::prelude::*;

use super::components::{Player, PlayerInput};

/// Keyboard mapping for a single player.
pub struct KeyboardMapping {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub down: KeyCode,
    pub turbo: KeyCode,
    pub use_item: KeyCode,
}

/// Default keyboard mappings for up to 4 players.
pub fn default_mappings() -> [KeyboardMapping; 4] {
    [
        // Player 1: WASD + G/H
        KeyboardMapping {
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            jump: KeyCode::KeyW,
            down: KeyCode::KeyS,
            turbo: KeyCode::KeyG,
            use_item: KeyCode::KeyH,
        },
        // Player 2: Arrow keys + K/L
        KeyboardMapping {
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            jump: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            turbo: KeyCode::KeyK,
            use_item: KeyCode::KeyL,
        },
        // Player 3: numpad
        KeyboardMapping {
            left: KeyCode::Numpad4,
            right: KeyCode::Numpad6,
            jump: KeyCode::Numpad8,
            down: KeyCode::Numpad5,
            turbo: KeyCode::Numpad7,
            use_item: KeyCode::Numpad9,
        },
        // Player 4: IJKL + U/O
        KeyboardMapping {
            left: KeyCode::KeyJ,
            right: KeyCode::KeyL,
            jump: KeyCode::KeyI,
            down: KeyCode::KeyK,
            turbo: KeyCode::KeyU,
            use_item: KeyCode::KeyO,
        },
    ]
}

/// Resource holding the current control mappings.
#[derive(Resource)]
pub struct InputMappings {
    pub mappings: [KeyboardMapping; 4],
}

impl Default for InputMappings {
    fn default() -> Self {
        Self {
            mappings: default_mappings(),
        }
    }
}

/// System that reads keyboard state and populates PlayerInput components.
pub fn read_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mappings: Res<InputMappings>,
    mut query: Query<(&Player, &mut PlayerInput)>,
) {
    for (player, mut input) in query.iter_mut() {
        if player.index >= mappings.mappings.len() {
            continue;
        }
        let map = &mappings.mappings[player.index];
        input.left = keyboard.pressed(map.left);
        input.right = keyboard.pressed(map.right);
        input.jump = keyboard.pressed(map.jump);
        input.jump_pressed = keyboard.just_pressed(map.jump);
        input.down = keyboard.pressed(map.down);
        input.turbo = keyboard.pressed(map.turbo);
        input.use_item = keyboard.just_pressed(map.use_item);
    }
}
