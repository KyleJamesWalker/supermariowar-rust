use bevy::prelude::*;
use std::io::{self, Read, Cursor};
use std::fs;
use std::path::Path;

use crate::core::constants::*;
use super::components::*;

// ---------------------------------------------------------------------------
// Helpers for reading little-endian primitives
// ---------------------------------------------------------------------------

fn read_i32_le(r: &mut Cursor<&[u8]>) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_f32_le(r: &mut Cursor<&[u8]>) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_i8_val(r: &mut Cursor<&[u8]>) -> io::Result<i8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

fn read_bool(r: &mut Cursor<&[u8]>) -> io::Result<bool> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

/// Reads a length-prefixed string (4-byte i32 length, then that many bytes).
fn read_string_long(r: &mut Cursor<&[u8]>) -> io::Result<String> {
    let len = read_i32_le(r)? as usize;
    if len == 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    // Trim trailing NUL bytes
    while buf.last() == Some(&0) {
        buf.pop();
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const W: usize = MAP_WIDTH as usize;
const H: usize = MAP_HEIGHT as usize;
const LAYERS: usize = MAP_LAYERS as usize;
const NUM_AUTO_FILTERS: usize = 12;
const TILESET_NONE: i8 = -2;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads a binary .map file.  On any parse error the function returns as much
/// data as was successfully read rather than propagating the error.
pub fn load_map_from_file(path: &Path) -> MapData {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            warn!("Could not read map file {:?}: {}", path, e);
            return generate_default_map();
        }
    };

    let mut r = Cursor::new(bytes.as_slice());

    // --- Version header (4 × i32) ---
    let major = read_i32_le(&mut r).unwrap_or(0);
    let minor = read_i32_le(&mut r).unwrap_or(0);
    let patch = read_i32_le(&mut r).unwrap_or(0);
    let build = read_i32_le(&mut r).unwrap_or(0);

    info!(
        "Loading map {:?}  version {}.{}.{}.{}",
        path, major, minor, patch, build
    );

    // We only implement the v1.8.x reader path since that is the current
    // format used by virtually all distributed maps.  Older formats are
    // rare and would require tileset-conversion tables.
    if major == 1 && minor >= 8 {
        match parse_v18(&mut r, patch) {
            Ok(md) => md,
            Err(e) => {
                warn!("Map parse error (v1.8): {} – falling back to default", e);
                generate_default_map()
            }
        }
    } else {
        warn!(
            "Unsupported map version {}.{}.{}.{} – generating default map",
            major, minor, patch, build
        );
        generate_default_map()
    }
}

// ---------------------------------------------------------------------------
// v1.8 loader
// ---------------------------------------------------------------------------

fn parse_v18(r: &mut Cursor<&[u8]>, patch: i32) -> io::Result<MapData> {
    let mut md = MapData::default();

    // 1. Auto-filter values (NUM_AUTO_FILTERS + 1 i32s)
    let num_af = NUM_AUTO_FILTERS + 1; // original writes 13 values
    for _ in 0..num_af {
        let _ = read_i32_le(r)?;
    }

    // 2. Tileset translation table
    let num_tilesets = read_i32_le(r)? as usize;
    let mut tileset_names: Vec<(i16, String)> = Vec::with_capacity(num_tilesets);
    let mut max_id: i16 = -1;
    for _ in 0..num_tilesets {
        let id = read_i32_le(r)? as i16;
        let name = read_string_long(r)?;
        if id > max_id {
            max_id = id;
        }
        tileset_names.push((id, name));
    }
    md.tileset_names = tileset_names;

    // 3. Tile data  –  [col 0..W][row 0..H][layer 0..4] each: 3 bytes
    //    Plus per-cell object block (1 byte type + 1 byte hidden).
    // Allocate
    md.tiles = vec![vec![vec![TilesetTileRef::default(); LAYERS]; H]; W];
    md.object_data = vec![vec![ObjectBlock::default(); H]; W];

    for row in 0..H {
        for col in 0..W {
            for layer in 0..LAYERS {
                let id = read_i8_val(r)?;
                let c = read_i8_val(r)?;
                let rv = read_i8_val(r)?;
                md.tiles[col][row][layer] = TilesetTileRef { id, col: c, row: rv };
            }
            let block_type = read_i8_val(r)?;
            let hidden = read_bool(r)?;
            md.object_data[col][row] = ObjectBlock {
                block_type,
                hidden,
                settings: [0; 8],
            };
        }
    }

    // 4. Background file
    md.background_file = read_string_long(r)?;

    // 5. Switch states (4 × i32)
    for i in 0..4 {
        md.switches[i] = read_i32_le(r)?;
    }

    // 6. Moving platforms
    let num_platforms = read_i32_le(r)? as usize;
    for _ in 0..num_platforms {
        if let Ok(p) = read_platform(r, patch) {
            md.platforms.push(p);
        }
    }

    // 7. Map items
    let num_items = read_i32_le(r)? as usize;
    for _ in 0..num_items {
        let item_type = read_i32_le(r)?;
        let ix = read_i32_le(r)?;
        let iy = read_i32_le(r)?;
        md.map_items.push(MapItemSpawn { item_type, tile_x: ix, tile_y: iy });
    }

    // 8. Map hazards
    let num_hazards = read_i32_le(r)? as usize;
    for _ in 0..num_hazards {
        let hazard_type = read_i32_le(r)?;
        let ix = read_i32_le(r)?;
        let iy = read_i32_le(r)?;
        let mut iparam = [0i32; NUM_MAP_HAZARD_PARAMS];
        for p in iparam.iter_mut() {
            *p = read_i32_le(r)?;
        }
        let mut dparam = [0.0f32; NUM_MAP_HAZARD_PARAMS];
        for p in dparam.iter_mut() {
            *p = read_f32_le(r)?;
        }
        md.map_hazards.push(MapHazard { hazard_type, tile_x: ix, tile_y: iy, iparam, dparam });
    }

    // 9. Eye candy  (v1.8.0x writes 1 layer, v1.8.2+ writes 3)
    if patch >= 2 {
        md.eyecandy[0] = read_i32_le(r)?;
        md.eyecandy[1] = read_i32_le(r)?;
        md.eyecandy[2] = read_i32_le(r)?;
    } else {
        md.eyecandy[2] = read_i32_le(r)?;
    }

    // 10. Music category
    md.music_category = read_i32_le(r)?;

    // 11. Per-tile collision + warp + nospawn data
    md.collision_map = vec![vec![CollisionType::NonSolid; H]; W];
    md.warp_data = vec![vec![PerTileWarp::default(); H]; W];
    md.no_spawn = vec![vec![vec![false; H]; W]; NUM_SPAWN_AREA_TYPES];

    for row in 0..H {
        for col in 0..W {
            md.collision_map[col][row] = CollisionType::from_raw(read_i32_le(r)?);

            let dir = read_i32_le(r)?;
            let conn = read_i32_le(r)?;
            let id = read_i32_le(r)?;
            md.warp_data[col][row] = PerTileWarp { direction: dir, connection: conn, id };

            for t in 0..NUM_SPAWN_AREA_TYPES {
                md.no_spawn[t][col][row] = read_bool(r)?;
            }
        }
    }

    // 12. Switchable block state data
    let num_switch_blocks = read_i32_le(r)? as usize;
    for _ in 0..num_switch_blocks {
        let sc = read_i8_val(r)? as usize;
        let sr = read_i8_val(r)? as usize;
        let setting = read_i8_val(r)?;
        if sc < W && sr < H {
            md.object_data[sc][sr].settings[0] = setting;
        }
    }

    // 13. Warp exits
    let num_warp_exits = read_i32_le(r)? as usize;
    for _ in 0..num_warp_exits.min(MAX_WARPS) {
        let direction = WarpDirection::from_raw(read_i32_le(r)?);
        let connection = read_i32_le(r)?;
        let id = read_i32_le(r)?;
        let x = read_i32_le(r)?;
        let y = read_i32_le(r)?;
        let lock_x = read_i32_le(r)?;
        let lock_y = read_i32_le(r)?;
        let warp_x = read_i32_le(r)?;
        let warp_y = read_i32_le(r)?;
        let num_blocks = read_i32_le(r)?;
        md.warp_exits.push(WarpExit {
            direction, connection, id, x, y,
            lock_x, lock_y, warp_x, warp_y, num_blocks,
        });
    }
    // Skip surplus warps (> MAXWARPS)
    if num_warp_exits > MAX_WARPS {
        for _ in 0..(num_warp_exits - MAX_WARPS) {
            for _ in 0..10 {
                let _ = read_i32_le(r);
            }
        }
    }

    // 14. Spawn areas (6 types)
    md.spawn_areas = Vec::with_capacity(NUM_SPAWN_AREA_TYPES);
    for _ in 0..NUM_SPAWN_AREA_TYPES {
        let count = read_i32_le(r)? as usize;
        let clamped = count.min(MAX_SPAWN_AREAS);
        let mut areas = Vec::with_capacity(clamped);
        for _ in 0..clamped {
            let left = read_i32_le(r)?;
            let top = read_i32_le(r)?;
            let width = read_i32_le(r)?;
            let height = read_i32_le(r)?;
            let size = read_i32_le(r)?;
            areas.push(SpawnArea { left, top, width, height, size });
        }
        // Skip surplus
        for _ in clamped..count {
            for _ in 0..5 {
                let _ = read_i32_le(r);
            }
        }
        // If no valid spawn areas, create a default
        if areas.is_empty() || areas.iter().map(|a| a.size).sum::<i32>() == 0 {
            areas = vec![SpawnArea { left: 0, top: 1, width: 20, height: 12, size: 220 }];
        }
        md.spawn_areas.push(areas);
    }

    // 15. Draw areas
    let num_draw_areas = read_i32_le(r)? as usize;
    let clamped_da = num_draw_areas.min(MAX_DRAW_AREAS);
    for _ in 0..clamped_da {
        let x = read_i32_le(r)?;
        let y = read_i32_le(r)?;
        let w = read_i32_le(r)?;
        let h = read_i32_le(r)?;
        md.draw_areas.push(DrawArea { x, y, w, h });
    }

    // The remaining sections (extra tile data, gamemode settings) are
    // optional and not critical for rendering.  We silently ignore parse
    // failures from here.
    let _ = read_extra_tiledata(r, &mut md);

    md.loaded = true;
    Ok(md)
}

fn read_extra_tiledata(r: &mut Cursor<&[u8]>, md: &mut MapData) -> io::Result<()> {
    let count = read_i32_le(r)? as usize;
    for _ in 0..count {
        let col = read_i8_val(r)? as usize;
        let row = read_i8_val(r)? as usize;
        let num_settings = read_i8_val(r)? as usize;
        for s in 0..num_settings {
            let val = read_i8_val(r)?;
            if col < W && row < H && s < 8 {
                md.object_data[col][row].settings[s] = val;
            }
        }
    }
    // Gamemode settings (race goals, flag bases) – skip gracefully
    let _ = read_gamemode_settings(r);
    Ok(())
}

fn read_gamemode_settings(r: &mut Cursor<&[u8]>) -> io::Result<()> {
    // Race goals
    let n = read_i32_le(r)? as usize;
    for _ in 0..n {
        let _ = read_i32_le(r)?;
        let _ = read_i32_le(r)?;
    }
    // Flag bases
    let n = read_i32_le(r)? as usize;
    for _ in 0..n {
        let _ = read_i32_le(r)?;
        let _ = read_i32_le(r)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Platform reading
// ---------------------------------------------------------------------------

fn read_platform(r: &mut Cursor<&[u8]>, patch: i32) -> io::Result<MovingPlatform> {
    let tile_width = read_i32_le(r)?;
    let tile_height = read_i32_le(r)?;

    let mut tiles = Vec::with_capacity((tile_width * tile_height) as usize);
    for _ in 0..tile_width {
        for _ in 0..tile_height {
            let id = read_i8_val(r)?;
            let col = read_i8_val(r)?;
            let row = read_i8_val(r)?;
            let coll = CollisionType::from_raw(read_i32_le(r)?);
            tiles.push(PlatformTile { tileset_id: id, col, row, collision: coll });
        }
    }

    // Draw layer (v1.8.1+)
    let draw_layer = if patch >= 1 {
        read_i32_le(r)?
    } else {
        2
    };

    let path_type = read_i32_le(r)?;
    let path = match path_type {
        0 => {
            let sx = read_f32_le(r)?;
            let sy = read_f32_le(r)?;
            let ex = read_f32_le(r)?;
            let ey = read_f32_le(r)?;
            let speed = read_f32_le(r)?;
            PlatformPath::Straight { start_x: sx, start_y: sy, end_x: ex, end_y: ey, speed }
        }
        1 => {
            let sx = read_f32_le(r)?;
            let sy = read_f32_le(r)?;
            let angle = read_f32_le(r)?;
            let speed = read_f32_le(r)?;
            PlatformPath::Continuous { start_x: sx, start_y: sy, angle, speed }
        }
        2 => {
            let rx = read_f32_le(r)?;
            let ry = read_f32_le(r)?;
            let cx = read_f32_le(r)?;
            let cy = read_f32_le(r)?;
            let angle = read_f32_le(r)?;
            let speed = read_f32_le(r)?;
            PlatformPath::Ellipse {
                radius_x: rx, radius_y: ry,
                center_x: cx, center_y: cy,
                start_angle: angle, speed,
            }
        }
        _ => PlatformPath::Straight {
            start_x: 0.0, start_y: 0.0, end_x: 0.0, end_y: 0.0, speed: 0.0,
        },
    };

    // Compute initial SMW pixel position, then leave it in SMW space for now.
    // The spawner (spawn_platforms) converts to Bevy coordinates.
    let (cx, cy) = match &path {
        PlatformPath::Straight { start_x, start_y, .. } => (*start_x, *start_y),
        PlatformPath::Continuous { start_x, start_y, .. } => (*start_x, *start_y),
        PlatformPath::Ellipse { center_x, center_y, radius_x, radius_y, start_angle, .. } => {
            (center_x + radius_x * start_angle.cos(), center_y + radius_y * start_angle.sin())
        }
    };

    Ok(MovingPlatform {
        tile_width,
        tile_height,
        tiles,
        draw_layer,
        path,
        current_x: cx,
        current_y: cy,
        vel_x: 0.0,
        vel_y: 0.0,
        angle: 0.0,
        direction: 1.0,
    })
}

// ---------------------------------------------------------------------------
// Default / fallback map
// ---------------------------------------------------------------------------

/// Generates a simple playable map when the binary loader fails.
pub fn generate_default_map() -> MapData {
    let mut md = MapData::default();

    // Allocate storage
    md.tiles = vec![vec![vec![TilesetTileRef { id: TILESET_NONE, col: 0, row: 0 }; LAYERS]; H]; W];
    md.object_data = vec![vec![ObjectBlock::default(); H]; W];
    md.collision_map = vec![vec![CollisionType::NonSolid; H]; W];
    md.warp_data = vec![vec![PerTileWarp::default(); H]; W];
    md.no_spawn = vec![vec![vec![false; H]; W]; NUM_SPAWN_AREA_TYPES];

    // Ground: bottom 2 rows solid
    for col in 0..W {
        for row in (H - 2)..H {
            md.collision_map[col][row] = CollisionType::Solid;
            // Use a basic tile visual (tileset 0, col 0, row 0 = usually a ground tile)
            for layer in 0..LAYERS {
                md.tiles[col][row][layer] = TilesetTileRef { id: 0, col: 0, row: 0 };
            }
        }
    }

    // Middle platforms
    // Platform 1: row 8, cols 3-7
    for col in 3..8 {
        md.collision_map[col][8] = CollisionType::SolidOnTop;
        md.tiles[col][8][0] = TilesetTileRef { id: 0, col: 2, row: 0 };
    }
    // Platform 2: row 5, cols 12-16
    for col in 12..17 {
        md.collision_map[col][5] = CollisionType::SolidOnTop;
        md.tiles[col][5][0] = TilesetTileRef { id: 0, col: 2, row: 0 };
    }
    // Platform 3: row 10, cols 8-11
    for col in 8..12 {
        md.collision_map[col][10] = CollisionType::SolidOnTop;
        md.tiles[col][10][0] = TilesetTileRef { id: 0, col: 2, row: 0 };
    }

    // Default spawn areas
    md.spawn_areas = vec![
        vec![SpawnArea { left: 0, top: 1, width: 20, height: 11, size: 200 }];
        NUM_SPAWN_AREA_TYPES
    ];

    md.background_file = "Land_Classic.png".to_string();
    md.loaded = true;

    md
}

// ---------------------------------------------------------------------------
// Conversion helper (kept for compatibility)
// ---------------------------------------------------------------------------

/// Converts a raw collision integer to our TileType enum (legacy path).
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
