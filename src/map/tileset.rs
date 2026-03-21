use bevy::prelude::*;

use crate::core::constants::*;

/// Identifies a tileset and its associated texture.
#[derive(Debug, Clone)]
pub struct TilesetDefinition {
    pub name: String,
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

/// Resource managing all loaded tilesets.
#[derive(Resource, Default)]
pub struct TilesetManager {
    pub tilesets: Vec<TilesetDefinition>,
    pub classic_tileset_index: Option<usize>,
}

impl TilesetManager {
    pub fn add_tileset(&mut self, def: TilesetDefinition) -> usize {
        let idx = self.tilesets.len();
        self.tilesets.push(def);
        idx
    }

    pub fn get_tileset(&self, index: usize) -> Option<&TilesetDefinition> {
        self.tilesets.get(index)
    }
}
