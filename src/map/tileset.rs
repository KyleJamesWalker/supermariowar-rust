use bevy::prelude::*;

use crate::core::constants::*;

// ---------------------------------------------------------------------------
// Tileset definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TilesetDefinition {
    pub name: String,
    /// Path relative to the Bevy asset root (the `data/` directory is
    /// typically set up as the asset root, so paths look like
    /// `gfx/packs/Classic/tilesets/Classic/large.png`).
    pub texture_path: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
}

impl TilesetDefinition {
    pub fn new(name: &str, texture_path: &str) -> Self {
        Self {
            name: name.to_string(),
            texture_path: texture_path.to_string(),
            tile_width: TILE_SIZE_U32,
            tile_height: TILE_SIZE_U32,
            columns: TILESET_WIDTH,
            rows: TILESET_HEIGHT,
        }
    }
}

// ---------------------------------------------------------------------------
// Loaded tileset (handle + atlas)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoadedTileset {
    pub definition: TilesetDefinition,
    pub texture: Handle<Image>,
    pub atlas_layout: Handle<TextureAtlasLayout>,
}

// ---------------------------------------------------------------------------
// TilesetManager resource
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TilesetManager {
    pub tilesets: Vec<LoadedTileset>,
    /// Quick lookup: name → index into `tilesets`.
    pub name_to_index: Vec<(String, usize)>,
    /// Index of the "Classic" tileset (most maps use this).
    pub classic_index: Option<usize>,
}

impl TilesetManager {
    pub fn index_from_name(&self, name: &str) -> Option<usize> {
        self.name_to_index.iter().find(|(n, _)| n == name).map(|(_, i)| *i)
    }

    pub fn get(&self, index: usize) -> Option<&LoadedTileset> {
        self.tilesets.get(index)
    }
}

// ---------------------------------------------------------------------------
// Standard tileset paths
// ---------------------------------------------------------------------------

/// (name, relative asset path to the `large.png` image)
const STANDARD_TILESETS: &[(&str, &str)] = &[
    ("Classic", "gfx/packs/Classic/tilesets/Classic/large.png"),
    ("SMB1", "gfx/packs/Classic/tilesets/SMB1/large.png"),
    ("SMB2", "gfx/packs/Classic/tilesets/SMB2/large.png"),
    ("SMB3", "gfx/packs/Classic/tilesets/SMB3/large.png"),
    ("SuperMarioWorld", "gfx/packs/Classic/tilesets/SuperMarioWorld/large.png"),
];

// ---------------------------------------------------------------------------
// System: load all standard tilesets on startup
// ---------------------------------------------------------------------------

/// Startup system that registers the standard tilesets, loads their images,
/// and builds `TextureAtlasLayout`s so tiles can be rendered with Bevy sprites.
pub fn load_tilesets(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut manager: ResMut<TilesetManager>,
) {
    for &(name, path) in STANDARD_TILESETS {
        let def = TilesetDefinition::new(name, path);

        let texture: Handle<Image> = asset_server.load(path);

        // Build a grid layout: each tile is 32×32 inside a 1024×960 sheet
        // (32 columns × 30 rows).
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(def.tile_width, def.tile_height),
            def.columns as u32,
            def.rows as u32,
            None,
            None,
        );
        let layout_handle = layouts.add(layout);

        let idx = manager.tilesets.len();
        manager.tilesets.push(LoadedTileset {
            definition: def,
            texture,
            atlas_layout: layout_handle,
        });
        manager.name_to_index.push((name.to_string(), idx));

        if name == "Classic" {
            manager.classic_index = Some(idx);
        }
    }

    info!("Loaded {} tilesets", manager.tilesets.len());
}

/// Resolves a tileset name from the map file into a `LoadedTileset` index.
/// Falls back to Classic if the name is unknown.
pub fn resolve_tileset_index(manager: &TilesetManager, name: &str) -> usize {
    manager
        .index_from_name(name)
        .unwrap_or_else(|| manager.classic_index.unwrap_or(0))
}
