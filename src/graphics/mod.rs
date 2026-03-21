pub mod animation;
pub mod effects;
pub mod sprites;

use bevy::prelude::*;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<sprites::SpriteSheets>()
            .init_resource::<effects::ScreenShake>()
            .init_resource::<effects::ScreenFade>()
            .add_systems(
                Update,
                (
                    animation::animate_sprites,
                    effects::apply_screen_shake,
                ),
            );
    }
}
