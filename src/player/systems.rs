use bevy::prelude::*;

use crate::core::constants::*;
use super::components::*;

/// Applies horizontal movement based on player input.
pub fn player_movement(
    mut query: Query<(&PlayerInput, &mut Velocity, &mut Facing, &mut PlayerAction, &Grounded)>,
) {
    for (input, mut vel, mut facing, mut action, grounded) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }

        let max_speed = if input.turbo { VEL_TURBO_MOVING } else { VEL_MOVING };
        let accel = VEL_MOVING_ADD;
        let friction = if grounded.0 { VEL_MOVING_FRICTION } else { VEL_AIR_FRICTION };

        if input.left {
            *facing = Facing::Left;
            vel.x -= accel;
            if vel.x < -max_speed {
                vel.x = -max_speed;
            }
        } else if input.right {
            *facing = Facing::Right;
            vel.x += accel;
            if vel.x > max_speed {
                vel.x = max_speed;
            }
        } else {
            // Apply friction
            if vel.x > 0.0 {
                vel.x -= friction;
                if vel.x < 0.0 {
                    vel.x = 0.0;
                }
            } else if vel.x < 0.0 {
                vel.x += friction;
                if vel.x > 0.0 {
                    vel.x = 0.0;
                }
            }
        }

        // Update action state for horizontal movement
        if grounded.0 && *action != PlayerAction::Jumping {
            if vel.x.abs() > 0.1 {
                *action = PlayerAction::Running;
            } else {
                *action = PlayerAction::Standing;
            }
        }
    }
}

/// Handles jump initiation.
pub fn player_jump(
    mut query: Query<(&PlayerInput, &mut Velocity, &mut PlayerAction, &mut Grounded)>,
) {
    for (input, mut vel, mut action, mut grounded) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }

        if input.jump_pressed && grounded.0 {
            let jump_vel = if input.turbo { VEL_TURBO_JUMP } else { JUMP_VELOCITY };
            vel.y = jump_vel;
            *action = PlayerAction::Jumping;
            grounded.0 = false;
        }

        // Variable jump height: releasing jump button reduces upward velocity
        if !input.jump && vel.y < -VEL_STOP_JUMP {
            vel.y = -VEL_STOP_JUMP;
        }
    }
}

/// Applies velocity to player transform and wraps around screen edges.
pub fn apply_player_velocity(
    mut query: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    for (vel, mut transform) in query.iter_mut() {
        transform.translation.x += vel.x;
        transform.translation.y += vel.y;

        // Screen wrapping (horizontal)
        if transform.translation.x > SCREEN_WIDTH + PLAYER_WIDTH {
            transform.translation.x = -PLAYER_WIDTH;
        } else if transform.translation.x < -PLAYER_WIDTH {
            transform.translation.x = SCREEN_WIDTH + PLAYER_WIDTH;
        }
    }
}
