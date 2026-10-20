//! Map file readers for every supported format version. One [`MapReader`] struct is tagged with its format
//! version, and each `read_*` method dispatches to the section layout that version uses.

use crate::game::Game;
use crate::globals::Aliased;
use crate::common::file_io::{throw_runtime_error, BinaryFile};
use crate::common::global_constants::{MAPHEIGHT, MAPWIDTH, MAXWARPS};
use crate::common::map::{CMap, ReadType, Warp, WARP_UNDEFINED};
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
