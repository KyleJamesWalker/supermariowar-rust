pub mod constants;
pub mod game_state;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<game_state::GameState>()
            .add_sub_state::<game_state::GameplayState>()
            .init_resource::<game_state::MatchResult>();
    }
}
