use bevy::prelude::*;

use crate::core::constants::*;
use crate::player::components::{Grounded, Velocity};
use super::components::GravityAffected;

/// Applies gravity to all entities that have the GravityAffected component.
pub fn apply_gravity(
    mut query: Query<(&mut Velocity, &GravityAffected)>,
) {
    for (mut vel, gravity) in query.iter_mut() {
        vel.y += GRAVITY * gravity.multiplier;
        if vel.y > MAX_FALL_SPEED {
            vel.y = MAX_FALL_SPEED;
        }
    }
}

/// Detects collision between players and the map tile grid.
/// Sets the Grounded component when a player lands on a solid tile.
pub fn map_collision(
    mut query: Query<(&mut Transform, &mut Velocity, &mut Grounded)>,
) {
    for (mut transform, mut vel, mut grounded) in query.iter_mut() {
        // Floor collision (bottom of screen boundary for now)
        let floor_y = SCREEN_HEIGHT - PLAYER_HEIGHT - PLAYER_HEIGHT_OFFSET;
        if transform.translation.y >= floor_y {
            transform.translation.y = floor_y;
            vel.y = 0.0;
            grounded.0 = true;
        } else {
            grounded.0 = false;
        }

        // Ceiling collision
        if transform.translation.y < 0.0 {
            transform.translation.y = 0.0;
            if vel.y < 0.0 {
                vel.y = 0.0;
            }
        }
    }
}
