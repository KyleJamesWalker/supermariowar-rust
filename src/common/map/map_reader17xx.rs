//! Port of src/common/map/MapReader17xx.cpp

use crate::common::file_io::BinaryFile;
use crate::common::game_values::default_powerup_setting;
use crate::common::global::g_tilesetmanager;
use crate::common::global_constants::*;
use crate::common::map::map_reader::MapReader;
use crate::common::map::map_reader_constants::g_szBackgroundConversion;
use crate::common::map::{read_type_preview, read_type_summary, CMap, ReadType, TilesetTile, WarpEnterDirection};
use crate::common::math::vec2::Vec2f;
use crate::common::moving_platform_paths::{EllipsePath, MovingPlatformPathTrait, StraightPath, StraightPathContinuous};
use crate::common::movingplatform::MovingPlatform;
use crate::common::tile_types::TileType;

impl MapReader {
    pub(super) fn read_autofilters_1702(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let mut iAutoFilterValues = [0i32; 9];
        mapfile.read_i32_array(&mut iAutoFilterValues);

        for iFilter in 0..8 {
            map.fAutoFilter[iFilter] = iAutoFilterValues[iFilter] > 0;
        }

        for iFilter in 8..NUM_AUTO_FILTERS as usize {
            map.fAutoFilter[iFilter] = false;
        }
    }

    pub(super) fn read_tiles_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iClassicTilesetID: i16 = unsafe { g_tilesetmanager.index_from_name("Classic") as i16 };

        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                for k in 0..MAPLAYERS as usize {
                    let iTileID: i16 = mapfile.read_i32() as i16;

                    let tile = &mut map.mapdata[i][j][k];

                    if iTileID as i32 == TILESETSIZE {
                        tile.iID = TILESETNONE as i16;
                        tile.iCol = 0;
                        tile.iRow = 0;
                    } else {
                        tile.iID = iClassicTilesetID;
                        tile.iCol = (iTileID as i32 % TILESETWIDTH) as i16;
                        tile.iRow = (iTileID as i32 / TILESETWIDTH) as i16;
                    }
                }

                map.objectdata[i][j].iType = mapfile.read_i32() as i16;
                if map.objectdata[i][j].iType == 15 {
                    map.objectdata[i][j].iType = -1;
                }

                map.objectdata[i][j].fHidden = false;

                if map.objectdata[i][j].iType == 1 {
                    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                        map.objectdata[i][j].iSettings[iSetting] = default_powerup_setting(0, iSetting);
                    }
                }
            }
        }
    }

    pub(super) fn read_background_1701(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.szBackgroundFile = mapfile.read_string_long(128);

        for background in g_szBackgroundConversion {
            let underscorePos = background.find('_').expect("All items must have an underscore in g_szBackgroundConversion");

            if background[underscorePos + 1..] == map.szBackgroundFile {
                map.szBackgroundFile = background.to_string();
                break;
            }
        }
    }

    pub(super) fn read_background_1702(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.szBackgroundFile = mapfile.read_string_long(128);
    }

    pub(super) fn set_preview_switches(&mut self, map: &mut CMap, _mapfile: &mut BinaryFile) {
        for iSwitch in 0..4 {
            map.iSwitches[iSwitch] = 1;
        }

        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                if map.objectdata[i][j].iType >= 11 && map.objectdata[i][j].iType <= 14 {
                    map.objectdata[i][j].iSettings[0] = 1;
                }
            }
        }
    }

    pub(super) fn read_switches_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        for iSwitch in 0..4 {
            map.iSwitches[iSwitch] = (1 - mapfile.read_i32() as i16 as i32) as i16;
        }

        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                if map.objectdata[i][j].iType >= 11 && map.objectdata[i][j].iType <= 14 {
                    map.objectdata[i][j].iSettings[0] = map.iSwitches[(map.objectdata[i][j].iType - 11) as usize];
                }
            }
        }
    }

    pub(super) fn read_music_category_1701(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.musicCategoryID = mapfile.read_i32() as i16;
    }

    pub(super) fn read_warp_locations_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                map.mapdatatop[i][j] = TileType::from_i32(mapfile.read_i32());

                map.warpdata[i][j].direction = mapfile.read_i32() as WarpEnterDirection;
                map.warpdata[i][j].connection = mapfile.read_i32() as i16;
                map.warpdata[i][j].id = mapfile.read_i32() as i16;

                let mut sType = 0usize;
                while sType < 6 {
                    map.nospawn[sType][i][j] = mapfile.read_i32() != 0;
                    sType += 5;
                }

                for sType in 1..5 {
                    map.nospawn[sType][i][j] = map.nospawn[0][i][j];
                }
            }
        }
    }

    pub(super) fn read_spawn_areas_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) -> bool {
        let mut i = 0usize;
        while i < 6 {
            map.totalspawnsize[i] = 0;
            map.numspawnareas[i] = mapfile.read_i32() as i16;

            if map.numspawnareas[i] as i32 > MAXSPAWNAREAS {
                println!();
                println!(" ERROR: Number of spawn areas ({}) was greater than max allowed ({})", map.numspawnareas[i], MAXSPAWNAREAS);
                return false;
            }

            for m in 0..map.numspawnareas[i].max(0) as usize {
                let a = &mut map.spawnareas[i][m];
                a.left = mapfile.read_i32() as i16;
                a.top = mapfile.read_i32() as i16;
                a.width = mapfile.read_i32() as i16;
                a.height = mapfile.read_i32() as i16;
                a.size = mapfile.read_i32() as i16;

                map.totalspawnsize[i] = map.totalspawnsize[i].wrapping_add(a.size);
            }
            i += 5;
        }

        for iType in 1..5usize {
            map.totalspawnsize[iType] = map.totalspawnsize[0];
            map.numspawnareas[iType] = map.numspawnareas[0];

            for m in 0..map.numspawnareas[0].max(0) as usize {
                map.spawnareas[iType][m] = map.spawnareas[0][m];
            }
        }

        true
    }

    pub(super) fn read_platforms_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, fPreview: bool) {
        map.clear_platforms();

        let iNumPlatforms: usize = mapfile.read_i32() as i16 as isize as usize;
        map.platforms.reserve(iNumPlatforms.min(1024));

        for _idx in 0..iNumPlatforms {
            let iWidth: i16 = mapfile.read_i32() as i16;
            let iHeight: i16 = mapfile.read_i32() as i16;

            let (tiles, types) = self.read_platform_tiles_1700(map, mapfile, iWidth, iHeight);

            let iDrawLayer: i16 = 2;

            let iPathType: i16 = 0;

            let Some(path) = self.read_platform_path_details(mapfile, iPathType, fPreview) else {
                continue;
            };

            let platform = MovingPlatform::new(tiles, types, iWidth, iHeight, iDrawLayer, path, fPreview);
            map.platforms.push(platform);
            map.platformdrawlayer[iDrawLayer as usize].push(platform);
        }
    }

    fn read_platform_tiles_1700(&mut self, _map: &mut CMap, mapfile: &mut BinaryFile, iWidth: i16, iHeight: i16) -> (Vec<TilesetTile>, Vec<TileType>) {
        let n = (iWidth as i32 * iHeight as i32).max(0) as usize;
        let mut tiles = Vec::with_capacity(n);
        let mut types = Vec::with_capacity(n);

        for _iCol in 0..iWidth {
            for _iRow in 0..iHeight {
                let iTile: i16 = mapfile.read_i32() as i16;

                let mut tile = TilesetTile::default();
                let r#type: TileType;

                if iTile as i32 == TILESETSIZE {
                    tile.iID = TILESETNONE as i16;
                    tile.iCol = 0;
                    tile.iRow = 0;

                    r#type = TileType::NonSolid;
                } else {
                    unsafe {
                        tile.iID = g_tilesetmanager.classic_tileset_index() as i16;
                        tile.iCol = (iTile as i32 % TILESETWIDTH) as i16;
                        tile.iRow = (iTile as i32 / TILESETWIDTH) as i16;

                        r#type = g_tilesetmanager.classic_tileset().tile_type(tile.iCol as isize as usize, tile.iRow as isize as usize);
                    }
                }

                tiles.push(tile);
                types.push(r#type);
            }
        }

        (tiles, types)
    }

    pub(super) fn read_platform_path_details(
        &mut self,
        mapfile: &mut BinaryFile,
        iPathType: i16,
        fPreview: bool,
    ) -> Option<Box<dyn MovingPlatformPathTrait>> {
        if iPathType == 0 {
            let startX = mapfile.read_float();
            let startY = mapfile.read_float();
            let endX = mapfile.read_float();
            let endY = mapfile.read_float();
            let speed = mapfile.read_float();

            return Some(Box::new(StraightPath::new(speed, Vec2f::new(startX, startY), Vec2f::new(endX, endY), fPreview)));
        }
        if iPathType == 1 {
            let startX = mapfile.read_float();
            let startY = mapfile.read_float();
            let angle = mapfile.read_float();
            let speed = mapfile.read_float();

            return Some(Box::new(StraightPathContinuous::new(speed, Vec2f::new(startX, startY), angle, fPreview)));
        }
        if iPathType == 2 {
            let radiusX = mapfile.read_float();
            let radiusY = mapfile.read_float();
            let centerX = mapfile.read_float();
            let centerY = mapfile.read_float();
            let angle = mapfile.read_float();
            let speed = mapfile.read_float();

            return Some(Box::new(EllipsePath::new(speed, angle, Vec2f::new(radiusX, radiusY), Vec2f::new(centerX, centerY), fPreview)));
        }
        None
    }

    pub(super) fn load_1700(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) -> bool {
        self.read_autofilters(map, mapfile);

        if readtype == read_type_summary {
            return true;
        }

        map.clear_platforms();

        self.read_tiles(map, mapfile);
        self.read_background(map, mapfile);

        if self.patch_version >= 1 {
            self.read_switches(map, mapfile);
        } else if readtype != read_type_preview {
            self.set_preview_switches(map, mapfile);
        }

        if self.patch_version >= 2 {
            self.read_platforms(map, mapfile, readtype == read_type_preview);
        }

        self.read_eyecandy(map, mapfile);
        self.read_music_category(map, mapfile);
        self.read_warp_locations(map, mapfile);

        if readtype == read_type_preview {
            return true;
        }

        self.read_warp_exits(map, mapfile);
        self.read_spawn_areas(map, mapfile);
        if !self.read_draw_areas(map, mapfile) {
            return false;
        }

        if self.patch_version <= 1 {
            self.read_platforms(map, mapfile, readtype == read_type_preview);
        }

        if self.patch_version == 0 {
            self.read_switches(map, mapfile);
        }

        true
    }
}
