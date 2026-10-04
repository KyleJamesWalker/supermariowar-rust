//! Port of src/smw/objects/overmap/WO_KingOfTheHillZone.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::{MAPHEIGHT, MAPWIDTH, TILESIZE};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_kingofthehill_area, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::tile_types::{tile_flag_death_on_top, tile_flag_solid, tile_flag_solid_on_top};
use crate::globals::*;
use crate::impl_base;
use crate::smw::main::players;
use crate::smw::objects::overmap::over_map_object::{IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

const iKingOfTheHillZoneLimits: [[i16; 4]; 4] = [[0, 0, 1, 2], [0, 1, 2, 4], [0, 2, 4, 7], [0, 2, 5, 12]];

//------------------------------------------------------------------------------
// class KingOfTheHillArea (for King of the Hill mode)
//------------------------------------------------------------------------------
pub struct OMO_KingOfTheHillZone {
    pub io_over_map_object: IO_OverMapObject,

    pub playersTouching: [Ptr<CPlayer>; 4],
    pub playersTouchingCount: [i16; 4],
    pub totalTouchingPlayers: i16,

    pub iPlayerID: i16,

    pub colorID: i16,
    pub scoretimer: i16,
    pub frame: i16,
    pub relocatetimer: i16,
    pub size: i16,

    pub multiplier: i16,
    pub multipliertimer: i16,
}
impl_base!(OMO_KingOfTheHillZone => io_over_map_object: IO_OverMapObject);

impl OMO_KingOfTheHillZone {
    pub fn new(nspr: Ptr<gfxSprite>) -> Self {
        let mut this = OMO_KingOfTheHillZone {
            io_over_map_object: IO_OverMapObject::new(nspr, Vec2s::zero(), 5, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            playersTouching: [Ptr::null(); 4],
            playersTouchingCount: [0; 4],
            totalTouchingPlayers: 0,
            iPlayerID: -1,
            colorID: -1,
            scoretimer: 0,
            frame: 0,
            relocatetimer: 0,
            size: 0,
            multiplier: 1,
            multipliertimer: 0,
        };
        unsafe {
            this.size = game_values.gamemodesettings.kingofthehill.areasize;

            if this.size < 2 {
                this.size = 2;
                game_values.gamemodesettings.kingofthehill.areasize = this.size;
            }
        }

        this.iw = (TILESIZE * this.size as i32) as i16;
        this.collisionWidth = this.iw;
        this.ih = (TILESIZE * this.size as i32) as i16;
        this.collisionHeight = this.ih;

        this.objectType = object_kingofthehill_area;
        this.state = 1;

        for iPlayer in 0..4usize {
            this.playersTouching[iPlayer] = Ptr::null();
            this.playersTouchingCount[iPlayer] = 0;
        }

        this.place_area();
        this
    }

    pub fn place_area(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        self.relocatetimer = 0;
        self.colorID = -1;
        self.iPlayerID = -1;
        self.frame = 0;

        self.multiplier = 1;
        self.multipliertimer = 0;

        let mut x: i16 = 0;
        let mut y: i16 = 0;

        let size = self.size;

        unsafe {
            for iLoop in 0..64i16 {
                x = RANDOM_INT(MAPWIDTH - size as i32 + 1) as i16;
                y = RANDOM_INT(MAPHEIGHT - size as i32) as i16;

                // First move the zone down so it is sitting on atleast 1 solid tile
                let mut iFindY: i16 = (y as i32 + size as i32) as i16;
                let iOldFindY: i16 = iFindY;
                let mut fTryAgain = false;
                let mut fDone = false;

                while !fDone {
                    for iCol in 0..size {
                        let r#type: i32 = g_map.map(x as i32 + iCol as i32, iFindY as i32);
                        if (((r#type & tile_flag_solid_on_top) != 0 || (r#type & tile_flag_solid) != 0) && (r#type & tile_flag_death_on_top) == 0)
                            || !g_map.block((x as i32 + iCol as i32) as i16, iFindY).is_null()
                        {
                            fDone = true;
                            break;
                        }
                    }

                    if fDone {
                        break;
                    }

                    iFindY += 1;
                    if iFindY as i32 >= MAPHEIGHT {
                        iFindY = size;
                    }

                    if iFindY == iOldFindY {
                        // If we didn't find solid ground in that loop, look for a new place for the zone
                        fTryAgain = true;
                        break;
                    }
                }

                if fTryAgain {
                    continue;
                }

                y = (iFindY as i32 - size as i32) as i16;

                // Now verify that the area is not completely covered with solid tiles
                let mut iCountSolidTiles: i16 = 0;
                for iRow in 0..size {
                    for iCol in 0..size {
                        let tx = (x as i32 + iCol as i32) as i16;
                        let ty = (y as i32 + iRow as i32) as i16;
                        // If there is a solid tile inside the zone
                        if (g_map.map(tx as i32, ty as i32) & tile_flag_solid) != 0 || !g_map.spawn(1, tx, ty) || !g_map.block(tx, ty).is_null() {
                            iCountSolidTiles += 1;

                            // Be more picky in the first few loops, but allow solid tiles to be in
                            let limits = &iKingOfTheHillZoneLimits[(size - 2) as usize];
                            if (iLoop < 16 && iCountSolidTiles > limits[0])
                                || (iLoop < 32 && iCountSolidTiles > limits[1])
                                || (iLoop < 48 && iCountSolidTiles > limits[2])
                                || (iLoop < 63 && iCountSolidTiles > limits[3])
                            {
                                fTryAgain = true;
                                break;
                            }
                        }
                    }

                    if fTryAgain {
                        break;
                    }
                }

                if fTryAgain {
                    continue;
                }

                // Verify zone is not in a platform
                if g_map.is_in_platform_no_spawn_zone(
                    ((x as i32) << 5) as i16,
                    ((y as i32) << 5) as i16,
                    ((size as i32) << 5) as i16,
                    ((size as i32) << 5) as i16,
                ) {
                    continue;
                }

                break;
            }
        }

        self.ix = ((x as i32) << 5) as i16;
        self.iy = ((y as i32) << 5) as i16;
    }

    pub fn reset(&mut self) {
        self.iPlayerID = -1;
        self.colorID = -1;
        self.scoretimer = 0;
        self.frame = 0;
    }

    pub fn get_color_id(&self) -> i16 {
        self.colorID
    }
}

impl CObjectTrait for OMO_KingOfTheHillZone {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        let size = self.size;
        for iRow in 0..size {
            let mut iYPiece: i16 = TILESIZE as i16;
            if iRow == 0 {
                iYPiece = 0;
            }
            if iRow == size - 1 {
                iYPiece = (TILESIZE * 2) as i16;
            }

            for iCol in 0..size {
                let mut iXPiece: i16 = TILESIZE as i16;
                if iCol == 0 {
                    iXPiece = 0;
                }
                if iCol == size - 1 {
                    iXPiece = (TILESIZE * 2) as i16;
                }

                let iColX: i16 = (self.ix as i32 + ((iCol as i32) << 5)) as i16;
                let iRowX: i16 = (self.iy as i32 + ((iRow as i32) << 5)) as i16;

                if self.multiplier > 1 {
                    unsafe {
                        rm.spr_awardkillsinrow.draw_src(
                            iColX as i32 + 8,
                            iRowX as i32 + 8,
                            &SDL_Rect { x: (self.multiplier as i32 - 1) << 4, y: (self.colorID as i32) << 4, w: 16, h: 16 },
                        );
                    }
                }

                self.spr.draw_src(iColX as i32, iRowX as i32, &SDL_Rect { x: iXPiece as i32 + self.frame as i32, y: iYPiece as i32, w: TILESIZE, h: TILESIZE });
            }
        }
    }

    fn update(&mut self) {
        let mut iMax: i16 = 0;
        let mut iMaxTeam: i16 = -1;

        for iTeam in 0..4i16 {
            if self.playersTouchingCount[iTeam as usize] > iMax {
                iMax = self.playersTouchingCount[iTeam as usize];
                iMaxTeam = iTeam;
            }
        }

        if ((iMax as i32) << 1) > self.totalTouchingPlayers as i32 {
            // If the max touching player team is greater than the rest of the touching players
            self.colorID = self.playersTouching[iMaxTeam as usize].get_color_id();
            self.iPlayerID = self.playersTouching[iMaxTeam as usize].localID;
            self.frame = (((self.colorID as i32 + 1) << 5) * 3) as i16;
        } else {
            self.colorID = -1;
            self.iPlayerID = -1;
            self.frame = 0;
        }

        unsafe {
            if self.iPlayerID != -1 && !game_values.gamemode.gameover {
                // Speed of point accumulation is proportional to how many players are in zone
                self.scoretimer = (self.scoretimer as i32 + (((iMax as i32) << 1) - self.totalTouchingPlayers as i32)) as i16;

                if self.scoretimer >= game_values.pointspeed {
                    self.scoretimer = 0;
                    players[self.iPlayerID as usize].score().adjust_score(self.multiplier);
                    game_values.gamemode.check_winner(players[self.iPlayerID as usize]);

                    if game_values.gamemodesettings.kingofthehill.maxmultiplier > 1 && {
                        self.multipliertimer += 1;
                        self.multipliertimer >= 5
                    } {
                        self.multipliertimer = 0;

                        if self.multiplier < game_values.gamemodesettings.kingofthehill.maxmultiplier {
                            self.multiplier += 1;
                        }
                    }
                }
            } else {
                self.multiplier = 1;
                self.multipliertimer = 0;
            }

            if game_values.gamemodesettings.kingofthehill.relocationfrequency > 0 {
                self.relocatetimer += 1;
                if self.relocatetimer >= game_values.gamemodesettings.kingofthehill.relocationfrequency {
                    self.relocatetimer = 0;
                    self.place_area();
                }
            }
        }

        for iPlayer in 0..4usize {
            self.playersTouching[iPlayer] = Ptr::null();
            self.playersTouchingCount[iPlayer] = 0;
        }

        self.totalTouchingPlayers = 0;
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if !player.is_tanooki_statue() {
            let team = player.get_team_id() as usize;
            self.playersTouching[team] = player;
            self.playersTouchingCount[team] += 1;
            self.totalTouchingPlayers += 1;
        }
        false
    }
}

impl IO_OverMapObjectTrait for OMO_KingOfTheHillZone {
    crate::impl_over_map_object_plumbing!();
}
