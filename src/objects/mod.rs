pub mod blocks;
pub mod components;
pub mod enemies;
pub mod hazards;
pub mod powerups;
pub mod projectiles;

use bevy::prelude::*;

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<blocks::OnOffSwitchState>();
    }
}
