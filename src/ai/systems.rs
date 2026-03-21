use bevy::prelude::*;

use crate::player::components::{Player, PlayerInput};
use super::components::*;

/// Updates AI perception of the world (finds nearest threats/targets).
pub fn ai_perception_update(
    mut ai_query: Query<(&Transform, &mut AiPerception), With<AiControlled>>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<AiControlled>)>,
) {
    for (ai_transform, mut perception) in ai_query.iter_mut() {
        let mut closest_dist = f32::MAX;
        let mut closest_entity = None;

        for (entity, player_transform) in player_query.iter() {
            let dist = ai_transform
                .translation
                .truncate()
                .distance(player_transform.translation.truncate());
            if dist < closest_dist {
                closest_dist = dist;
                closest_entity = Some(entity);
            }
        }

        perception.nearest_player = closest_entity;
        perception.nearest_player_distance = closest_dist;
    }
}

/// AI decision-making: determines intent based on perception and difficulty.
pub fn ai_decision(
    time: Res<Time>,
    mut query: Query<(&mut AiControlled, &mut AiIntent, &AiPerception, &Transform)>,
) {
    let dt = time.delta_secs();

    for (mut ai, mut intent, perception, transform) in query.iter_mut() {
        ai.decision_timer += dt;
        if ai.decision_timer < ai.decision_interval {
            continue;
        }
        ai.decision_timer = 0.0;

        // Reset intent
        *intent = AiIntent::default();

        // Simple behavior: move toward nearest player
        if let Some(_target) = perception.nearest_player {
            // Target position would come from querying the target entity
            // For now, use a simple heuristic
            if perception.nearest_player_distance > 100.0 {
                // Move toward target (simplified)
                intent.move_right = rand::random::<bool>();
                intent.move_left = !intent.move_right;
            }

            // Jump occasionally
            if rand::random::<f32>() < 0.3 {
                intent.jump = true;
            }
        }
    }
}

/// Converts AI intent into PlayerInput so the normal movement systems handle it.
pub fn ai_apply_intent(
    mut query: Query<(&AiIntent, &mut PlayerInput), With<AiControlled>>,
) {
    for (intent, mut input) in query.iter_mut() {
        input.left = intent.move_left;
        input.right = intent.move_right;
        input.jump = intent.jump;
        input.jump_pressed = intent.jump;
        input.use_item = intent.use_item;
    }
}
