//! Port of src/common/map/MapReader18xx.cpp

use crate::common::file_io::BinaryFile;
use crate::common::global::g_tilesetmanager;
use crate::common::global_constants::*;
use crate::common::map::map_reader::MapReader;
use crate::common::map::{read_type_preview, read_type_summary, CMap, MapHazard, MapItem, MapItemType, ReadType, TilesetTile, WarpEnterDirection};
use crate::common::movingplatform::MovingPlatform;
use crate::common::tile_types::TileType;

struct TilesetTranslation {
    iID: i16,
    szName: String,
}

impl MapReader {
    pub(super) fn read_autofilters_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let mut iAutoFilterValues = [0i32; NUM_AUTO_FILTERS as usize + 1];
        mapfile.read_i32_array(&mut iAutoFilterValues);

        for iFilter in 0..NUM_AUTO_FILTERS as usize {
            map.fAutoFilter[iFilter] = iAutoFilterValues[iFilter] > 0;
        }
    }

    fn read_tileset(&mut self, mapfile: &mut BinaryFile) {
        let iNumTilesets: i16 = mapfile.read_i32() as i16;

        let mut translation: Vec<TilesetTranslation> = Vec::with_capacity(iNumTilesets.max(0) as usize);

        self.iMaxTilesetID = 0;
        for _iTileset in 0..iNumTilesets {
            let iTilesetID: i16 = mapfile.read_i32() as i16;

            if iTilesetID > self.iMaxTilesetID {
                self.iMaxTilesetID = iTilesetID;
            }

            let szName = mapfile.read_string_long(128);
            translation.push(TilesetTranslation { iID: iTilesetID, szName });
        }

        // C++ leaves entries for IDs missing from the table uninitialized (`new short[]`).
        let n = (self.iMaxTilesetID as i32 + 1) as usize;
        let mut translationid = vec![0i16; n];
        let mut tilesetwidths = vec![0i16; n];
        let mut tilesetheights = vec![0i16; n];

        unsafe {
            for t in &translation {
                let iID = t.iID as usize;
                translationid[iID] = g_tilesetmanager.index_from_name(&t.szName) as i16;

                if translationid[iID] as i32 == TILESETUNKNOWN {
                    tilesetwidths[iID] = 1;
                    tilesetheights[iID] = 1;
                } else {
                    let ts = g_tilesetmanager.tileset(translationid[iID] as isize as usize);
                    tilesetwidths[iID] = ts.width();
                    tilesetheights[iID] = ts.height();
                }
            }
        }

        self.translationid = Some(translationid);
        self.tilesetwidths = Some(tilesetwidths);
        self.tilesetheights = Some(tilesetheights);
    }

    pub(super) fn read_tiles_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let translationid = self.translationid.as_ref().unwrap();
        let tilesetwidths = self.tilesetwidths.as_ref().unwrap();
        let tilesetheights = self.tilesetheights.as_ref().unwrap();

        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                for k in 0..MAPLAYERS as usize {
                    let tile = &mut map.mapdata[i][j][k];
                    tile.iID = mapfile.read_i8() as i16;
                    tile.iCol = mapfile.read_i8() as i16;
                    tile.iRow = mapfile.read_i8() as i16;

                    if tile.iID >= 0 {
                        if tile.iID > self.iMaxTilesetID {
                            tile.iID = 0;
                        }

                        if tile.iCol < 0 || tile.iCol >= tilesetwidths[tile.iID as usize] {
                            tile.iCol = 0;
                        }

                        if tile.iRow < 0 || tile.iRow >= tilesetheights[tile.iID as usize] {
                            tile.iRow = 0;
                        }

                        tile.iID = translationid[tile.iID as usize];
                    }
                }

                map.objectdata[i][j].iType = mapfile.read_i8() as i16;
                map.objectdata[i][j].fHidden = mapfile.read_bool();
            }
        }
    }

    pub(super) fn read_switches_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        for iSwitch in 0..4 {
            map.iSwitches[iSwitch] = mapfile.read_i32() as i16;
        }
    }

    fn read_items(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumMapItems: usize = mapfile.read_i32() as isize as usize;

        for _idx in 0..iNumMapItems {
            let item = MapItem { itype: mapfile.read_i32() as MapItemType, ix: mapfile.read_i32() as i16, iy: mapfile.read_i32() as i16 };
            map.mapitems.push(item);
        }
    }

    fn read_hazards(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumMapHazards: usize = mapfile.read_i32() as isize as usize;

        for _idx in 0..iNumMapHazards {
            let mut hazard = MapHazard::default();
            hazard.itype = mapfile.read_i32() as i16;
            hazard.ix = mapfile.read_i32() as i16;
            hazard.iy = mapfile.read_i32() as i16;

            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                hazard.iparam[iParam] = mapfile.read_i32() as i16;
            }

            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                hazard.dparam[iParam] = mapfile.read_float();
            }

            map.maphazards.push(hazard);
        }
    }

    pub(super) fn read_eyecandy_1802(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.eyecandy[0] = mapfile.read_i32() as i16;
        map.eyecandy[1] = mapfile.read_i32() as i16;
        map.eyecandy[2] = mapfile.read_i32() as i16;
    }

    pub(super) fn read_warp_locations_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        for j in 0..MAPHEIGHT as usize {
            for i in 0..MAPWIDTH as usize {
                map.mapdatatop[i][j] = TileType::from_i32(mapfile.read_i32());

                map.warpdata[i][j].direction = mapfile.read_i32() as WarpEnterDirection;
                map.warpdata[i][j].connection = mapfile.read_i32() as i16;
                map.warpdata[i][j].id = mapfile.read_i32() as i16;

                for sType in 0..NUMSPAWNAREATYPES as usize {
                    map.nospawn[sType][i][j] = mapfile.read_bool();
                }
            }
        }
    }

    fn read_switchable_blocks(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumSwitchBlockData: i32 = mapfile.read_i32();
        let mut iBlock: i16 = 0;
        while (iBlock as i32) < iNumSwitchBlockData {
            let iCol: i16 = mapfile.read_i8() as i16;
            let iRow: i16 = mapfile.read_i8() as i16;

            map.objectdata[iCol as usize][iRow as usize].iSettings[0] = mapfile.read_i8() as i16;
            iBlock = iBlock.wrapping_add(1);
        }
    }

    pub(super) fn read_spawn_areas_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) -> bool {
        for i in 0..NUMSPAWNAREATYPES as usize {
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

            if map.totalspawnsize[i] == 0 {
                map.numspawnareas[i] = 1;
                map.spawnareas[i][0].left = 0;
                map.spawnareas[i][0].width = 20;
                map.spawnareas[i][0].top = 1;
                map.spawnareas[i][0].height = 12;
                map.spawnareas[i][0].size = 220;
                map.totalspawnsize[i] = 220;
            }
        }

        true
    }

    fn read_extra_tiledata(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumExtendedDataBlocks: i32 = mapfile.read_i32();

        let mut iBlock: i16 = 0;
        while (iBlock as i32) < iNumExtendedDataBlocks {
            let iCol: i16 = mapfile.read_i8() as i16;
            let iRow: i16 = mapfile.read_i8() as i16;

            let iNumSettings: i16 = mapfile.read_i8() as i16;
            for iSetting in 0..iNumSettings {
                map.objectdata[iCol as usize][iRow as usize].iSettings[iSetting as usize] = mapfile.read_i8() as i16;
            }
            iBlock = iBlock.wrapping_add(1);
        }
    }

    fn read_gamemode_settings(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.iNumRaceGoals = mapfile.read_i32() as i16;
        let mut j: u16 = 0;
        while (j as i32) < map.iNumRaceGoals as i32 {
            map.racegoallocations[j as usize].x = mapfile.read_i32() as i16;
            map.racegoallocations[j as usize].y = mapfile.read_i32() as i16;
            j += 1;
        }

        map.iNumFlagBases = mapfile.read_i32() as i16;
        let mut j: u16 = 0;
        while (j as i32) < map.iNumFlagBases as i32 {
            map.flagbaselocations[j as usize].x = mapfile.read_i32() as i16;
            map.flagbaselocations[j as usize].y = mapfile.read_i32() as i16;
            j += 1;
        }
    }

    pub(super) fn read_platforms_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, fPreview: bool) {
        map.clear_platforms();

        let iNumPlatforms: usize = mapfile.read_i32() as i16 as isize as usize;
        map.platforms.reserve(iNumPlatforms.min(1024));

        for _idx in 0..iNumPlatforms {
            let iWidth: i16 = mapfile.read_i32() as i16;
            let iHeight: i16 = mapfile.read_i32() as i16;

            let (tiles, types) = self.read_platform_tiles_1800(mapfile, iWidth, iHeight);

            let mut iDrawLayer: i16 = 2;
            if self.patch_version >= 1 {
                iDrawLayer = mapfile.read_i32() as i16;
            }

            let iPathType: i16 = mapfile.read_i32() as i16;

            let Some(path) = self.read_platform_path_details(mapfile, iPathType, fPreview) else {
                continue;
            };

            let platform = MovingPlatform::new(tiles, types, iWidth, iHeight, iDrawLayer, path, fPreview);
            map.platforms.push(platform);
            map.platformdrawlayer[iDrawLayer as usize].push(platform);
        }
    }

    fn read_platform_tiles_1800(&mut self, mapfile: &mut BinaryFile, iWidth: i16, iHeight: i16) -> (Vec<TilesetTile>, Vec<TileType>) {
        let n = (iWidth as i32 * iHeight as i32).max(0) as usize;
        let mut tiles = Vec::with_capacity(n);
        let mut types = Vec::with_capacity(n);

        for _iCol in 0..iWidth {
            for _iRow in 0..iHeight {
                let mut tile = TilesetTile { iID: mapfile.read_i8() as i16, iCol: mapfile.read_i8() as i16, iRow: mapfile.read_i8() as i16 };

                if tile.iID >= 0 {
                    if self.iMaxTilesetID != -1 && tile.iID > self.iMaxTilesetID {
                        tile.iID = 0;
                    }

                    if tile.iCol < 0 || self.tilesetwidths.as_ref().is_some_and(|w| tile.iCol >= w[tile.iID as usize]) {
                        tile.iCol = 0;
                    }

                    if tile.iRow < 0 || self.tilesetheights.as_ref().is_some_and(|h| tile.iRow >= h[tile.iID as usize]) {
                        tile.iRow = 0;
                    }

                    if let Some(t) = self.translationid.as_ref() {
                        tile.iID = t[tile.iID as usize];
                    }
                }

                let r#type = TileType::from_i32(mapfile.read_i32());

                tiles.push(tile);
                types.push(r#type);
            }
        }

        (tiles, types)
    }

    pub(super) fn load_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) -> bool {
        self.read_autofilters(map, mapfile);

        if readtype == read_type_summary {
            return true;
        }

        map.clear_platforms();

        self.read_tileset(mapfile);

        self.read_tiles(map, mapfile);
        self.read_background(map, mapfile);
        self.read_switches(map, mapfile);
        self.read_platforms(map, mapfile, readtype == read_type_preview);

        self.translationid = None;
        self.tilesetwidths = None;
        self.tilesetheights = None;

        self.read_items(map, mapfile);
        self.read_hazards(map, mapfile);
        self.read_eyecandy(map, mapfile);
        self.read_music_category(map, mapfile);
        self.read_warp_locations(map, mapfile);
        self.read_switchable_blocks(map, mapfile);

        if readtype == read_type_preview {
            return true;
        }

        self.read_warp_exits(map, mapfile);
        if !self.read_spawn_areas(map, mapfile) {
            return false;
        }

        if !self.read_draw_areas(map, mapfile) {
            return false;
        }

        self.read_extra_tiledata(map, mapfile);
        self.read_gamemode_settings(map, mapfile);

        true
    }
}
