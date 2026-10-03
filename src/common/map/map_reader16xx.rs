//! Port of src/common/map/MapReader16xx.cpp

use crate::common::file_io::BinaryFile;
use crate::common::game_values::default_powerup_setting;
use crate::common::global::g_tilesetmanager;
use crate::common::global_constants::*;
use crate::common::map::map_reader::MapReader;
use crate::common::map::{read_type_summary, CMap, ReadType, WarpEnterDirection, WarpExitDirection};
use crate::common::tile_types::TileType;

const g_iTileConversion: [i16; 300] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 575, 670, 702, 703, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 332, 331, 330, 637, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 365, 299, 366, 853, 537, 595, 505, 658, 659,
    656, 657, 774, 775, 776, 540, 96, 97, 98, 643, 644, 645, 26, 27, 506, 122, 123, 124, 690, 691, 688, 689, 745, 746, 747, 569, 128,
    129, 130, 704, 677, 907, 90, 91, 572, 931, 602, 539, 885, 728, 729, 730, 731, 186, 187, 188, 160, 161, 162, 736, 192, 194, 30, 31,
    898, 737, 738, 739, 800, 760, 761, 762, 763, 218, 219, 220, 864, 865, 509, 768, 224, 226, 62, 63, 930, 769, 770, 771, 508, 598, 599,
    600, 507, 601, 510, 603, 896, 897, 192, 193, 260, 259, 193, 194, 541, 627, 699, 697, 940, 941, 942, 860, 861, 862, 250, 252, 543,
    158, 498, 499, 500, 922, 924, 854, 886, 605, 125, 126, 127, 720, 721, 752, 754, 753, 722, 723, 928, 929, 563, 531, 532, 923, 571,
    882, 851, 309, 310, 311, 343, 278, 341, 99, 100, 101, 489, 490, 491, 384, 385, 386, 147, 113, 148, 914, 664, 373, 374, 375, 376,
    310, 377, 131, 132, 133, 553, 554, 555, 416, 417, 418, 179, 145, 180, 946, 570, 867, 868, 869, 213, 214, 215, 163, 164, 165, 566,
    567, 568, 448, 449, 450, 863, 530, 504, 892, 883, 899, 900, 901, 245, 246, 247, 777, 778, 779, 250, 251, 252, 856, 857, 858, 859,
    562, 712, 893, 905, 908, 909, 910, 624, 625, 626, 592, 593, 594, 282, 283, 284, 888, 889, 890, 891, 710, 711, 894, 937,
];

impl MapReader {
    pub(super) fn read_tiles_1600(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        unsafe {
            for j in 0..MAPHEIGHT as usize {
                for i in 0..MAPWIDTH as usize {
                    for k in 0..MAPLAYERS as usize {
                        let iTile: i16 = mapfile.read_i32() as i16;

                        if iTile == 300 {
                            map.mapdata[i][j][k].iID = TILESETNONE as i16;
                        } else {
                            let iTileID = g_iTileConversion[iTile as isize as usize];

                            let tile = &mut map.mapdata[i][j][k];
                            tile.iID = g_tilesetmanager.classic_tileset_index() as i16;
                            tile.iCol = iTileID % 32;
                            tile.iRow = iTileID / 32;
                        }
                    }

                    map.mapdatatop[i][j] = TileType::NonSolid;

                    for k in (0..MAPLAYERS as usize).rev() {
                        let tile = map.mapdata[i][j][k];
                        let r#type = g_tilesetmanager.classic_tileset().tile_type(tile.iCol as isize as usize, tile.iRow as isize as usize);
                        if r#type != TileType::NonSolid {
                            map.mapdatatop[i][j] = r#type;
                            break;
                        }
                    }

                    map.objectdata[i][j].iType = mapfile.read_i32() as i16;
                    if map.objectdata[i][j].iType == 6 {
                        map.objectdata[i][j].iType = -1;
                    }

                    map.objectdata[i][j].fHidden = false;

                    if map.objectdata[i][j].iType == 1 {
                        for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                            map.objectdata[i][j].iSettings[iSetting] = default_powerup_setting(0, iSetting);
                        }
                    }

                    map.warpdata[i][j].direction = mapfile.read_i32() as WarpEnterDirection;
                    map.warpdata[i][j].connection = mapfile.read_i32() as i16;
                    map.warpdata[i][j].id = mapfile.read_i32() as i16;

                    if self.parse_nospawn {
                        map.nospawn[0][i][j] = mapfile.read_i32() != 0;

                        for iType in 1..NUMSPAWNAREATYPES as usize {
                            map.nospawn[iType][i][j] = map.nospawn[0][i][j];
                        }
                    }
                }
            }
        }
    }

    pub(super) fn read_eyecandy_1600(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.eyecandy[2] = mapfile.read_i32() as i16;
    }

    pub(super) fn read_warp_exits(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) {
        map.maxConnection = 0;

        map.numwarpexits = mapfile.read_i32() as i16;
        let mut i: u16 = 0;
        while (i as i32) < map.numwarpexits as i32 && (i as i32) < MAXWARPS {
            let w = &mut map.warpexits[i as usize];
            w.direction = mapfile.read_i32() as WarpExitDirection;
            w.connection = mapfile.read_i32() as i16;
            w.id = mapfile.read_i32() as i16;
            w.x = mapfile.read_i32() as i16;
            w.y = mapfile.read_i32() as i16;

            w.lockx = mapfile.read_i32() as i16;
            w.locky = mapfile.read_i32() as i16;

            w.warpx = mapfile.read_i32() as i16;
            w.warpy = mapfile.read_i32() as i16;
            w.numblocks = mapfile.read_i32() as i16;

            if w.connection > map.maxConnection {
                map.maxConnection = w.connection;
            }
            i += 1;
        }

        let mut i: u16 = 0;
        while (i as i32) < map.numwarpexits as i32 - MAXWARPS {
            for _j in 0..10 {
                mapfile.read_i32();
            }
            i += 1;
        }

        if map.numwarpexits as i32 > MAXWARPS {
            map.numwarpexits = MAXWARPS as i16;
        }
    }

    pub(super) fn read_spawn_areas_1600(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) -> bool {
        map.totalspawnsize[0] = 0;
        map.numspawnareas[0] = mapfile.read_i32() as i16;

        if map.numspawnareas[0] as i32 > MAXSPAWNAREAS {
            println!();
            println!(" ERROR: Number of spawn areas ({}) was greater than max allowed ({})", map.numspawnareas[0], MAXSPAWNAREAS);
            return false;
        }

        for m in 0..map.numspawnareas[0].max(0) as usize {
            let a = &mut map.spawnareas[0][m];
            a.left = mapfile.read_i32() as i16;
            a.top = mapfile.read_i32() as i16;
            a.width = mapfile.read_i32() as i16;
            a.height = mapfile.read_i32() as i16;
            a.size = mapfile.read_i32() as i16;

            if self.fix_spawnareas {
                a.width -= a.left;
                a.height -= a.top;
            }

            map.totalspawnsize[0] = map.totalspawnsize[0].wrapping_add(a.size);
        }

        for i2 in 1..NUMSPAWNAREATYPES as usize {
            map.totalspawnsize[i2] = map.totalspawnsize[0];
            map.numspawnareas[i2] = map.numspawnareas[0];

            for m in 0..map.numspawnareas[0].max(0) as usize {
                let src = map.spawnareas[0][m];
                let a = &mut map.spawnareas[i2][m];
                a.left = src.left;
                a.top = src.top;
                a.width = src.width;
                a.height = src.height;
                a.size = src.size;

                if self.fix_spawnareas {
                    a.width -= a.left;
                    a.height -= a.top;
                }
            }
        }

        true
    }

    pub(super) fn read_draw_areas(&mut self, map: &mut CMap, mapfile: &mut BinaryFile) -> bool {
        map.numdrawareas = mapfile.read_i32() as i16;

        if map.numdrawareas as i32 > MAXDRAWAREAS {
            println!();
            println!(" ERROR: Number of draw areas ({}) was greater than max allowed ({})", map.numdrawareas, MAXDRAWAREAS);
            return false;
        }

        for m in 0..map.numdrawareas.max(0) as usize {
            map.drawareas[m].x = mapfile.read_i32() as i16 as i32;
            map.drawareas[m].y = mapfile.read_i32() as i16 as i32;
            map.drawareas[m].w = mapfile.read_i32() as u16 as i32;
            map.drawareas[m].h = mapfile.read_i32() as u16 as i32;
        }

        true
    }

    pub(super) fn load_1600(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) -> bool {
        self.read_autofilters(map, mapfile);

        if readtype == read_type_summary {
            return true;
        }

        map.clear_platforms();

        self.read_tiles(map, mapfile);
        self.read_background(map, mapfile);
        self.read_music_category(map, mapfile);

        self.read_eyecandy(map, mapfile);
        self.read_warp_exits(map, mapfile);

        if !self.read_spawn_areas(map, mapfile) {
            return false;
        }

        true
    }

    pub(super) fn load_1610(&mut self, map: &mut CMap, mapfile: &mut BinaryFile, readtype: ReadType) -> bool {
        if !self.load_1600(map, mapfile, readtype) {
            return false;
        }

        self.read_draw_areas(map, mapfile)
    }
}
