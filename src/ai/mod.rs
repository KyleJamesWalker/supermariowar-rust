pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use crate::core::GameSystemSet;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // AI systems run during gameplay in a fixed order:
        //   perception -> memory -> mode_awareness -> goal_selection -> navigation -> apply_intent
        //
        // They are scheduled in GameSystemSet::Input and ordered *before* the
        // human input reader so that `PlayerInput` written by `ai_apply_intent`
        // is not overwritten by `read_player_input` (which skips AI players).
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
                .in_set(GameSystemSet::Input)
                .before(crate::player::input::read_player_input)
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::Playing)),
        );
    }
}
