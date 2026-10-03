//! Port of src/common/map/MapReader.cpp
//!
//! The C++ reader class hierarchy (MapReader1500 ... MapReader1802) is one struct tagged with the
//! concrete class; each `read_*` method dispatches to the override that class would use.

use crate::globals::Aliased;
use crate::common::file_io::BinaryFile;
use crate::common::map::{CMap, ReadType};
use crate::common::version::Version;

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

pub struct MapReader {
    pub kind: MapReaderKind,

    pub parse_nospawn: bool,
    pub fix_spawnareas: bool,

    pub patch_version: u8,

    pub iMaxTilesetID: i16,
    pub translationid: Option<Vec<i16>>,
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

    pub fn load(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) -> bool {
        use MapReaderKind::*;
        match self.kind {
            R1500 => self.load_1500(map, mapfile, readtype),
            R1600 | R160A => self.load_1600(map, mapfile, readtype),
            R1610 => self.load_1610(map, mapfile, readtype),
            R1700 | R1701 | R1702 => self.load_1700(map, mapfile, readtype),
            R1800 | R1801 | R1802 => self.load_1800(map, mapfile, readtype),
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

    pub(super) fn read_tiles(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        use MapReaderKind::*;
        match self.kind {
            R1500 => self.read_tiles_1500(map, mapfile),
            R1600 | R160A | R1610 => self.read_tiles_1600(map, mapfile),
            R1700 | R1701 | R1702 => self.read_tiles_1700(map, mapfile),
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

    pub(super) fn read_spawn_areas(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) -> bool {
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

    pub(super) fn read_platforms(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, preview: bool) {
        use MapReaderKind::*;
        match self.kind {
            R1800 | R1801 | R1802 => self.read_platforms_1800(map, mapfile, preview),
            _ => self.read_platforms_1700(map, mapfile, preview),
        }
    }
}

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
