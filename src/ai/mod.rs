pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // AI systems run during gameplay in a fixed order:
        //   perception -> memory -> mode_awareness -> goal_selection -> navigation -> apply_intent
        //
        // They are scheduled to run *before* the player input system so that
        // `PlayerInput` is populated before the movement systems consume it.
        app.add_systems(
            Update,
            (
                systems::ai_perception_update,
                systems::ai_memory_update,
                systems::ai_record_death,
                systems::ai_mode_awareness,
                systems::ai_goal_selection,
                systems::ai_navigation,
                systems::ai_apply_intent,
            )
                .chain()
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::Playing)),
        );
    }
}
