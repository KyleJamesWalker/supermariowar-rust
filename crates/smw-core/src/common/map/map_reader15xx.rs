//! Reader for 1.5 map files: one classic-tileset layer, objects, and a numeric background ID.

use crate::game::Game;
use crate::common::file_io::BinaryFile;
use crate::common::game_values::default_powerup_setting;
use crate::common::global_constants::*;
use crate::common::map::map_reader::{check_object_type, checked_index, MapReader};
use crate::common::map::map_reader_constants::g_szBackgroundConversion;
use crate::common::map::{read_type_summary, CMap, ReadType};

/// Music category for each legacy background ID.
const g_iMusicCategoryConversion: [i16; 26] = [0, 3, 8, 5, 1, 9, 3, 4, 10, 8, 1, 0, 9, 0, 0, 7, 4, 1, 1, 6, 4, 7, 6, 3, 0, 4];

impl MapReader {
    pub(super) fn read_autofilters_1500(&mut self, map: &mut CMap, _mapfile: &mut BinaryFile) {
        for iFilter in 0..NUM_AUTO_FILTERS as usize {
            map.fAutoFilter[iFilter] = false;
        }
    }

    pub(super) fn read_tiles_1500(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile) {
        let mut gp = g.ptr();
        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                let iTileID: i16 = mapfile.read_i32() as i16;
                checked_index("tile", iTileID as i32, TILESETSIZE);

                let tile = &mut map.mapdata[i][j][1];
                tile.iID = g.g_tilesetmanager.classic_tileset_index() as i16;
                tile.iCol = iTileID % 32;
                tile.iRow = iTileID / 32;

                let iType = gp.g_tilesetmanager.classic_tileset(g).tile_type(tile.iCol as isize as usize, tile.iRow as isize as usize);
                map.mapdatatop[i][j] = iType;

                map.mapdata[i][j][0].iID = TILESETNONE as i16;
                map.mapdata[i][j][2].iID = TILESETNONE as i16;
                map.mapdata[i][j][3].iID = TILESETNONE as i16;
            }
        }

        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                map.objectdata[i][j].iType = mapfile.read_i32() as i16;
                if map.objectdata[i][j].iType == 6 {
                    map.objectdata[i][j].iType = -1;
                }
                check_object_type(map.objectdata[i][j].iType);

                map.objectdata[i][j].fHidden = false;

                if map.objectdata[i][j].iType == 1 {
                    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                        map.objectdata[i][j].iSettings[iSetting] = default_powerup_setting(0, iSetting);
                    }
                }
            }
        }
    }

    pub(super) fn read_background_1500(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.backgroundID = mapfile.read_i32() as i16;
        let background = checked_index("background", map.backgroundID as i32, g_szBackgroundConversion.len() as i32);
        map.szBackgroundFile = g_szBackgroundConversion[background].to_string();
    }

    pub(super) fn read_music_category_1500(&mut self, map: &mut CMap, _mapfile: &mut BinaryFile) {
        map.musicCategoryID = g_iMusicCategoryConversion[map.backgroundID as usize];
    }

    pub(super) fn load_1500(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) {
        self.read_autofilters(map, mapfile);

        if readtype == read_type_summary {
            return;
        }

        map.clear_platforms();

        // 1.5 files have no version header; the tile data starts at offset 0.
        mapfile.rewind();

        map.clear_map();

        self.read_tiles(g, map, mapfile);
        self.read_background(map, mapfile);
        self.read_music_category(map, mapfile);

        map.eyecandy[2] = 1;

        for iSwitch in 0..4 {
            map.iSwitches[iSwitch] = 0;
        }
    }
}
