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

/// Full list of available skin names, ordered by index.
/// Indices 0-3 match DEFAULT_SKIN_NAMES; additional skins are loadable on demand.
pub const ALL_SKIN_NAMES: &[&str] = &[
    "0smw",
    "4matsy_BubbleBobble",
    "4matsy_Darknut",
    "4matsy_DigDug",
    "4matsy_Dinotorch",
    "4matsy_Ducker",
    "4matsy_Fygar",
    "4matsy_Goriya",
    "4matsy_Mappy",
    "4matsy_Moblin",
    "4matsy_Muncher",
    "4matsy_Pacman",
    "4matsy_Pooka",
    "4matsy_Stalfos",
    "4matsy_Vire",
    "Bahamut",
    "BlackMage",
    "BubBob",
    "BusterBeetle",
    "Chocobo",
    "Classic",
    "Flurry",
    "FTG_Monkey",
    "FTG_Octorok",
    "ftg_RedMage",
    "ftg_Train",
    "FTG_Yoshi",
    "Ghost",
    "hazey_Bomberman",
    "hazey_Carbuncle",
    "hazey_ChuChuRocket",
    "hazey_HelloKitty",
    "hazey_JumpnBump",
    "hazey_Lolo",
    "hazey_Megaman",
    "hazey_Pikachu",
    "hazey_RainbowIslands",
    "hazey_Trainer",
    "Kirby",
    "Kooper",
    "Link",
    "link901_Knuckles",
    "link901_Tails",
    "Luigi",
    "Marioman64_WaddleDoo",
    "MontyMole",
    "NinjaKid",
    "Ninji",
    "NMcCoy_nipper",
    "PacMan",
    "peardian_BabyYoshi",
    "peardian_Blaziken",
    "peardian_Fawful",
    "peardian_Gardevoir",
    "peardian_GrimLeecher",
    "peardian_PsychoKamek",
    "peardian_Shroob",
    "Peardian_Zero",
    "Pig",
    "Pyoro",
    "Sgraff_AntiGuy",
    "sgraff_Boo",
    "sgraff_Bow",
    "sgraff_Cobrat",
    "sgraff_Daisy",
    "sgraff_DinoRex",
    "sgraff_Goomboo",
    "sgraff_HammerBrother",
    "sgraff_HammerMario",
    "sgraff_Lakitu",
    "sgraff_Luigi",
    "sgraff_Mario",
    "sgraff_Oldman",
    "sgraff_Peach",
    "sgraff_Shaggy",
    "sgraff_Toad",
    "sgraff_WarioWare",
    "Shyguy",
    "Snifit",
    "Sonic",
    "Spike",
    "SuperWario_Blooper",
    "SuperWario_Bowser",
    "SuperWario_DK",
    "SuperWario_Koopa",
    "Tweeter",
    "wasabi_crab",
    "wasabi_excitebike",
    "wasabi_Fighterfly",
    "wasabi_Goomba",
    "wasabi_IceHockeyFat",
    "wasabi_IceHockeyGoalie",
    "wasabi_IceHockeyMedium",
    "wasabi_IceHockeySkinny",
    "wasabi_pit",
    "wasabi_SlipIce",
    "wasabi_Wario",
    "WhiteMage",
    "Wizrobe",
    "XdragonSB_Samus",
    "Xijar_Beetle",
    "Xijar_Cecil",
    "ym7_Grovyle",
    "ym7_Pikachu",
    "ym7_R.O.B",
    "ym7_Tingle",
    "ym7_Wario",
    "ym7_Yoshi",
    "YoshiMan_Sandbag",
    "YoshiMan_Tanooki",
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
    // Projectile sprite sheets (per-image atlas dimensions from original SMW)
    // -----------------------------------------------------------------------
    for name in PROJECTILE_FILES {
        let path = format!("gfx/packs/Classic/projectiles/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        // Tile sizes and grid dimensions sourced from the original SMW C++
        // constructors (MO_Fireball, MO_Hammer, MO_Boomerang, CO_Shell,
        // CO_Bomb, MO_BulletBill, CO_ThrowBlock, MO_IceBlast, tanooki).
        let (tile_w, tile_h, columns, rows): (u32, u32, u32, u32) = match *name {
            "fireball"   => (18, 18, 4, 8),   // 72x144: width>>2, height>>3
            "hammer"     => (28, 28, 6, 4),   // 168x112: width/6, height>>2
            "boomerang"  => (32, 32, 4, 8),   // 128x256: width/4, height>>3
            "shell"      => (32, 32, 4, 8),   // 128x256: 32x32 frames
            "bomb"       => (28, 38, 5, 5),   // 140x190: iw=28, ih=38
            "bulletbill" => (32, 32, 4, 8),   // 128x256: 32x32 frames
            "throwblock" => (32, 32, 4, 3),   // 128x96: 32x32 frames
            "wandblast"  => (32, 32, 4, 5),   // 128x160: 32x32 frames
            "statue"     => (32, 58, 4, 1),   // 128x58: 32 wide, 58 tall
            _            => (32, 32, 4, 4),   // fallback
        };

        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(tile_w, tile_h),
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
    // Block sprite sheets (per-image atlas dimensions from original SMW)
    // -----------------------------------------------------------------------
    for name in BLOCK_FILES {
        let path = format!("gfx/packs/Classic/blocks/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        // All blocks use 32x32 (TILESIZE) frames. Grid dimensions vary
        // based on the number of animation frames (columns) and block
        // type variants (rows) in each sheet.
        let (columns, rows): (u32, u32) = match *name {
            "breakableblock"       => (4, 1),   // 128x32
            "powerupblock"         => (4, 2),   // 128x64
            "noteblock"            => (4, 3),   // 128x96
            "switchblock"          => (4, 4),   // 128x128
            "flipblock"            => (4, 1),   // 128x32
            "bounceblock"          => (1, 1),   // 32x32
            "donutblock"           => (1, 1),   // 32x32
            "throwblock"           => (4, 3),   // 128x96
            "viewblock"            => (1, 2),   // 32x64
            "weaponbreakableblock" => (10, 1),  // 320x32
            _                      => (4, 1),   // fallback
        };

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
    // Hazard / enemy sprites (per-image atlas dimensions from original SMW)
    // -----------------------------------------------------------------------
    for name in HAZARD_FILES {
        let path = format!("gfx/packs/Classic/hazards/{}.png", name);
        let texture: Handle<Image> = asset_server.load(&path);

        // Hazard sheets use varying tile sizes and layouts. The flame and
        // piranha plant sprites have non-uniform sub-regions in the original
        // game, but we use the dominant tile size for the Bevy atlas grid.
        let (tile_w, tile_h, columns, rows): (u32, u32, u32, u32) = match *name {
            "fireball"       => (18, 18, 4, 2),   // 72x36: 18x18 frames
            "flame"          => (32, 32, 8, 7),    // 256x224: 32x32 grid
            "pirhanaplant"   => (32, 32, 11, 8),   // 352x256: 32x32 grid
            "rotodisc"       => (32, 32, 21, 1),   // 672x32: 32x32 frames
            "bulletbill"     => (32, 32, 1, 2),    // 32x64: 32x32 frames
            "bulletbilldead" => (32, 32, 2, 1),    // 64x32: 32x32 frames
            _                => (32, 32, 4, 1),    // fallback
        };

        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(tile_w, tile_h),
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
