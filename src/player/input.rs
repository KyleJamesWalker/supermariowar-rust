use bevy::prelude::*;
use bevy::input::gamepad::Gamepad;

use super::components::{PauseEvent, Player, PlayerInput};
use crate::ai::components::AiControlled;

/// Keyboard mapping for a single player.
pub struct KeyboardMapping {
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub down: KeyCode,
    pub turbo: KeyCode,
    pub use_item: KeyCode,
    pub pause: KeyCode,
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
            pause: KeyCode::Escape,
        },
        // Player 2: Arrow keys + K/L
        KeyboardMapping {
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            jump: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            turbo: KeyCode::KeyK,
            use_item: KeyCode::KeyL,
            pause: KeyCode::Enter,
        },
        // Player 3: numpad
        KeyboardMapping {
            left: KeyCode::Numpad4,
            right: KeyCode::Numpad6,
            jump: KeyCode::Numpad8,
            down: KeyCode::Numpad5,
            turbo: KeyCode::Numpad7,
            use_item: KeyCode::Numpad9,
            pause: KeyCode::NumpadEnter,
        },
        // Player 4: second numpad cluster
        KeyboardMapping {
            left: KeyCode::Numpad1,
            right: KeyCode::Numpad3,
            jump: KeyCode::NumpadAdd,
            down: KeyCode::Numpad2,
            turbo: KeyCode::Numpad0,
            use_item: KeyCode::NumpadDecimal,
            pause: KeyCode::Escape,
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

/// Resource tracking which gamepad entity is assigned to which player slot.
/// When a gamepad connects, assign it to the next available slot.
#[derive(Resource, Default)]
pub struct GamepadAssignments {
    pub assignments: [Option<Entity>; 4],
}

/// System that reads keyboard and gamepad state and populates PlayerInput components.
pub fn read_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mappings: Res<InputMappings>,
    gamepad_assignments: Res<GamepadAssignments>,
    gamepad_query: Query<&Gamepad>,
    mut query: Query<(&Player, &mut PlayerInput), Without<AiControlled>>,
    mut pause_events: EventWriter<PauseEvent>,
) {
    for (player, mut input) in query.iter_mut() {
        if player.index >= mappings.mappings.len() {
            continue;
        }
        let map = &mappings.mappings[player.index];

        // Read keyboard input
        let kb_left = keyboard.pressed(map.left);
        let kb_right = keyboard.pressed(map.right);
        let kb_jump = keyboard.pressed(map.jump);
        let kb_jump_pressed = keyboard.just_pressed(map.jump);
        let kb_down = keyboard.pressed(map.down);
        let kb_turbo = keyboard.pressed(map.turbo);
        let kb_use_item = keyboard.just_pressed(map.use_item);
        let kb_pause = keyboard.just_pressed(map.pause);

        // Read gamepad input if a gamepad is assigned to this player
        let (gp_left, gp_right, gp_jump, gp_jump_pressed, gp_down, gp_turbo, gp_use_item, gp_pause) =
            if let Some(gamepad_entity) = gamepad_assignments.assignments[player.index] {
                if let Ok(gamepad) = gamepad_query.get(gamepad_entity) {
                    (
                        gamepad.pressed(GamepadButton::DPadLeft),
                        gamepad.pressed(GamepadButton::DPadRight),
                        gamepad.pressed(GamepadButton::South),
                        gamepad.just_pressed(GamepadButton::South),
                        gamepad.pressed(GamepadButton::DPadDown),
                        gamepad.pressed(GamepadButton::West),
                        gamepad.just_pressed(GamepadButton::East),
                        gamepad.just_pressed(GamepadButton::Start),
                    )
                } else {
                    (false, false, false, false, false, false, false, false)
                }
            } else {
                (false, false, false, false, false, false, false, false)
            };

        // Combine keyboard and gamepad: either source activates
        input.left = kb_left || gp_left;
        input.right = kb_right || gp_right;
        input.jump = kb_jump || gp_jump;
        input.jump_pressed = kb_jump_pressed || gp_jump_pressed;
        input.down = kb_down || gp_down;
        input.turbo = kb_turbo || gp_turbo;
        input.use_item = kb_use_item || gp_use_item;
        input.pause_pressed = kb_pause || gp_pause;

        // Fire pause event on first frame of press
        if input.pause_pressed {
            pause_events.send(PauseEvent {
                player_index: player.index,
            });
        }
    }
}
