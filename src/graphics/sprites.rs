use bevy::prelude::*;

/// Resource managing sprite sheet handles for the game.
#[derive(Resource, Default)]
pub struct SpriteSheets {
    pub player_sheets: Vec<Handle<Image>>,
    pub tileset_sheet: Option<Handle<Image>>,
    pub powerup_sheet: Option<Handle<Image>>,
    pub enemy_sheet: Option<Handle<Image>>,
    pub projectile_sheet: Option<Handle<Image>>,
    pub menu_sheet: Option<Handle<Image>>,
}

/// Component linking an entity to a specific sprite sheet region.
#[derive(Component, Debug)]
pub struct SpriteIndex {
    pub sheet: usize,
    pub index: usize,
}

/// Loads player sprite sheets from the data directory.
pub fn load_player_sprites(
    mut sprite_sheets: ResMut<SpriteSheets>,
    asset_server: Res<AssetServer>,
) {
    // Player skins are in data/gfx/skins/
    // For now, load a placeholder; actual skin loading will match the original format
    for i in 0..4 {
        let path = format!("gfx/skins/skin{}.png", i);
        let handle = asset_server.load(path);
        sprite_sheets.player_sheets.push(handle);
    }
}
