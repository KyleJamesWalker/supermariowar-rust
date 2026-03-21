use bevy::prelude::*;
use std::io::{self, Read, BufReader};
use std::fs::File;
use std::path::Path;

use crate::core::constants::*;
use super::components::*;

/// Version header bytes in SMW .map files.
const MAP_VERSION_1800: [u8; 4] = [0x00, 0x18, 0x00, 0x00];

/// Loads a binary .map file from the original SMW format.
///
/// The SMW map format stores tile data as a grid of tile indices per layer,
/// followed by warp definitions, spawn areas, and other metadata.
pub fn load_map_from_file(path: &Path) -> io::Result<MapData> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Read version header (4 bytes)
    let mut version = [0u8; 4];
    reader.read_exact(&mut version)?;

    info!("Loading map: {:?} (version: {:?})", path, version);

    // Initialize empty map data
    let mut map_data = MapData::default();
    map_data.tiles = vec![
        vec![vec![TileType::Empty; MAP_WIDTH as usize]; MAP_HEIGHT as usize];
        MAP_LAYERS as usize
    ];
    map_data.tile_indices = vec![
        vec![vec![0i32; MAP_WIDTH as usize]; MAP_HEIGHT as usize];
        MAP_LAYERS as usize
    ];

    // TODO: Parse the actual binary format based on version.
    // The full parser needs to handle versions 1500-1800+.
    // For now return empty map data so the game can start.

    Ok(map_data)
}

/// Converts a raw tile type integer from the map file to our TileType enum.
pub fn tile_type_from_raw(raw: i32) -> TileType {
    match raw {
        0 => TileType::Empty,
        1 => TileType::Solid,
        2 => TileType::SolidOnTop,
        3 => TileType::Ice,
        4 => TileType::Death,
        5 => TileType::SuperDeath,
        6 => TileType::NonSolid,
        _ => TileType::Empty,
    }
}
