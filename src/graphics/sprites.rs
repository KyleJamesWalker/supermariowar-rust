use bevy::prelude::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Z-layer constants (used throughout the graphics module)
// ---------------------------------------------------------------------------

/// Z-ordering constants for rendering layers.
pub mod z_order {
    pub const BACKGROUND: f32 = -100.0;
    pub const TILES_BASE: f32 = 0.0;
    pub const TILES_MAX: f32 = 3.0;
    pub const OBJECTS: f32 = 5.0;
    pub const PLAYERS: f32 = 10.0;
    pub const PARTICLES: f32 = 15.0;
    pub const PROJECTILES: f32 = 11.0;
    pub const EFFECTS: f32 = 20.0;
    pub const HUD: f32 = 100.0;
    pub const FADE_OVERLAY: f32 = 200.0;
}

// ---------------------------------------------------------------------------
// Skin sprite sheet layout
// ---------------------------------------------------------------------------

/// SMW player skin sprite sheets are 192x32 PNG strips (6 frames of 32x32).
/// Frame layout: 0=standing, 1=walk1, 2=walk2, 3=walk3, 4=jump, 5=dead/spin
pub const SKIN_FRAME_WIDTH: u32 = 32;
pub const SKIN_FRAME_HEIGHT: u32 = 32;
pub const SKIN_FRAMES_PER_STRIP: u32 = 6;
pub const SKIN_STRIP_WIDTH: u32 = SKIN_FRAME_WIDTH * SKIN_FRAMES_PER_STRIP; // 192

// ---------------------------------------------------------------------------
// SpriteSheets resource
// ---------------------------------------------------------------------------

/// Resource managing sprite sheet handles for the game.
#[derive(Resource, Default)]
pub struct SpriteSheets {
    /// Player skin textures, keyed by skin filename (sans extension).
    pub player_skins: Vec<LoadedSkinSheet>,
    /// Tileset sheet (managed by TilesetManager, kept here for cross-reference).
    pub tileset_sheet: Option<Handle<Image>>,
    /// Powerup sprite sheet (large.png: 896x32, 28 frames of 32x32).
    pub powerup_sheet: Option<LoadedSpriteSheet>,
    /// Individual powerup sprites keyed by filename stem.
    pub powerup_images: HashMap<String, Handle<Image>>,
    /// Projectile sprites keyed by filename stem.
    pub projectile_images: HashMap<String, LoadedSpriteSheet>,
    /// Block sprites keyed by filename stem.
    pub block_images: HashMap<String, LoadedSpriteSheet>,
    /// Enemy/hazard sprites keyed by filename stem.
    pub hazard_images: HashMap<String, LoadedSpriteSheet>,
    /// Menu graphics keyed by filename stem.
    pub menu_images: HashMap<String, Handle<Image>>,
    /// Background images keyed by filename stem.
    pub background_images: HashMap<String, Handle<Image>>,
}

// ---------------------------------------------------------------------------
// Loaded sheet types
// ---------------------------------------------------------------------------

/// A loaded player skin: texture + atlas layout for the 6-frame strip.
#[derive(Debug, Clone)]
pub struct LoadedSkinSheet {
    pub name: String,
    pub texture: Handle<Image>,
    pub atlas_layout: Handle<TextureAtlasLayout>,
}

/// A loaded sprite sheet with a grid-based atlas.
#[derive(Debug, Clone)]
pub struct LoadedSpriteSheet {
    pub texture: Handle<Image>,
    pub atlas_layout: Handle<TextureAtlasLayout>,
    pub columns: u32,
    pub rows: u32,
}

// ---------------------------------------------------------------------------
// GameAssets resource
// ---------------------------------------------------------------------------

/// Centralized resource holding all loaded game assets.
#[derive(Resource, Default)]
pub struct GameAssets {
    /// Whether initial asset loading has been kicked off.
    pub loading_started: bool,
    /// Whether all assets are assumed loaded (we rely on async loading).
    pub loading_complete: bool,
}

// ---------------------------------------------------------------------------
// SpriteRegion component
// ---------------------------------------------------------------------------

/// Component for sub-rectangle rendering from a sprite sheet.
/// Used when an entity needs to display a specific region of a larger texture.
#[derive(Component, Debug, Clone)]
pub struct SpriteRegion {
    /// Which atlas index to render.
    pub atlas_index: usize,
}

// ---------------------------------------------------------------------------
// Asset loading system
// ---------------------------------------------------------------------------

/// Default skins to pre-load (first 4 for the 4 player slots).
const DEFAULT_SKIN_NAMES: &[&str] = &[
    "0smw",
    "4matsy_BubbleBobble",
    "4matsy_Darknut",
    "4matsy_DigDug",
];

/// Powerup image files within `gfx/packs/Classic/powerups/`.
const POWERUP_FILES: &[&str] = &[
    "fireflower",
    "hammerpowerup",
    "boomerangpowerup",
    "featherpowerup",
    "leafpowerup",
    "bombpowerup",
    "starpowerup",
    "poisonpowerup",
    "mysterymushroom",
    "1uppowerup",
    "2uppowerup",
    "3uppowerup",
    "5uppowerup",
    "clockpowerup",
    "bobombpowerup",
    "powpowerup",
    "bulletbillpowerup",
    "modpowerup",
    "heartpowerup",
    "icewandpowerup",
    "jailkeypowerup",
    "kuriboshoe",
    "pwings",
    "spring",
    "spike",
    "throwbox",
    "extratimepowerup",
    "podobopowerup",
    "tanooki",
];

/// Projectile image files within `gfx/packs/Classic/projectiles/`.
const PROJECTILE_FILES: &[&str] = &[
    "fireball",
    "hammer",
    "boomerang",
    "shell",
    "bomb",
    "bulletbill",
    "throwblock",
    "wandblast",
    "statue",
];

/// Block image files within `gfx/packs/Classic/blocks/`.
const BLOCK_FILES: &[&str] = &[
    "breakableblock",
    "powerupblock",
    "noteblock",
    "switchblock",
    "flipblock",
    "bounceblock",
    "donutblock",
    "throwblock",
    "viewblock",
    "weaponbreakableblock",
];

/// Hazard image files within `gfx/packs/Classic/hazards/`.
const HAZARD_FILES: &[&str] = &[
    "fireball",
    "flame",
    "pirhanaplant",
    "rotodisc",
    "bulletbill",
    "bulletbilldead",
];

/// Menu image files within `gfx/packs/Classic/menu/`.
const MENU_FILES: &[&str] = &[
    "menu_background",
    "menu_smw",
    "menu_shade",
    "menu_font_large",
    "menu_font_small",
    "scoreboard",
    "score_coins",
    "score_hearts",
    "score_cards",
    "game_countdown_numbers",
    "menu_player_select",
    "player_select_background",
    "player_select_ready",
    "menu_match_select",
    "menu_mode_large",
    "menu_mode_small",
    "splash_credits",
    "menu_version",
    "tournament_background",
    "menu_selectfield",
    "menu_slider_bar",
    "dialog",
    "dialog_button",
    "menu_announcement_icons",
    "menu_vertical_arrows",
];

/// Startup system that loads all game assets.
pub fn load_game_assets(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut sprite_sheets: ResMut<SpriteSheets>,
    mut game_assets: ResMut<GameAssets>,
) {
    if game_assets.loading_started {
        return;
    }
    game_assets.loading_started = true;

    // -----------------------------------------------------------------------
    // Player skins
    // -----------------------------------------------------------------------
    for skin_name in DEFAULT_SKIN_NAMES {
        let path = format!("gfx/skins/{}.png", skin_name);
        let texture: Handle<Image> = asset_server.load(&path);

        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(SKIN_FRAME_WIDTH, SKIN_FRAME_HEIGHT),
            SKIN_FRAMES_PER_STRIP,
            1,
            None,
            None,
        );
        let atlas_layout = layouts.add(layout);

        sprite_sheets.player_skins.push(LoadedSkinSheet {
            name: skin_name.to_string(),
            texture,
            atlas_layout,
        });
    }

    // -----------------------------------------------------------------------
    // Powerup large sprite sheet (896x32 = 28 frames of 32x32)
    // -----------------------------------------------------------------------
    {
        let path = "gfx/packs/Classic/powerups/large.png";
        let texture: Handle<Image> = asset_server.load(path);
        let columns = 28u32;
        let rows = 1u32;
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(32, 32),
            columns,
            rows,
            None,
            None,
        );
        let atlas_layout = layouts.add(layout);
        sprite_sheets.powerup_sheet = Some(LoadedSpriteSheet {
            texture,
            atlas_layout,
            columns,
            rows,
        });
    }

    // -----------------------------------------------------------------------
    // Individual powerup images
    // -----------------------------------------------------------------------
    for name in POWERUP_FILES {
        let path = format!("gfx/packs/Classic/powerups/{}.png", name);
        let handle: Handle<Image> = asset_server.load(&path);
        sprite_sheets.powerup_images.insert(name.to_string(), handle);
    }

    // -----------------------------------------------------------------------
    // Projectile sprite sheets
    // -----------------------------------------------------------------------
    for name in PROJECTILE_FILES {
        let path = format!("gfx/packs/Classic/projectiles/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        // Most projectile sheets are small grids; use a reasonable default
        // that can be adjusted per-projectile. Common sizes:
        //   fireball: 72x144 (could be 4 cols x 9 rows at ~18x16, or various)
        //   hammer: variable
        // We'll use 32x32 grid as a safe default; actual frame selection
        // happens via animation bank configuration.
        let columns = 4u32;
        let rows = 4u32;
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(16, 16),
            columns,
            rows,
            None,
            None,
        );
        let atlas_layout = layouts.add(layout);

        sprite_sheets.projectile_images.insert(
            name.to_string(),
            LoadedSpriteSheet {
                texture,
                atlas_layout,
                columns,
                rows,
            },
        );
    }

    // -----------------------------------------------------------------------
    // Block sprite sheets
    // -----------------------------------------------------------------------
    for name in BLOCK_FILES {
        let path = format!("gfx/packs/Classic/blocks/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        // Block sheets are typically 128x32 (4 frames of 32x32)
        let columns = 4u32;
        let rows = 1u32;
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(32, 32),
            columns,
            rows,
            None,
            None,
        );
        let atlas_layout = layouts.add(layout);

        sprite_sheets.block_images.insert(
            name.to_string(),
            LoadedSpriteSheet {
                texture,
                atlas_layout,
                columns,
                rows,
            },
        );
    }

    // -----------------------------------------------------------------------
    // Hazard / enemy sprites
    // -----------------------------------------------------------------------
    for name in HAZARD_FILES {
        let path = format!("gfx/packs/Classic/hazards/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        let columns = 4u32;
        let rows = 1u32;
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(32, 32),
            columns,
            rows,
            None,
            None,
        );
        let atlas_layout = layouts.add(layout);

        sprite_sheets.hazard_images.insert(
            name.to_string(),
            LoadedSpriteSheet {
                texture,
                atlas_layout,
                columns,
                rows,
            },
        );
    }

    // -----------------------------------------------------------------------
    // Menu images (loaded as full images, not atlases)
    // -----------------------------------------------------------------------
    for name in MENU_FILES {
        let path = format!("gfx/packs/Classic/menu/{}.png", name);
        let handle: Handle<Image> = asset_server.load(&path);
        sprite_sheets.menu_images.insert(name.to_string(), handle);
    }

    // -----------------------------------------------------------------------
    // Background images
    // -----------------------------------------------------------------------
    // We pre-load a selection of commonly used backgrounds.
    // The full set is loaded on demand via spawn_background.
    let common_backgrounds = [
        "Battle_Black",
        "Battle_CityScape",
        "Battle_Audience",
    ];
    for name in &common_backgrounds {
        let path = format!("gfx/packs/Classic/backgrounds/{}.png", name);
        let handle: Handle<Image> = asset_server.load(&path);
        sprite_sheets.background_images.insert(name.to_string(), handle);
    }

    game_assets.loading_complete = true;
    info!(
        "Game assets loading initiated: {} skins, {} powerup images, {} projectile sheets, {} block sheets",
        sprite_sheets.player_skins.len(),
        sprite_sheets.powerup_images.len(),
        sprite_sheets.projectile_images.len(),
        sprite_sheets.block_images.len(),
    );
}

/// Load a background image on demand (called by the backgrounds module).
pub fn load_background_on_demand(
    asset_server: &AssetServer,
    sprite_sheets: &mut SpriteSheets,
    name: &str,
) -> Handle<Image> {
    if let Some(handle) = sprite_sheets.background_images.get(name) {
        return handle.clone();
    }

    let path = format!("gfx/packs/Classic/backgrounds/{}.png", name);
    let handle: Handle<Image> = asset_server.load(&path);
    sprite_sheets.background_images.insert(name.to_string(), handle.clone());
    handle
}

/// Load a player skin on demand, returning the index into `player_skins`.
pub fn load_skin_on_demand(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_sheets: &mut SpriteSheets,
    skin_name: &str,
) -> usize {
    // Check if already loaded
    if let Some(idx) = sprite_sheets
        .player_skins
        .iter()
        .position(|s| s.name == skin_name)
    {
        return idx;
    }

    let path = format!("gfx/skins/{}.png", skin_name);
    let texture: Handle<Image> = asset_server.load(&path);
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(SKIN_FRAME_WIDTH, SKIN_FRAME_HEIGHT),
        SKIN_FRAMES_PER_STRIP,
        1,
        None,
        None,
    );
    let atlas_layout = layouts.add(layout);

    let idx = sprite_sheets.player_skins.len();
    sprite_sheets.player_skins.push(LoadedSkinSheet {
        name: skin_name.to_string(),
        texture,
        atlas_layout,
    });
    idx
}
