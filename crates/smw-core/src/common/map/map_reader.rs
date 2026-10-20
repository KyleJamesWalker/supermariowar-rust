//! Map file readers for every supported format version. One [`MapReader`] struct is tagged with its format
//! version, and each `read_*` method dispatches to the section layout that version uses.

use crate::game::Game;
use crate::globals::Aliased;
use crate::common::file_io::{throw_runtime_error, BinaryFile};
use crate::common::global_constants::{MAPHEIGHT, MAPWIDTH, MAXWARPS};
use crate::common::map::{CMap, MapHazard, ReadType, Warp, ThrowBox, WARP_UNDEFINED};
use crate::smw::gs_gameplay::MapBlock_WpnBreakLeaf;
use crate::common::version::Version;

/// Map file format version a reader parses, named by version digits (`R160A` is 1.6.0.10).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MapReaderKind {
    R1500,
    R1600,
    R160A,
    R1610,
    R1700,
    R1701,
    R1702,
    R1800,
    R1801,
    R1802,
}

/// Reads one map file of a given format version into a [`CMap`].
pub struct MapReader {
    pub kind: MapReaderKind,

    /// The tile section stores a no-spawn flag per tile (1.6.1).
    pub parse_nospawn: bool,
    /// Spawn areas store right and bottom edges; convert them to width and height (1.6.0.10 and 1.6.1).
    pub fix_spawnareas: bool,

    /// Last version digit within 1.7 and 1.8 (0 to 2), which picks section variants.
    pub patch_version: u8,

    /// Highest tileset ID in the 1.8 tileset table, or -1 before it is read.
    pub iMaxTilesetID: i16,
    /// Map-file tileset ID to loaded tileset index, while reading a 1.8 map's tiles and platforms.
    pub translationid: Option<Vec<i16>>,
    /// Width and height in tiles of each map-file tileset ID, for bounds-checking tile coordinates.
    pub tilesetwidths: Option<Vec<i16>>,
    pub tilesetheights: Option<Vec<i16>>,
    _alias: Aliased,
}

impl MapReader {
    fn new(kind: MapReaderKind) -> Self {
        use MapReaderKind::*;
        let mut r = MapReader {
            _alias: Aliased::new(),
            kind,
            parse_nospawn: false,
            fix_spawnareas: false,
            patch_version: 0,
            iMaxTilesetID: -1,
            translationid: None,
            tilesetwidths: None,
            tilesetheights: None,
        };
        match kind {
            R160A => r.fix_spawnareas = true,
            R1610 => {
                r.fix_spawnareas = true;
                r.parse_nospawn = true;
            }
            R1701 | R1801 => r.patch_version = 1,
            R1702 | R1802 => r.patch_version = 2,
            _ => {}
        }
        r
    }

    /// Reads `mapfile`, positioned after the version header, into `map`. `readtype` can stop early for a summary
    /// or preview. A damaged file fails through `throw_runtime_error`.
    pub fn load(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) {
        use MapReaderKind::*;
        match self.kind {
            R1500 => self.load_1500(g, map, mapfile, readtype),
            R1600 | R160A => self.load_1600(g, map, mapfile, readtype),
            R1610 => self.load_1610(g, map, mapfile, readtype),
            R1700 | R1701 | R1702 => self.load_1700(g, map, mapfile, readtype),
            R1800 | R1801 | R1802 => self.load_1800(g, map, mapfile, readtype),
        }
    }

    pub(super) fn read_autofilters(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1702 => self.read_autofilters_1702(map, mapfile),
            R1800 | R1801 | R1802 => self.read_autofilters_1800(map, mapfile),
            _ => self.read_autofilters_1500(map, mapfile),
        }
    }

    pub(super) fn read_tiles(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1500 => self.read_tiles_1500(g, map, mapfile),
            R1600 | R160A | R1610 => self.read_tiles_1600(g, map, mapfile),
            R1700 | R1701 | R1702 => self.read_tiles_1700(g, map, mapfile),
            R1800 | R1801 | R1802 => self.read_tiles_1800(map, mapfile),
        }
    }

    pub(super) fn read_background(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1701 => self.read_background_1701(map, mapfile),
            R1702 | R1800 | R1801 | R1802 => self.read_background_1702(map, mapfile),
            _ => self.read_background_1500(map, mapfile),
        }
    }

    pub(super) fn read_music_category(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1500 | R1600 | R160A | R1610 | R1700 => self.read_music_category_1500(map, mapfile),
            _ => self.read_music_category_1701(map, mapfile),
        }
    }

    pub(super) fn read_eyecandy(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        match self.kind {
            MapReaderKind::R1802 => self.read_eyecandy_1802(map, mapfile),
            _ => self.read_eyecandy_1600(map, mapfile),
        }
    }

    pub(super) fn read_spawn_areas(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1700 | R1701 | R1702 => self.read_spawn_areas_1700(map, mapfile),
            R1800 | R1801 | R1802 => self.read_spawn_areas_1800(map, mapfile),
            _ => self.read_spawn_areas_1600(map, mapfile),
        }
    }

    pub(super) fn read_warp_locations(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1800 | R1801 | R1802 => self.read_warp_locations_1800(map, mapfile),
            _ => self.read_warp_locations_1700(map, mapfile),
        }
    }

    pub(super) fn read_switches(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1800 | R1801 | R1802 => self.read_switches_1800(map, mapfile),
            _ => self.read_switches_1700(map, mapfile),
        }
    }

    pub(super) fn read_platforms(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile, preview: bool) {
        use MapReaderKind::*;
        match self.kind {
            R1800 | R1801 | R1802 => self.read_platforms_1800(g, map, mapfile, preview),
            _ => self.read_platforms_1700(g, map, mapfile, preview),
        }
    }
}

/// `value` as an index below `len`; any other value fails the load as a damaged map.
pub(super) fn checked_index(what: &str, value: i32, len: i32) -> usize {
    if !(0..len).contains(&value) {
        throw_runtime_error(&format!("Damaged map: {what} {value} is not in 0..{len}"));
    }
    value as usize
}

/// `value` as a count of at most `max`; any other value fails the load as a damaged map.
pub(super) fn checked_count(what: &str, value: i32, max: i32) -> usize {
    checked_index(what, value, max + 1)
}

/// `value` when it is in `min..=max`; any other value fails the load as a damaged map.
pub(super) fn checked_range(what: &str, value: i32, min: i32, max: i32) -> i32 {
    if !(min..=max).contains(&value) {
        throw_runtime_error(&format!("Damaged map: {what} {value} is not in {min}..={max}"));
    }
    value
}

/// Positions, speeds and angles in map files stay far inside this; past it, paths that wrap an angle never finish.
const MAX_MAP_FLOAT: f32 = 65536.0;

/// `value` when it is finite and within `MAX_MAP_FLOAT`; anything else fails the load as a damaged map.
pub(super) fn checked_float(what: &str, value: f32) -> f32 {
    if !(value.abs() <= MAX_MAP_FLOAT) {
        throw_runtime_error(&format!("Damaged map: {what} {value} is not within {MAX_MAP_FLOAT}"));
    }
    value
}

/// Fails the load on a tile object type no block exists for; -1 is no object.
pub(super) fn check_object_type(object_type: i16) {
    checked_range("object type", object_type as i32, -1, MapBlock_WpnBreakLeaf);
}

/// Fails the load on an item the game cannot place inside the map.
pub(super) fn check_item(item_type: i32, x: i16, y: i16) {
    checked_range("item type", item_type, 0, ThrowBox as i32);
    checked_index("item column", x as i32, MAPWIDTH);
    checked_index("item row", y as i32, MAPHEIGHT);
}

/// Fails the load on a hazard the game cannot create: unknown type, off the map in half tiles, a count the editor
/// never writes, a zero or negative frequency, a direction past the hazard's sprites, or an unusable float.
pub(super) fn check_hazard(hazard: &MapHazard) {
    checked_range("hazard type", hazard.itype as i32, 0, 7);
    checked_index("hazard column", hazard.ix as i32, 2 * MAPWIDTH);
    checked_index("hazard row", hazard.iy as i32, 2 * MAPHEIGHT);
    if hazard.itype <= 1 {
        checked_range("hazard count", hazard.iparam[0] as i32, 0, 9);
    } else {
        checked_range("hazard frequency", hazard.iparam[0] as i32, 1, i16::MAX as i32);
    }
    if hazard.itype >= 3 {
        checked_index("hazard direction", hazard.iparam[1] as i32, 4);
    }
    for value in hazard.dparam {
        checked_float("hazard parameter", value);
    }
}

/// Fails the load when a warp entrance's connection has no exit, which a player entering it would need.
pub(super) fn check_warp_exits(map: &CMap) {
    let exits = &map.warpexits[..map.numwarpexits as usize];
    for warp in map.warpdata.iter().flatten() {
        if warp.direction != WARP_UNDEFINED && !exits.iter().any(|e| e.connection == warp.connection) {
            throw_runtime_error(&format!("Damaged map: warp connection {} has no exit", warp.connection));
        }
    }
}

/// Fails the load on a platform size the map cannot hold.
pub(super) fn check_platform_size(width: i16, height: i16) {
    if !(1..=MAPWIDTH).contains(&(width as i32)) || !(1..=MAPHEIGHT).contains(&(height as i32)) {
        throw_runtime_error(&format!("Damaged map: platform size {width}x{height} is not within {MAPWIDTH}x{MAPHEIGHT}"));
    }
}

/// Fails the load when a warp entrance names a connection or exit the map cannot hold.
pub(super) fn check_warp(map: &CMap, warp: &Warp) {
    if warp.direction != WARP_UNDEFINED {
        checked_index("warp connection", warp.connection as i32, map.warplocked.len() as i32);
        checked_index("warp exit", warp.id as i32, MAXWARPS);
    }
}

/// The reader for the newest format version not above `mapversion`; anything below 1.6 reads as 1.5.
pub fn get_loader_by_version(mapversion: &Version) -> Box<MapReader> {
    use MapReaderKind::*;
    let v = |major, minor, patch, build| Version { major, minor, patch, build };

    let kind = if *mapversion >= v(1, 8, 0, 2) {
        R1802
    } else if *mapversion >= v(1, 8, 0, 1) {
        R1801
    } else if *mapversion >= v(1, 8, 0, 0) {
        R1800
    } else if *mapversion >= v(1, 7, 0, 2) {
        R1702
    } else if *mapversion >= v(1, 7, 0, 1) {
        R1701
    } else if *mapversion >= v(1, 7, 0, 0) {
        R1700
    } else if *mapversion >= v(1, 6, 1, 0) {
        R1610
    } else if *mapversion >= v(1, 6, 0, 10) {
        R160A
    } else if *mapversion >= v(1, 6, 0, 0) {
        R1600
    } else {
        R1500
    };

    Box::new(MapReader::new(kind))
}
