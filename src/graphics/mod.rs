pub mod animation;
pub mod backgrounds;
pub mod color_key;
pub mod effects;
pub mod player_sprites;
pub mod sprites;

use bevy::prelude::*;

use crate::core::game_state::GameState;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.init_resource::<sprites::SpriteSheets>()
            .init_resource::<sprites::GameAssets>()
            .init_resource::<effects::ScreenShake>()
            .init_resource::<effects::ScreenFade>()
            .init_resource::<color_key::ProcessedColorKeyImages>()
            .insert_resource(animation::build_default_animation_bank());

        // Startup: load all game assets
        app.add_systems(Startup, sprites::load_game_assets);

        // Gameplay entry: spawn background
        app.add_systems(OnEnter(GameState::Gameplay), backgrounds::spawn_background);

        // Gameplay exit: clean up backgrounds
        app.add_systems(OnExit(GameState::Gameplay), backgrounds::cleanup_backgrounds);

        // Update systems (always active for global effects)
        app.add_systems(
            Update,
            (
                // Magenta color-key → alpha transparency conversion
                color_key::process_color_key_transparency,
                // Core animation tick
                animation::animate_sprites,
                // Screen effects
                effects::apply_screen_shake,
                effects::screen_fade_system,
                // Particle systems
                effects::particle_spawn_system,
                effects::particle_update_system,
                // Spawn effects
                effects::spawn_effect_system,
            ),
        );

        // Update systems (gameplay-specific)
        app.add_systems(
            Update,
            (
                // Player sprite setup (runs once per new player entity)
                player_sprites::setup_player_sprites,
                // Player visual overlays
                player_sprites::spawn_player_overlays,
                // Animation state sync (reads PlayerAction, updates AnimationController)
                animation::animation_state_sync,
                // Sprite facing flip
                animation::sprite_flip_system,
                // Player-specific sprite sync
                player_sprites::sync_player_sprite_action,
                player_sprites::sync_player_facing,
                // Visual effects on players
                player_sprites::apply_team_tint,
                player_sprites::star_power_visual_system,
                player_sprites::manage_spawn_blink,
                player_sprites::manage_death_animation,
                // Animation-driven effects
                animation::blink_system,
                animation::death_animation_system,
                // Player overlay systems
                player_sprites::shield_visual_system,
                player_sprites::attack_attachment_system,
                player_sprites::carried_object_visual_system,
                // Background scrolling
                backgrounds::update_scrolling_backgrounds,
            )
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}
