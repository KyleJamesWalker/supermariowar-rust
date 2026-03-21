pub mod music;
pub mod sfx;

use bevy::prelude::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<music::MusicPlayer>()
            .init_resource::<sfx::SfxLibrary>();
    }
}
