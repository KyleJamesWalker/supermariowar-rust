use bevy::prelude::*;

/// Marker for the map entity that holds the tile grid.
#[derive(Component, Debug)]
pub struct GameMap {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// A single tile in the map grid.
#[derive(Component, Debug, Clone)]
pub struct Tile {
    pub grid_x: u32,
    pub grid_y: u32,
    pub layer: u32,
    pub tile_type: TileType,
    pub tileset_index: i32,
}

/// The type/behavior of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Empty,
    Solid,
    SolidOnTop,        // One-way platform
    Ice,
    Death,             // Kills on contact
    SuperDeath,        // Kills even with star
    NonSolid,          // Background decoration
    Animated,
}

impl Default for TileType {
    fn default() -> Self {
        Self::Empty
    }
}

/// Warp zone definition within a map.
#[derive(Debug, Clone)]
pub struct WarpZone {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub dest_x: u32,
    pub dest_y: u32,
    pub direction: WarpDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Spawn area where players (or items) can appear.
#[derive(Debug, Clone)]
pub struct SpawnArea {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub size: u32,
}

/// Resource holding the loaded map data.
#[derive(Resource, Debug, Default)]
pub struct MapData {
    pub tiles: Vec<Vec<Vec<TileType>>>, // [layer][y][x]
    pub tile_indices: Vec<Vec<Vec<i32>>>,
    pub warps: Vec<WarpZone>,
    pub spawn_areas: Vec<SpawnArea>,
    pub background_id: u32,
}
