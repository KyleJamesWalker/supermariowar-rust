pub mod components;
pub mod loader;
pub mod tileset;

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<components::MapData>()
            .init_resource::<tileset::TilesetManager>();
    }
}
