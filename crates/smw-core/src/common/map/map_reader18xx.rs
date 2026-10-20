//! Readers for 1.8 map files, which name their tilesets and add items, hazards and per-type spawn areas.

use crate::game::Game;
use crate::common::file_io::BinaryFile;
use crate::common::global_constants::*;
use crate::common::map::map_reader::{check_platform_size, check_warp, checked_count, checked_index, MapReader};
use crate::common::map::{read_type_preview, read_type_summary, CMap, MapHazard, MapItem, MapItemType, ReadType, TilesetTile, WarpEnterDirection};
use crate::common::movingplatform::MovingPlatform;
use crate::common::tile_types::TileType;

/// One entry of a 1.8 map's tileset table: the map-file ID and the tileset name it refers to.
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

    /// Reads the tileset table and builds the ID translation and size tables used by the tile readers.
    fn read_tileset(&mut self, g: &mut Game, mapfile: &mut BinaryFile) {
        let mut gp = g.ptr();
        let iNumTilesets: i16 = mapfile.read_i32() as i16;

        let mut translation: Vec<TilesetTranslation> = Vec::with_capacity(iNumTilesets.max(0) as usize);

        self.iMaxTilesetID = 0;
        for _iTileset in 0..iNumTilesets {
            let iTilesetID: i16 = mapfile.read_i32() as i16;
            checked_index("tileset ID", iTilesetID as i32, i16::MAX as i32);

            if iTilesetID > self.iMaxTilesetID {
                self.iMaxTilesetID = iTilesetID;
            }

            let szName = mapfile.read_string_long(128);
            translation.push(TilesetTranslation { iID: iTilesetID, szName });
        }

        // IDs missing from the table map to tileset 0 with size 0, which zeroes their tile coordinates.
        let n = (self.iMaxTilesetID as i32 + 1) as usize;
        let mut translationid = vec![0i16; n];
        let mut tilesetwidths = vec![0i16; n];
        let mut tilesetheights = vec![0i16; n];

        for t in &translation {
            let iID = t.iID as usize;
            translationid[iID] = g.g_tilesetmanager.index_from_name(&t.szName) as i16;

            if translationid[iID] as i32 == TILESETUNKNOWN {
                tilesetwidths[iID] = 1;
                tilesetheights[iID] = 1;
            } else {
                let ts = gp.g_tilesetmanager.tileset(g, translationid[iID] as isize as usize);
                tilesetwidths[iID] = ts.width();
                tilesetheights[iID] = ts.height();
            }
        }

        self.translationid = Some(translationid);
        self.tilesetwidths = Some(tilesetwidths);
        self.tilesetheights = Some(tilesetheights);
    }

    /// Reads every tile layer and object; out-of-range tileset IDs and tile coordinates become 0.
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
        let iNumMapItems = checked_count("item count", mapfile.read_i32(), MAXMAPITEMS);

        for _idx in 0..iNumMapItems {
            let item = MapItem { itype: mapfile.read_i32() as MapItemType, ix: mapfile.read_i32() as i16, iy: mapfile.read_i32() as i16 };
            map.mapitems.push(item);
        }
    }

    fn read_hazards(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumMapHazards = checked_count("hazard count", mapfile.read_i32(), MAXMAPHAZARDS);

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
                check_warp(map, &map.warpdata[i][j]);

                for sType in 0..NUMSPAWNAREATYPES as usize {
                    map.nospawn[sType][i][j] = mapfile.read_bool();
                }
            }
        }
    }

    /// Reads the on/off state of each switch block by tile column and row.
    fn read_switchable_blocks(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumSwitchBlockData = checked_count("switch block count", mapfile.read_i32(), MAPWIDTH * MAPHEIGHT);
        for _iBlock in 0..iNumSwitchBlockData {
            let iCol = checked_index("switch block column", mapfile.read_i8() as i32, MAPWIDTH);
            let iRow = checked_index("switch block row", mapfile.read_i8() as i32, MAPHEIGHT);

            map.objectdata[iCol][iRow].iSettings[0] = mapfile.read_i8() as i16;
        }
    }

    /// Reads each spawn type's areas; a type with no spawn size gets one default area covering the map from row 1.
    pub(super) fn read_spawn_areas_1800(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        for i in 0..NUMSPAWNAREATYPES as usize {
            map.totalspawnsize[i] = 0;
            map.numspawnareas[i] = mapfile.read_i32() as i16;
            checked_count("spawn area count", map.numspawnareas[i] as i32, MAXSPAWNAREAS);

            for m in 0..map.numspawnareas[i] as usize {
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
    }

    /// Reads per-tile object settings (powerup weights and the like) by tile column and row.
    fn read_extra_tiledata(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        let iNumExtendedDataBlocks = checked_count("tile data count", mapfile.read_i32(), MAPWIDTH * MAPHEIGHT);

        for _iBlock in 0..iNumExtendedDataBlocks {
            let iCol = checked_index("tile data column", mapfile.read_i8() as i32, MAPWIDTH);
            let iRow = checked_index("tile data row", mapfile.read_i8() as i32, MAPHEIGHT);

            let iNumSettings = checked_count("tile setting count", mapfile.read_i8() as i32, NUM_BLOCK_SETTINGS);
            for iSetting in 0..iNumSettings {
                map.objectdata[iCol][iRow].iSettings[iSetting] = mapfile.read_i8() as i16;
            }
        }
    }

    /// Reads race goal and flag base positions, in pixels.
    fn read_gamemode_settings(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.iNumRaceGoals = mapfile.read_i32() as i16;
        checked_count("race goal count", map.iNumRaceGoals as i32, MAXRACEGOALS);
        let mut j: u16 = 0;
        while (j as i32) < map.iNumRaceGoals as i32 {
            map.racegoallocations[j as usize].x = mapfile.read_i32() as i16;
            map.racegoallocations[j as usize].y = mapfile.read_i32() as i16;
            j += 1;
        }

        map.iNumFlagBases = mapfile.read_i32() as i16;
        checked_count("flag base count", map.iNumFlagBases as i32, map.flagbaselocations.len() as i32);
        let mut j: u16 = 0;
        while (j as i32) < map.iNumFlagBases as i32 {
            map.flagbaselocations[j as usize].x = mapfile.read_i32() as i16;
            map.flagbaselocations[j as usize].y = mapfile.read_i32() as i16;
            j += 1;
        }
    }

    pub(super) fn read_platforms_1800(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile, fPreview: bool) {
        map.clear_platforms();

        let iNumPlatforms = checked_count("platform count", mapfile.read_i32() as i16 as i32, MAXMAPPLATFORMS);
        map.platforms.reserve(iNumPlatforms);

        for _idx in 0..iNumPlatforms {
            let iWidth: i16 = mapfile.read_i32() as i16;
            let iHeight: i16 = mapfile.read_i32() as i16;
            check_platform_size(iWidth, iHeight);

            let (tiles, types) = self.read_platform_tiles_1800(mapfile, iWidth, iHeight);

            let mut iDrawLayer: i16 = 2;
            if self.patch_version >= 1 {
                iDrawLayer = mapfile.read_i32() as i16;
                checked_index("platform draw layer", iDrawLayer as i32, map.platformdrawlayer.len() as i32);
            }

            let iPathType: i16 = mapfile.read_i32() as i16;

            let Some(path) = self.read_platform_path_details(mapfile, iPathType, fPreview) else {
                continue;
            };

            let platform = MovingPlatform::new(g, tiles, types, iWidth, iHeight, iDrawLayer, path, fPreview);
            map.add_permanent_platform(platform);
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

    pub(super) fn load_1800(&mut self, g: &mut Game, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) {
        self.read_autofilters(map, mapfile);

        if readtype == read_type_summary {
            return;
        }

        map.clear_platforms();

        self.read_tileset(g, mapfile);

        self.read_tiles(g, map, mapfile);
        self.read_background(map, mapfile);
        self.read_switches(map, mapfile);
        self.read_platforms(g, map, mapfile, readtype == read_type_preview);

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
            return;
        }

        self.read_warp_exits(map, mapfile);
        self.read_spawn_areas(map, mapfile);
        self.read_draw_areas(map, mapfile);
        self.read_extra_tiledata(map, mapfile);
        self.read_gamemode_settings(map, mapfile);
    }
}
