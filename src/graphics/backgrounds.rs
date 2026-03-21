use bevy::prelude::*;

use crate::core::constants::*;
use crate::graphics::sprites::{z_order, SpriteSheets, load_background_on_demand};
use crate::map::components::MapData;

// ---------------------------------------------------------------------------
// Background component
// ---------------------------------------------------------------------------

/// Marker component for the background entity.
#[derive(Component, Debug)]
pub struct Background {
    /// Name of the background image file (without path/extension).
    pub name: String,
    /// Whether this background scrolls.
    pub scrolling: bool,
    /// Scroll speed in pixels per second (x, y).
    pub scroll_speed: Vec2,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            name: String::new(),
            scrolling: false,
            scroll_speed: Vec2::ZERO,
        }
    }
}

/// Marker for cleanup: background entities are tagged with this for
/// easy despawning when leaving gameplay.
#[derive(Component, Debug)]
pub struct BackgroundEntity;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the background sprite behind everything when entering gameplay.
/// Reads the map's `background_file` field to determine which image to use.
pub fn spawn_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_data: Res<MapData>,
    mut sprite_sheets: ResMut<SpriteSheets>,
) {
    // Determine background name from map data
    let bg_name = if map_data.background_file.is_empty() {
        "Battle_Black".to_string()
    } else {
        // The map stores a full path like "gfx/packs/Classic/backgrounds/Battle_Black.png"
        // or just the filename. Extract the stem.
        extract_background_name(&map_data.background_file)
    };

    let texture = load_background_on_demand(&asset_server, &mut sprite_sheets, &bg_name);

    // Spawn the background as a full-screen sprite at the lowest z-layer.
    // The original game is 640x480; backgrounds are exactly that size.
    commands.spawn((
        Sprite {
            image: texture,
            custom_size: Some(Vec2::new(SCREEN_WIDTH, SCREEN_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z_order::BACKGROUND),
        Background {
            name: bg_name,
            scrolling: true,
            scroll_speed: Vec2::new(0.5, 0.0),
        },
        BackgroundEntity,
    ));

    info!("Background spawned");
}

/// Cleans up background entities when leaving gameplay.
pub fn cleanup_backgrounds(
    mut commands: Commands,
    query: Query<Entity, With<BackgroundEntity>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Updates scrolling backgrounds (optional parallax effect).
pub fn update_scrolling_backgrounds(
    time: Res<Time>,
    mut query: Query<(&Background, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (bg, mut transform) in query.iter_mut() {
        if !bg.scrolling {
            continue;
        }

        transform.translation.x += bg.scroll_speed.x * dt;
        transform.translation.y += bg.scroll_speed.y * dt;

        // Wrap horizontally
        let half_w = SCREEN_WIDTH / 2.0;
        if transform.translation.x > half_w {
            transform.translation.x -= SCREEN_WIDTH;
        } else if transform.translation.x < -half_w {
            transform.translation.x += SCREEN_WIDTH;
        }

        // Wrap vertically
        let half_h = SCREEN_HEIGHT / 2.0;
        if transform.translation.y > half_h {
            transform.translation.y -= SCREEN_HEIGHT;
        } else if transform.translation.y < -half_h {
            transform.translation.y += SCREEN_HEIGHT;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extracts the background name (file stem) from various path formats.
fn extract_background_name(raw: &str) -> String {
    // Could be "gfx/packs/Classic/backgrounds/Battle_Black.png"
    // or just "Battle_Black.png" or "Battle_Black"
    let filename = raw.rsplit('/').next().unwrap_or(raw);
    let stem = filename.strip_suffix(".png")
        .or_else(|| filename.strip_suffix(".PNG"))
        .unwrap_or(filename);
    stem.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_background_name_full_path() {
        assert_eq!(
            extract_background_name("gfx/packs/Classic/backgrounds/Battle_Black.png"),
            "Battle_Black"
        );
    }

    #[test]
    fn test_extract_background_name_filename_only() {
        assert_eq!(
            extract_background_name("Battle_CityScape.png"),
            "Battle_CityScape"
        );
    }

    #[test]
    fn test_extract_background_name_no_extension() {
        assert_eq!(
            extract_background_name("Battle_Audience"),
            "Battle_Audience"
        );
    }
}
