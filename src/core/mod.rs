pub mod constants;
pub mod game_state;

use bevy::prelude::*;

/// System sets for ordering gameplay systems across plugins.
/// Configured as a chain: Input → Movement → Physics → Combat → DeathRespawn → Scoring
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystemSet {
    /// Read keyboard/gamepad input
    Input,
    /// Player movement, jumping (modifies velocity from input)
    Movement,
    /// Gravity, map collision, entity collision
    Physics,
    /// Stomp detection, attack systems, powerup use
    Combat,
    /// Death events, respawn, warp pipe animations
    DeathRespawn,
    /// Score/stat updates
    Scoring,
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<game_state::GameState>()
            .add_sub_state::<game_state::GameplayState>()
            .init_resource::<game_state::MatchResult>()
            // Configure system set ordering: all gameplay systems follow this chain
            .configure_sets(
                Update,
                (
                    GameSystemSet::Input,
                    GameSystemSet::Movement,
                    GameSystemSet::Physics,
                    GameSystemSet::Combat,
                    GameSystemSet::DeathRespawn,
                    GameSystemSet::Scoring,
                )
                    .chain(),
            );
    }
}
