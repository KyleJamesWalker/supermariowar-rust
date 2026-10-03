//! Port of src/smw/world.cpp

use crate::common::file_io::throw_runtime_error;
use crate::common::file_list::WorldMusicCategory;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{NUM_POWERUPS, NUM_WORLD_POWERUPS, PREVIEWTILESIZE, THUMBTILESIZE, TILESIZE};
use crate::common::linfunc::tokenize;
use crate::common::math::vec2::Vec2s;
use crate::common::path::strip_path_and_extension;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::version::Version;
use crate::common::world_tour_stop::{parse_tour_stop_line, reset_tour_stops, write_tour_stop_line};
use crate::globals::*;
use crate::common::util::grid::Grid;
use sdl2::sys::{SDL_Rect, SDL_Surface, SDL_UpperBlit};
use std::collections::{BTreeMap, VecDeque};

pub const WORLD_BACKGROUND_SPRITE_SET_SIZE: i16 = 60;
pub const WORLD_PATH_SPRITE_SET_SIZE: i16 = 20;

pub const WORLD_FOREGROUND_STAGE_OFFSET: i16 = 200;
pub const WORLD_WINNING_TEAM_SPRITE_OFFSET: i16 = 600;
pub const WORLD_BRIDGE_SPRITE_OFFSET: i16 = 604;
pub const WORLD_START_SPRITE_OFFSET: i16 = 608;
pub const WORLD_FOREGROUND_SPRITE_OFFSET: i16 = 700;
pub const WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET: i16 = 900;

pub static mut g_worldmap: Global<WorldMap> = Global::uninit();

/// Static-storage constructor of `WorldMap g_worldmap(0, 0);` (needs `game_values` constructed first).
pub fn init_globals() {
    unsafe { g_worldmap.init(WorldMap::new(0, 0)) };
}

fn pop_next<'a>(list: &mut VecDeque<&'a str>) -> &'a str {
    list.pop_front().unwrap_or("")
}

/// `std::from_chars` into an `int`: no leading whitespace or '+'; on failure the default is kept.
fn to_int(text: &str) -> i32 {
    let defval = 0;
    let b = text.as_bytes();
    let mut i = 0;
    let neg = !b.is_empty() && b[0] == b'-';
    if neg {
        i = 1;
    }
    let start = i;
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v * 10 + (b[i] - b'0') as i64;
        if v > i32::MAX as i64 + 1 {
            v = i64::MAX / 16;
        }
        i += 1;
    }
    if i == start {
        return defval;
    }
    let v = if neg { -v } else { v };
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
        return defval;
    }
    v as i32
}

fn pop_next_int(list: &mut VecDeque<&str>) -> i32 {
    to_int(pop_next(list))
}

/// `std::stoi`: leading whitespace, optional sign, digits; throws when nothing converts or out of range.
fn stoi(text: &str) -> i32 {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (b'\t'..=b'\r').contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v * 10 + (b[i] - b'0') as i64;
        if v > i32::MAX as i64 + 1 {
            std::panic::panic_any("stoi".to_string());
        }
        i += 1;
    }
    if i == start {
        std::panic::panic_any("stoi".to_string());
    }
    let v = if neg { -v } else { v };
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
        std::panic::panic_any("stoi".to_string());
    }
    v as i32
}

/// `static_cast<WorldMusicCategory>(int)`; values past the enum (corrupt files) become `Grass`.
fn world_music_category_from_int(v: i32) -> WorldMusicCategory {
    let v = v as u8;
    if v <= WorldMusicCategory::COUNT as u8 {
        unsafe { std::mem::transmute::<u8, WorldMusicCategory>(v) }
    } else {
        WorldMusicCategory::Grass
    }
}

#[inline]
unsafe fn blit_surface(src: *mut SDL_Surface, srcrect: *const SDL_Rect, dst: *mut SDL_Surface, dstrect: &mut SDL_Rect) {
    SDL_UpperBlit(src, srcrect, dst, dstrect);
}

#[derive(Clone, Copy, Default, Debug)]
pub struct WorldMapTile {
    //Id is used for searching for AI
    pub iID: i16,

    pub iType: i16,
    pub iBackgroundWater: i16,
    pub iBackgroundSprite: i16,
    pub iForegroundSprite: i16,

    pub iConnectionType: i16,
    pub fConnection: [bool; 4],

    pub iWarp: i16,

    pub iCompleted: i16,
    pub fAnimated: bool,

    pub iCol: i16,
    pub iRow: i16,

    pub iVehicleBoundary: i16,
}

/**********************************
* WorldMovingObject
**********************************/

#[derive(Clone, Default)]
pub struct WorldMovingObject {
    pub pos: Vec2s,
    pub currentTile: Vec2s,
    pub destTile: Vec2s,

    pub iState: i16,
    pub iDrawSprite: i16,
    pub iDrawDirection: i16,
    pub iAnimationFrame: i16,
    pub iAnimationTimer: i16,

    pub iTileSize: i16,
    pub iTileSheet: i16,

    pub _alias: Aliased,
}

impl WorldMovingObject {
    pub fn new() -> Self {
        let mut this = WorldMovingObject::default();
        this.set_position(0, 0);
        this
    }

    pub fn init(&mut self, iCol: i16, iRow: i16, iSprite: i16, iInitialDirection: i16, tilesize: i16) {
        self.iTileSize = tilesize;
        self.iTileSheet = if tilesize as i32 == TILESIZE { 0 } else { 1 };
        self.set_position(iCol, iRow);

        self.iDrawSprite = iSprite;
        self.iDrawDirection = iInitialDirection;
    }

    pub fn r#move(&mut self, iDirection: i16) {
        if iDirection == 0 {
            self.destTile.y -= 1;
            self.iState = 1;
        } else if iDirection == 1 {
            self.destTile.y += 1;
            self.iState = 2;
        } else if iDirection == 2 {
            self.destTile.x -= 1;
            self.iState = 3;
            self.iDrawDirection = 1;
        } else if iDirection == 3 {
            self.destTile.x += 1;
            self.iState = 4;
            self.iDrawDirection = 0;
        }
    }

    pub fn update(&mut self) -> bool {
        self.iAnimationTimer += 1;
        if self.iAnimationTimer > 15 {
            self.iAnimationTimer = 0;
            self.iAnimationFrame += 2;
            if self.iAnimationFrame > 2 {
                self.iAnimationFrame = 0;
            }
        }

        let ts = self.iTileSize as i32;
        if self.iState == 1 {
            self.pos.y -= 2;
            if (self.pos.y as i32) < self.destTile.y as i32 * ts {
                self.pos.y = (self.destTile.y as i32 * ts) as i16;
                self.iState = 0;
                self.currentTile.y = self.destTile.y;

                return true;
            }
        } else if self.iState == 2 {
            //down
            self.pos.y += 2;
            if self.pos.y as i32 > self.destTile.y as i32 * ts {
                self.pos.y = (self.destTile.y as i32 * ts) as i16;
                self.iState = 0;
                self.currentTile.y = self.destTile.y;

                return true;
            }
        } else if self.iState == 3 {
            //left
            self.pos.x -= 2;
            if (self.pos.x as i32) < self.destTile.x as i32 * ts {
                self.pos.x = (self.destTile.x as i32 * ts) as i16;
                self.iState = 0;
                self.currentTile.x = self.destTile.x;

                return true;
            }
        } else if self.iState == 4 {
            //right
            self.pos.x += 2;
            if self.pos.x as i32 > self.destTile.x as i32 * ts {
                self.pos.x = (self.destTile.x as i32 * ts) as i16;
                self.iState = 0;
                self.currentTile.x = self.destTile.x;

                return true;
            }
        }

        false
    }

    pub fn face_direction(&mut self, iDirection: i16) {
        self.iDrawDirection = iDirection;
    }

    pub fn set_position(&mut self, iCol: i16, iRow: i16) {
        self.pos = Vec2s::new((iCol as i32 * self.iTileSize as i32) as i16, (iRow as i32 * self.iTileSize as i32) as i16);
        self.currentTile = Vec2s::new(iCol, iRow);
        self.destTile = Vec2s::new(iCol, iRow);

        self.iState = 0;
        self.iAnimationFrame = 0;
        self.iAnimationTimer = 0;
    }
}

/**********************************
* WorldPlayer
**********************************/

#[derive(Clone, Default)]
pub struct WorldPlayer {
    pub world_moving_object: WorldMovingObject,
}
crate::impl_base!(WorldPlayer => world_moving_object: WorldMovingObject);

impl WorldPlayer {
    pub fn new() -> Self {
        Self::new_at(0, 0)
    }

    pub fn new_at(iCol: i16, iRow: i16) -> Self {
        let mut this = WorldPlayer { world_moving_object: WorldMovingObject::new() };
        this.world_moving_object.init(iCol, iRow, 0, 0, 32);
        this
    }

    pub fn draw(&self, iMapOffsetX: i16, iMapOffsetY: i16) {
        unsafe {
            rm.spr_player[self.iDrawSprite as usize][(self.iAnimationFrame + self.iDrawDirection) as usize].draw_src(
                self.pos.x as i32 + iMapOffsetX as i32,
                self.pos.y as i32 + iMapOffsetY as i32,
                &SDL_Rect { x: 0, y: 0, w: 32, h: 32 },
            );
        }
    }

    pub fn set_sprite(&mut self, iPlayer: i16) {
        unsafe {
            let p = iPlayer as usize;
            while !rm.load_menu_skin(iPlayer, game_values.skinids[p], game_values.colorids[p], true) {
                game_values.skinids[p] += 1;
                if game_values.skinids[p] as usize >= skinlist.count() {
                    game_values.skinids[p] = 0;
                }
            }
        }

        self.iDrawSprite = iPlayer;
    }
}

/**********************************
* WorldVehicle
**********************************/

#[derive(Clone)]
pub struct WorldVehicle {
    pub world_moving_object: WorldMovingObject,

    pub srcRects: [SDL_Rect; 5],

    pub iMinMoves: i16,
    pub iMaxMoves: i16,
    pub iNumMoves: i16,

    pub iActionId: i16,

    pub fEnabled: bool,

    pub fSpritePaces: bool,
    pub iPaceOffset: i16,
    pub iPaceTimer: i16,

    pub iBoundary: i16,
}
crate::impl_base!(WorldVehicle => world_moving_object: WorldMovingObject);

impl Default for WorldVehicle {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldVehicle {
    pub fn new() -> Self {
        WorldVehicle {
            world_moving_object: WorldMovingObject::new(),
            srcRects: [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 5],
            iMinMoves: 0,
            iMaxMoves: 0,
            iNumMoves: 0,
            iActionId: 0,
            fEnabled: false,
            fSpritePaces: false,
            iPaceOffset: 0,
            iPaceTimer: 0,
            iBoundary: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, iCol: i16, iRow: i16, iAction: i16, iSprite: i16, minMoves: i16, maxMoves: i16, spritePaces: bool, iInitialDirection: i16, boundary: i16, tilesize: i16) {
        self.world_moving_object.init(iCol, iRow, iSprite, iInitialDirection, tilesize);

        self.fEnabled = true;

        let mut iRectOffsetX: i16 = 0;
        let mut iRectOffsetY: i16 = 0;

        if self.iDrawSprite >= 0 && self.iDrawSprite <= 8 {
            iRectOffsetX = 0;
            iRectOffsetY = (self.iDrawSprite as i32 * tilesize as i32) as i16;
        }

        for iRect in 0..5i16 {
            self.srcRects[iRect as usize] = SDL_Rect { x: iRect as i32 * tilesize as i32 + iRectOffsetX as i32, y: iRectOffsetY as i32, w: tilesize as i32, h: tilesize as i32 };
        }

        self.iNumMoves = 0;
        self.iActionId = iAction;

        self.iMinMoves = minMoves;
        self.iMaxMoves = maxMoves;

        self.fSpritePaces = spritePaces;
        self.iPaceOffset = 0;
        self.iPaceTimer = 0;

        self.iBoundary = boundary;
    }

    pub fn r#move(&mut self) {
        self.iNumMoves = (RANDOM_INT(self.iMaxMoves as i32 - self.iMinMoves as i32 + 1) + self.iMinMoves as i32) as i16;

        if self.iNumMoves > 0 {
            self.iPaceOffset = 0;
            self.iPaceTimer = 0;
        }

        self.set_next_dest();
    }

    fn set_next_dest(&mut self) {
        if self.iState != 0 || self.iMaxMoves == 0 {
            return;
        }

        unsafe {
            let currentTile = self.currentTile;
            let tile: WorldMapTile = *g_worldmap.tiles.at(currentTile.x as usize, currentTile.y as usize);
            let iPlayerCurrentTile = g_worldmap.get_player_current_tile();

            let n = self.iNumMoves;
            self.iNumMoves -= 1;
            if n <= 0 {
                if tile.iType == 0 && iPlayerCurrentTile != currentTile && g_worldmap.num_vehicles_in_tile(currentTile) <= 1 {
                    return;
                }
            }

            //Don't allow vehicle to move forever, cap it at 10 moves over the number attempted
            if self.iNumMoves <= -10 {
                return;
            }

            //Check to see what directions are available to move
            //Can't move through doors or vehicle boundaries
            let mut iConnections: [i16; 4] = [0; 4];
            let mut iNumConnections: i16 = 0;
            let (cx, cy) = (currentTile.x, currentTile.y);
            let iBoundary = self.iBoundary;
            for iDirection in 0..4i16 {
                let mut fIsDoor = false;
                if iDirection == 0 {
                    fIsDoor = g_worldmap.is_door(cx, cy - 1) || (iBoundary != 0 && g_worldmap.get_vehicle_boundary(cx, cy - 1) == iBoundary);
                } else if iDirection == 1 {
                    fIsDoor = g_worldmap.is_door(cx, cy + 1) || (iBoundary != 0 && g_worldmap.get_vehicle_boundary(cx, cy + 1) == iBoundary);
                } else if iDirection == 2 {
                    fIsDoor = g_worldmap.is_door(cx - 1, cy) || (iBoundary != 0 && g_worldmap.get_vehicle_boundary(cx - 1, cy) == iBoundary);
                } else if iDirection == 3 {
                    fIsDoor = g_worldmap.is_door(cx + 1, cy) || (iBoundary != 0 && g_worldmap.get_vehicle_boundary(cx + 1, cy) == iBoundary);
                }

                if tile.fConnection[iDirection as usize] && !fIsDoor {
                    iConnections[iNumConnections as usize] = iDirection;
                    iNumConnections += 1;
                }
            }

            if iNumConnections > 0 {
                let iConnection = iConnections[RANDOM_INT(iNumConnections as i32) as usize];
                self.world_moving_object.r#move(iConnection);
            }
        }
    }

    pub fn update(&mut self) -> bool {
        let fMoveDone = self.world_moving_object.update();

        if fMoveDone {
            if self.currentTile == unsafe { g_worldmap.get_player_current_tile() } {
                return true;
            }

            self.set_next_dest();
        }

        //If we're done moving, start pacing in place
        if self.fSpritePaces && self.iState == 0 && {
            self.iPaceTimer += 1;
            self.iPaceTimer > 1
        } {
            self.iPaceTimer = 0;

            if self.iDrawDirection != 0 {
                self.iPaceOffset -= 1;
                if self.iPaceOffset as i32 <= -((self.iTileSize >> 1) as i32) {
                    self.iDrawDirection = 0;
                }
            } else {
                self.iPaceOffset += 1;
                if self.iPaceOffset >= (self.iTileSize >> 1) {
                    self.iDrawDirection = 1;
                }
            }
        }

        false
    }

    pub fn draw(&self, iWorldOffsetX: i16, iWorldOffsetY: i16, fVehiclesSleeping: bool) {
        unsafe {
            let ts = self.iTileSize as i32;
            if fVehiclesSleeping {
                let mut rDst = SDL_Rect { x: self.pos.x as i32 + iWorldOffsetX as i32, y: self.pos.y as i32 + iWorldOffsetY as i32, w: ts, h: ts };
                blit_surface(rm.spr_worldvehicle[self.iTileSheet as usize].get_surface(), &self.srcRects[4], blitdest, &mut rDst);
            } else {
                let mut rDst = SDL_Rect { x: self.pos.x as i32 + iWorldOffsetX as i32 + self.iPaceOffset as i32, y: self.pos.y as i32 + iWorldOffsetY as i32, w: ts, h: ts };
                blit_surface(rm.spr_worldvehicle[self.iTileSheet as usize].get_surface(), &self.srcRects[(self.iDrawDirection + self.iAnimationFrame) as usize], blitdest, &mut rDst);
            }
        }
    }
}

/**********************************
* WorldWarp
**********************************/

#[derive(Clone, Copy, Debug)]
pub struct WorldWarp {
    pub id: i16,
    pub posA: Vec2s,
    pub posB: Vec2s,
}

impl WorldWarp {
    pub fn new(id: i16, posA: Vec2s, posB: Vec2s) -> Self {
        WorldWarp { id, posA, posB }
    }

    pub fn get_other_side(&self, target: Vec2s) -> Vec2s {
        if target == self.posA {
            return self.posB;
        }
        if target == self.posB {
            return self.posA;
        }
        target
    }
}

/**********************************
* WorldMap
**********************************/

pub struct WorldMap {
    pub iWidth: i16,
    pub iHeight: i16,
    pub iStartX: i16,
    pub iStartY: i16,

    pub iNumStages: i16,

    pub tiles: Grid<WorldMapTile>,
    pub player: WorldPlayer,
    pub vehicles: Vec<WorldVehicle>,
    pub warps: Vec<WorldWarp>,

    pub iNumInitialBonuses: i16,
    pub iInitialBonuses: [i16; 32],

    pub iMusicCategory: WorldMusicCategory,

    pub iTileSize: i16,
    pub iTileSizeShift: i16,
    pub iTileSheet: i16,

    pub iLastDrawRow: i16,
    pub iLastDrawCol: i16,

    pub iTilesPerCycle: i16,

    pub worldName: String,

    pub _alias: Aliased,
}

impl WorldMap {
    /// Members without a C++ initializer start at zero.
    fn blank() -> Self {
        WorldMap {
            iWidth: 0,
            iHeight: 0,
            iStartX: 0,
            iStartY: 0,
            iNumStages: 0,
            tiles: Grid::new(0, 0),
            player: WorldPlayer::new(),
            vehicles: Vec::new(),
            warps: Vec::new(),
            iNumInitialBonuses: 0,
            iInitialBonuses: [0; 32],
            iMusicCategory: WorldMusicCategory::Grass,
            iTileSize: 0,
            iTileSizeShift: 0,
            iTileSheet: 0,
            iLastDrawRow: 0,
            iLastDrawCol: 0,
            iTilesPerCycle: 0,
            worldName: String::new(),
            _alias: Aliased::new(),
        }
    }

    pub fn new(w: i16, h: i16) -> Self {
        let mut this = Self::blank();
        this.iWidth = w;
        this.iHeight = h;
        this.tiles = Grid::new(w as usize, h as usize);

        reset_tour_stops(); // FIXME
        this
    }

    pub fn new_path(path: &str, tilesize: i16) -> Self {
        let mut this = Self::blank();
        this.reset_draw_cycle();

        this.iTileSize = tilesize;

        if this.iTileSize as i32 == TILESIZE {
            this.iTileSizeShift = 5;
            this.iTileSheet = 0;
        } else if this.iTileSize as i32 == PREVIEWTILESIZE {
            this.iTileSizeShift = 4;
            this.iTileSheet = 1;
        } else if this.iTileSize as i32 == THUMBTILESIZE {
            this.iTileSizeShift = 3;
            this.iTileSheet = 2;
        }

        this.worldName = strip_path_and_extension(path);

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => throw_runtime_error("Could not open the world file"),
        };
        let mut lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
        if bytes.last() == Some(&b'\n') || bytes.is_empty() {
            lines.pop();
        }

        reset_tour_stops();

        let mut iReadType: i16 = 0;
        let mut version = Version::default();
        let mut iMapTileReadRow: i16 = 0;
        let mut iCurrentStage: i16 = 0;
        let mut iNumWarps: i16 = 0;
        let mut iNumVehicles: i16 = 0;

        unsafe {
            for raw in lines {
                if raw.is_empty() {
                    continue;
                }

                if raw[0] == b'#' || raw[0] == b'\r' || raw[0] == b' ' || raw[0] == b'\t' {
                    continue;
                }

                let line_owned = String::from_utf8_lossy(raw).into_owned();
                let line: &str = &line_owned;

                if iReadType == 0 {
                    //Read version number
                    let mut tokens: VecDeque<&str> = tokenize(line, '.', usize::MAX).into();
                    version.major = pop_next_int(&mut tokens) as u8;
                    version.minor = pop_next_int(&mut tokens) as u8;
                    version.patch = pop_next_int(&mut tokens) as u8;
                    version.build = pop_next_int(&mut tokens) as u8;
                    iReadType = 1;
                } else if iReadType == 1 {
                    //music category
                    this.iMusicCategory = world_music_category_from_int(stoi(line)); // FIXME
                    iReadType = 2;
                } else if iReadType == 2 {
                    //world width
                    this.iWidth = stoi(line) as i16;
                    iReadType = 3;
                } else if iReadType == 3 {
                    //world height
                    this.iHeight = stoi(line) as i16;
                    iReadType = 4;

                    this.tiles = Grid::new(this.iWidth as usize, this.iHeight as usize);

                    let mut iDrawSurfaceTiles: i16 = (this.iWidth as i32 * this.iHeight as i32) as i16;

                    if iDrawSurfaceTiles > 456 {
                        iDrawSurfaceTiles = 456; //19 * 24 = 456 max tiles in world surface
                    }

                    this.iTilesPerCycle = iDrawSurfaceTiles / 8;
                } else if iReadType == 4 {
                    //background water
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    for iMapTileReadCol in 0..this.iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iBackgroundWater = pop_next_int(&mut tokens) as i16;
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 5;
                        iMapTileReadRow = 0;
                    }
                } else if iReadType == 5 {
                    //background sprites
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    let iWidth = this.iWidth;
                    for iMapTileReadCol in 0..iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iBackgroundSprite = pop_next_int(&mut tokens) as i16;
                        tile.fAnimated = (tile.iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE) != 1;

                        tile.iID = (iMapTileReadRow as i32 * iWidth as i32 + iMapTileReadCol as i32) as i16;
                        tile.iCol = iMapTileReadCol;
                        tile.iRow = iMapTileReadRow;
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 6;
                        iMapTileReadRow = 0;
                    }
                } else if iReadType == 6 {
                    //foreground sprites
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    for iMapTileReadCol in 0..this.iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iForegroundSprite = pop_next_int(&mut tokens) as i16;

                        let iForegroundSprite = tile.iForegroundSprite;

                        //Animated parts of paths
                        if !tile.fAnimated && iForegroundSprite >= 0 && iForegroundSprite <= 8 * WORLD_PATH_SPRITE_SET_SIZE {
                            let iForeground = iForegroundSprite % WORLD_PATH_SPRITE_SET_SIZE;
                            tile.fAnimated = iForeground >= 3 && iForeground <= 10;
                        }

                        //Animated 1-100 stages
                        if !tile.fAnimated {
                            tile.fAnimated = iForegroundSprite >= WORLD_FOREGROUND_STAGE_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_STAGE_OFFSET + 399;
                        }

                        //Animated foreground tiles
                        if !tile.fAnimated {
                            tile.fAnimated = iForegroundSprite >= WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET + 29;
                        }
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 7;
                        iMapTileReadRow = 0;
                    }
                } else if iReadType == 7 {
                    //path connections
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    for iMapTileReadCol in 0..this.iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iConnectionType = pop_next_int(&mut tokens) as i16;
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 8;
                        iMapTileReadRow = 0;

                        //1 == |  2 == -  3 == -!  4 == L  5 == ,-  6 == -,
                        //7 == -|  8 == -`-  9 == |-  10 == -,-  11 == +
                        //12 == horizontal bridge starts open,  13 == horizontal bridge closed
                        //14 == vertical bridge starts open,  15 == vertical bridge closed
                        for iRow in 0..this.iHeight {
                            for iCol in 0..this.iWidth {
                                this.set_tile_connections(iCol, iRow);
                            }
                        }
                    }
                } else if iReadType == 8 {
                    //stages
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    for iMapTileReadCol in 0..this.iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iType = pop_next_int(&mut tokens) as i16;
                        tile.iWarp = -1;

                        let iType = tile.iType;
                        tile.iCompleted = if iType <= 5 { -1 } else { -2 };

                        if iType == 1 {
                            this.iStartX = iMapTileReadCol;
                            this.iStartY = iMapTileReadRow;
                            let (sx, sy) = (this.iStartX, this.iStartY);
                            this.player.set_position(sx, sy);
                        }
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 9;
                        iMapTileReadRow = 0;
                    }
                } else if iReadType == 9 {
                    //vehicle boundaries
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();
                    if tokens.len() < this.iWidth as usize {
                        break;
                    }

                    for iMapTileReadCol in 0..this.iWidth {
                        let tile = this.tiles.at_mut(iMapTileReadCol as usize, iMapTileReadRow as usize);
                        tile.iVehicleBoundary = pop_next_int(&mut tokens) as i16;
                    }

                    iMapTileReadRow += 1;
                    if iMapTileReadRow == this.iHeight {
                        iReadType = 10;
                    }
                } else if iReadType == 10 {
                    //number of stages
                    this.iNumStages = stoi(line) as i16;

                    iReadType = if this.iNumStages == 0 { 12 } else { 11 };
                } else if iReadType == 11 {
                    //stage details
                    let ts = Ptr::new_box(parse_tour_stop_line(raw, &version, true));

                    game_values.tourstops.push(ts);

                    iCurrentStage += 1;
                    if iCurrentStage >= this.iNumStages {
                        //Scan stage IDs and make sure we have a stage for each one
                        let iMaxStage: i16 = (game_values.tourstops.len() + 5) as i16;
                        let mut invalid = false;
                        'scan: for iRow in 0..this.iHeight {
                            for iCol in 0..this.iWidth {
                                let iType = this.tiles.at(iCol as usize, iRow as usize).iType;
                                if iType < 0 || iType > iMaxStage {
                                    invalid = true;
                                    break 'scan;
                                }
                            }
                        }
                        if invalid {
                            break;
                        }

                        iReadType = 12;
                    }
                } else if iReadType == 12 {
                    //number of warps
                    iNumWarps = stoi(line) as i16;

                    if iNumWarps < 0 {
                        iNumWarps = 0;
                    }

                    if iNumWarps > 0 {
                        this.warps.reserve(iNumWarps as usize);
                    }

                    iReadType = if iNumWarps == 0 { 14 } else { 13 };
                } else if iReadType == 13 {
                    //warp details
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();

                    let iCol1 = 0.max(pop_next_int(&mut tokens)) as i16;
                    let iRow1 = 0.max(pop_next_int(&mut tokens)) as i16;
                    let iCol2 = 0.max(pop_next_int(&mut tokens)) as i16;
                    let iRow2 = 0.max(pop_next_int(&mut tokens)) as i16;

                    let warpId = this.warps.len() as i16;
                    this.warps.push(WorldWarp::new(warpId, Vec2s::new(iCol1, iRow1), Vec2s::new(iCol2, iRow2)));

                    this.tiles.at_mut(iCol1 as usize, iRow1 as usize).iWarp = warpId;
                    this.tiles.at_mut(iCol2 as usize, iRow2 as usize).iWarp = warpId;

                    if this.warps.len() >= iNumWarps as usize {
                        iReadType = 14;
                    }
                } else if iReadType == 14 {
                    //number of vehicles
                    iNumVehicles = stoi(line) as i16;

                    if iNumVehicles < 0 {
                        iNumVehicles = 0;
                    }

                    if iNumVehicles > 0 {
                        this.vehicles.reserve(iNumVehicles as usize);
                    }

                    iReadType = if iNumVehicles == 0 { 16 } else { 15 };
                } else if iReadType == 15 {
                    //vehicles
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();

                    let iSprite = pop_next_int(&mut tokens) as i16;

                    let mut iStage = pop_next_int(&mut tokens) as i16;
                    if iStage > this.iNumStages {
                        iStage = 0;
                    }

                    let iCol = pop_next_int(&mut tokens) as i16;
                    let iRow = pop_next_int(&mut tokens) as i16;

                    let iMinMoves = 0.max(pop_next_int(&mut tokens)) as i16;
                    let iMaxMoves = iMinMoves.max(pop_next_int(&mut tokens) as i16);

                    let fSpritePaces = pop_next_int(&mut tokens) == 1;

                    let mut iInitialDirection = pop_next_int(&mut tokens) as i16;
                    if iInitialDirection != 0 {
                        iInitialDirection = 1;
                    }

                    let iBoundary = pop_next_int(&mut tokens) as i16;

                    this.vehicles.push(WorldVehicle::new());
                    let iTileSize = this.iTileSize;
                    this.vehicles.last_mut().unwrap().init(iCol, iRow, iStage, iSprite, iMinMoves, iMaxMoves, fSpritePaces, iInitialDirection, iBoundary, iTileSize);

                    if this.vehicles.len() >= iNumVehicles as usize {
                        iReadType = 16;
                    }
                } else if iReadType == 16 {
                    //initial bonus items
                    let mut tokens: VecDeque<&str> = tokenize(line, ',', usize::MAX).into();

                    this.iNumInitialBonuses = 0;

                    while !tokens.is_empty() {
                        let token = pop_next(&mut tokens);
                        if token.is_empty() {
                            break;
                        }

                        let t0 = token.as_bytes()[0];

                        //0 indicates no initial bonuses
                        if t0 == b'0' {
                            break;
                        }

                        let mut iBonusOffset: i16 = 0;
                        if t0 == b'w' || t0 == b'W' {
                            iBonusOffset += NUM_POWERUPS as i16;
                        }

                        let mut iBonus: i16 = (to_int(token.get(1..).unwrap_or("")) + iBonusOffset as i32) as i16;
                        if iBonus < 0 || iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                            iBonus = 0;
                        }

                        if this.iNumInitialBonuses < 32 {
                            this.iInitialBonuses[this.iNumInitialBonuses as usize] = iBonus;
                            this.iNumInitialBonuses += 1;
                        } else {
                            this.iInitialBonuses[31] = iBonus;
                        }
                    }

                    iReadType = 17;
                }
            }
        }

        // RETURN:
        if iReadType != 17 {
            throw_runtime_error("Invalid world file");
        }

        this
    }

    fn set_tile_connections(&mut self, iCol: i16, iRow: i16) {
        if iCol < 0 || iRow < 0 || iCol >= self.iWidth || iRow >= self.iHeight {
            return;
        }

        let (c, r) = (iCol as usize, iRow as usize);

        for iDirection in 0..4 {
            self.tiles.at_mut(c, r).fConnection[iDirection] = false;
        }

        let ct = self.tiles.at(c, r).iConnectionType;

        if iRow > 0 {
            let t = self.tiles.at(c, r - 1).iConnectionType;

            self.tiles.at_mut(c, r).fConnection[0] = (t == 1 || t == 5 || t == 6 || t == 7 || t == 9 || t == 10 || t == 11 || t == 15)
                && (ct == 1 || ct == 3 || ct == 4 || ct == 7 || ct == 8 || ct == 9 || ct == 11 || ct == 15);
        }

        if iRow < self.iHeight - 1 {
            let t = self.tiles.at(c, r + 1).iConnectionType;

            self.tiles.at_mut(c, r).fConnection[1] = (t == 1 || t == 3 || t == 4 || t == 7 || t == 8 || t == 9 || t == 11 || t == 15)
                && (ct == 1 || ct == 5 || ct == 6 || ct == 7 || ct == 9 || ct == 10 || ct == 11 || ct == 15);
        }

        if iCol > 0 {
            let t = self.tiles.at(c - 1, r).iConnectionType;

            self.tiles.at_mut(c, r).fConnection[2] = (t == 2 || t == 4 || t == 5 || t == 8 || t == 9 || t == 10 || t == 11 || t == 13)
                && (ct == 2 || ct == 3 || ct == 6 || ct == 7 || ct == 8 || ct == 10 || ct == 11 || ct == 13);
        }

        if iCol < self.iWidth - 1 {
            let t = self.tiles.at(c + 1, r).iConnectionType;

            self.tiles.at_mut(c, r).fConnection[3] = (t == 2 || t == 3 || t == 6 || t == 7 || t == 8 || t == 10 || t == 11 || t == 13)
                && (ct == 2 || ct == 4 || ct == 5 || ct == 8 || ct == 9 || ct == 10 || ct == 11 || ct == 13);
        }
    }

    //Saves world to file
    pub fn save(&self, szPath: &str) -> bool {
        use std::fmt::Write as _;

        if std::fs::File::create(szPath).is_err() {
            return false;
        }

        let mut file = String::new();

        file += "#Version\n";
        // For compatibility, let's use the final 1.8 version until
        // there's no actual change in the world format.
        file += "1.8.0.4\n\n";

        file += "#Music Category\n";
        let _ = write!(file, "{}\n\n", self.iMusicCategory as u8 as i32);

        file += "#Width\n";
        let _ = write!(file, "{}\n\n", self.iWidth);

        file += "#Height\n";
        let _ = write!(file, "{}\n\n", self.iHeight);

        let layer = |file: &mut String, title: &str, get: &dyn Fn(&WorldMapTile) -> i16| {
            *file += title;
            for iMapTileReadRow in 0..self.iHeight {
                for iMapTileReadCol in 0..self.iWidth {
                    let tile = self.tiles.at(iMapTileReadCol as usize, iMapTileReadRow as usize);
                    let _ = write!(file, "{}", get(tile));

                    if iMapTileReadCol == self.iWidth - 1 {
                        *file += "\n";
                    } else {
                        *file += ",";
                    }
                }
            }
            *file += "\n";
        };

        layer(&mut file, "#Sprite Water Layer\n", &|t| t.iBackgroundWater);
        layer(&mut file, "#Sprite Background Layer\n", &|t| t.iBackgroundSprite);
        layer(&mut file, "#Sprite Foreground Layer\n", &|t| t.iForegroundSprite);
        layer(&mut file, "#Connections\n", &|t| t.iConnectionType);
        layer(&mut file, "#Tile Types (Stages, Doors, Start Tiles)\n", &|t| t.iType);
        layer(&mut file, "#Vehicle Boundaries\n", &|t| t.iVehicleBoundary);

        file += "#Stages\n";
        file += "#Stage Type 0,Map,Mode,Goal,Points,Bonus List(Max 10),Name,End World, then mode settings (see sample tour file for details)\n";
        file += "#Stage Type 1,Bonus House Name,Sequential/Random Order,Display Text,Powerup List(Max 5)\n";

        unsafe {
            let _ = writeln!(file, "{}", game_values.tourstops.len() as i32);

            for iStage in 0..game_values.tourstops.len() {
                let line = write_tour_stop_line(&game_values.tourstops[iStage], true);
                file += &line;
            }
        }
        file += "\n";

        file += "#Warps\n";
        file += "#location 1 x, y, location 2 x, y\n";

        let _ = writeln!(file, "{}", self.warps.len() as i32);

        for warp in &self.warps {
            let _ = write!(file, "{},", warp.posA.x);
            let _ = write!(file, "{},", warp.posA.y);
            let _ = write!(file, "{},", warp.posB.x);
            let _ = writeln!(file, "{}", warp.posB.y);
        }
        file += "\n";

        file += "#Vehicles\n";
        file += "#Sprite,Stage Type, Start Column, Start Row, Min Moves, Max Moves, Sprite Paces, Sprite Direction, Boundary\n";

        let _ = writeln!(file, "{}", self.vehicles.len() as i32);

        for v in &self.vehicles {
            let _ = write!(file, "{},", v.iDrawSprite);
            let _ = write!(file, "{},", v.iActionId);
            let _ = write!(file, "{},", v.currentTile.x);
            let _ = write!(file, "{},", v.currentTile.y);
            let _ = write!(file, "{},", v.iMinMoves);
            let _ = write!(file, "{},", v.iMaxMoves);
            let _ = write!(file, "{},", v.fSpritePaces as i32);
            let _ = write!(file, "{},", v.iDrawDirection);
            let _ = writeln!(file, "{}", v.iBoundary);
        }
        file += "\n";

        file += "#Initial Items\n";

        for iItem in 0..self.iNumInitialBonuses {
            if iItem != 0 {
                file += ",";
            }

            let mut iBonus = self.iInitialBonuses[iItem as usize];
            let mut cBonusType = 'p';
            if iBonus as i32 >= NUM_POWERUPS {
                iBonus -= NUM_POWERUPS as i16;
                cBonusType = 'w';
            }

            let _ = write!(file, "{}{}", cBonusType, iBonus);
        }

        if self.iNumInitialBonuses == 0 {
            file += "0";
        }

        file += "\n";

        if std::fs::write(szPath, file.as_bytes()).is_err() {
            return false;
        }

        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(szPath, std::fs::Permissions::from_mode(0o774));
        }

        true
    }

    pub fn clear(&mut self) {
        for row in 0..self.tiles.rows() {
            for col in 0..self.tiles.cols() {
                let iWidth = self.iWidth;
                let tile = self.tiles.at_mut(col, row);
                tile.iBackgroundSprite = 0;
                tile.iBackgroundWater = 0;
                tile.iForegroundSprite = 0;
                tile.iConnectionType = 0;
                tile.iType = 0;
                tile.iID = (row as i64 * iWidth as i64 + col as i64) as i16;
                tile.iVehicleBoundary = 0;
                tile.iWarp = 0;
            }
        }

        self.vehicles.clear();
        self.warps.clear();
    }

    //Resizes world keeping intact current tiles (if possible)
    pub fn resize(&mut self, w: i16, h: i16) {
        let iOldWidth = self.iWidth;
        let iOldHeight = self.iHeight;

        //Create new map
        self.iWidth = w;
        self.iHeight = h;
        let mut newTiles: Grid<WorldMapTile> = Grid::new(w as usize, h as usize);

        //Copy tiles to new map
        for iCol in 0..self.iWidth {
            for iRow in 0..self.iHeight {
                let newTile = newTiles.at_mut(iCol as usize, iRow as usize);
                if iCol < iOldWidth && iRow < iOldHeight {
                    *newTile = *self.tiles.at(iCol as usize, iRow as usize);
                } else {
                    newTile.iBackgroundSprite = 0;
                    newTile.iBackgroundWater = 0;
                    newTile.iForegroundSprite = 0;
                    newTile.iConnectionType = 0;
                    newTile.iType = 0;
                    newTile.iID = (iRow as i32 * self.iWidth as i32 + iCol as i32) as i16;
                    newTile.iVehicleBoundary = 0;
                    newTile.iWarp = 0;
                }
            }
        }

        // Apply the new tiles
        self.tiles = newTiles;
    }

    pub fn update(&mut self, fPlayerVehicleCollision: &mut bool) -> bool {
        let mut fPlayMovingVehicleSound = false;

        let fPlayerDoneMove = self.player.update();

        *fPlayerVehicleCollision = false;
        for i in 0..self.vehicles.len() {
            if !self.vehicles[i].fEnabled {
                continue;
            }

            let vehicle: *mut WorldVehicle = &mut self.vehicles[i];
            *fPlayerVehicleCollision |= unsafe { (*vehicle).update() };

            if self.vehicles[i].iState > 0 {
                fPlayMovingVehicleSound = true;
            }
        }

        unsafe {
            if fPlayMovingVehicleSound && !rm.sfx_boomerang.is_playing() {
                if_sound_on_play(&mut rm.sfx_boomerang);
            }
        }

        fPlayerDoneMove
    }

    pub fn draw(&self, iMapOffsetX: i16, iMapOffsetY: i16, fDrawPlayer: bool, fVehiclesSleeping: bool) {
        for vehicle in &self.vehicles {
            if !vehicle.fEnabled {
                continue;
            }

            vehicle.draw(iMapOffsetX, iMapOffsetY, fVehiclesSleeping);
        }

        if fDrawPlayer {
            self.player.draw(iMapOffsetX, iMapOffsetY);
        }
    }

    pub fn update_tile(&mut self, surface: *mut SDL_Surface, iCol: i16, iRow: i16, iMapDrawOffsetCol: i16, iMapDrawOffsetRow: i16, iAnimationFrame: i16) {
        self.draw_tile_to_surface(surface, iCol, iRow, iMapDrawOffsetCol, iMapDrawOffsetRow, true, iAnimationFrame, 0);
    }

    /// `DrawMapToSurface(SDL_Surface*)`: the whole map in one pass.
    pub fn draw_map_to_surface_full(&self, surface: *mut SDL_Surface) {
        for iRow in 0..self.iHeight {
            for iCol in 0..self.iWidth {
                self.draw_tile_to_surface(surface, iCol, iRow, 0, 0, true, 0, 0);
            }
        }
    }

    pub fn reset_draw_cycle(&mut self) {
        self.iLastDrawRow = 0;
        self.iLastDrawCol = 0;
    }

    pub fn draw_map_to_surface(&mut self, iCycleIndex: i16, fFullRefresh: bool, surface: *mut SDL_Surface, iMapDrawOffsetCol: i16, iMapDrawOffsetRow: i16, iAnimationFrame: i16) {
        let iRowEnd: i16 = if 19 + (iMapDrawOffsetRow as i32) < self.iHeight as i32 { 19 } else { self.iHeight - iMapDrawOffsetRow };
        let iColEnd: i16 = if 24 + (iMapDrawOffsetCol as i32) < self.iWidth as i32 { 24 } else { self.iWidth - iMapDrawOffsetCol };

        let mut iCounter: i16 = 0;
        let iLayer: i16 = if iCycleIndex == -1 {
            0
        } else if iCycleIndex < 8 {
            1
        } else {
            2
        };
        let mut iRow = self.iLastDrawRow;
        while iRow < iRowEnd {
            let mut iCol = self.iLastDrawCol;
            while iCol < iColEnd {
                self.draw_tile_to_surface(surface, iCol, iRow, iMapDrawOffsetCol, iMapDrawOffsetRow, fFullRefresh, iAnimationFrame, iLayer);

                if iCycleIndex >= 0 && iCycleIndex != 7 && iCycleIndex != 15 && {
                    iCounter += 1;
                    iCounter > self.iTilesPerCycle
                } {
                    // STOPDRAWING:
                    self.iLastDrawRow = iRow;
                    self.iLastDrawCol = iCol;
                    return;
                }

                iCol += 1;
            }

            self.iLastDrawCol = 0;
            iRow += 1;
        }

        self.iLastDrawRow = 0;
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tile_to_surface(&self, surface: *mut SDL_Surface, iCol: i16, iRow: i16, iMapDrawOffsetCol: i16, iMapDrawOffsetRow: i16, fFullRefresh: bool, iAnimationFrame: i16, iLayer: i16) {
        let tile = self.tiles.at((iCol as i32 + iMapDrawOffsetCol as i32) as usize, (iRow as i32 + iMapDrawOffsetRow as i32) as usize);

        if !tile.fAnimated && !fFullRefresh {
            return;
        }

        let ts = self.iTileSize as i32;
        let shift = self.iTileSizeShift as i32;
        let iAnimationFrame = iAnimationFrame as i32;
        let sheet = self.iTileSheet as usize;

        let mut r = SDL_Rect { x: iCol as i32 * ts, y: iRow as i32 * ts, w: ts, h: ts };

        let mut iBackgroundSprite = tile.iBackgroundSprite;
        let iBackgroundWater = tile.iBackgroundWater;
        let mut iForegroundSprite = tile.iForegroundSprite;

        let iBackgroundStyleOffset = (iBackgroundSprite / WORLD_BACKGROUND_SPRITE_SET_SIZE) as i32 * (4 << shift);
        let iBackgroundStyleOffset = iBackgroundStyleOffset as i16 as i32;

        iBackgroundSprite %= WORLD_BACKGROUND_SPRITE_SET_SIZE;

        unsafe {
            //The solid background tile is not animated, but all the rest are
            if iLayer != 2 {
                let bg = rm.spr_worldbackground[sheet].get_surface();
                if iBackgroundSprite == 1 {
                    let rSrc = SDL_Rect { x: ts + iBackgroundStyleOffset, y: ts, w: ts, h: ts };
                    blit_surface(bg, &rSrc, surface, &mut r);
                } else {
                    let mut rSrc = SDL_Rect { x: iAnimationFrame + ((iBackgroundWater as i32) << (2 + shift)), y: 0, w: ts, h: ts };
                    blit_surface(bg, &rSrc, surface, &mut r);

                    if iBackgroundSprite >= 2 && iBackgroundSprite <= 48 {
                        let b = iBackgroundSprite as i32;
                        if iBackgroundSprite >= 45 {
                            rSrc = SDL_Rect { x: (3 << shift) + iBackgroundStyleOffset, y: (b - 44) << shift, w: ts, h: ts };
                            blit_surface(bg, &rSrc, surface, &mut r);
                        } else if iBackgroundSprite >= 30 {
                            rSrc = SDL_Rect { x: (2 << shift) + iBackgroundStyleOffset, y: (b - 29) << shift, w: ts, h: ts };
                            blit_surface(bg, &rSrc, surface, &mut r);
                        } else if iBackgroundSprite >= 16 {
                            rSrc = SDL_Rect { x: ts + iBackgroundStyleOffset, y: (b - 14) << shift, w: ts, h: ts };
                            blit_surface(bg, &rSrc, surface, &mut r);
                        } else {
                            rSrc = SDL_Rect { x: iBackgroundStyleOffset, y: b << shift, w: ts, h: ts };
                            blit_surface(bg, &rSrc, surface, &mut r);
                        }
                    }
                }
            }

            if iLayer != 1 {
                let special = rm.spr_worldforegroundspecial[sheet].get_surface();
                if tile.iCompleted >= 0 {
                    let rSrc = SDL_Rect { x: (tile.iCompleted as i32 + 10) << shift, y: 5 << shift, w: ts, h: ts };
                    blit_surface(special, &rSrc, surface, &mut r);
                } else if iForegroundSprite >= 0 && iForegroundSprite < WORLD_FOREGROUND_STAGE_OFFSET {
                    let paths = rm.spr_worldpaths[sheet].get_surface();
                    let iPathStyle = iForegroundSprite / WORLD_PATH_SPRITE_SET_SIZE;
                    let iPathOffsetX = ((iPathStyle as i32 % 4) * (5 << shift)) as i16 as i32;
                    let iPathOffsetY = ((iPathStyle as i32 >> 2) * (10 << shift)) as i16 as i32;
                    iForegroundSprite %= WORLD_PATH_SPRITE_SET_SIZE;
                    let f = iForegroundSprite as i32;

                    if iForegroundSprite == 1 || iForegroundSprite == 2 {
                        //Non-animated straight paths
                        let rSrc = SDL_Rect { x: iPathOffsetX, y: ((f - 1) << shift) + iPathOffsetY, w: ts, h: ts };
                        blit_surface(paths, &rSrc, surface, &mut r);
                    } else if iForegroundSprite >= 3 && iForegroundSprite <= 10 {
                        //Animated paths with "coins" in them
                        let rSrc = SDL_Rect { x: iPathOffsetX + iAnimationFrame, y: ((f - 1) << shift) + iPathOffsetY, w: ts, h: ts };
                        blit_surface(paths, &rSrc, surface, &mut r);
                    } else if iForegroundSprite >= 11 && iForegroundSprite <= 18 {
                        //Non-animated straight paths over water
                        let iSpriteX = ((((f - 11) / 2) + 1) << shift) as i16 as i32;
                        let iSpriteY = (((f - 11) % 2) << shift) as i16 as i32;

                        let rSrc = SDL_Rect { x: iPathOffsetX + iSpriteX, y: iSpriteY + iPathOffsetY, w: ts, h: ts };
                        blit_surface(paths, &rSrc, surface, &mut r);
                    }
                } else if iForegroundSprite >= WORLD_FOREGROUND_STAGE_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_STAGE_OFFSET + 399 {
                    let iTileColor = (iForegroundSprite - WORLD_FOREGROUND_STAGE_OFFSET) / 100;
                    let mut rSrc = SDL_Rect { x: (10 << shift) + iAnimationFrame, y: (iTileColor as i32) << shift, w: ts, h: ts };
                    blit_surface(special, &rSrc, surface, &mut r);

                    let iTileNumber = (iForegroundSprite - WORLD_FOREGROUND_STAGE_OFFSET) % 100;
                    rSrc.x = ((iTileNumber % 10) as i32) << shift;
                    rSrc.y = ((iTileNumber / 10) as i32) << shift;
                    blit_surface(special, &rSrc, surface, &mut r);
                } else if iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 3 {
                    let rSrc = SDL_Rect { x: ((iForegroundSprite - WORLD_BRIDGE_SPRITE_OFFSET + 10) as i32) << shift, y: 7 << shift, w: ts, h: ts };
                    blit_surface(special, &rSrc, surface, &mut r);
                } else if iForegroundSprite >= WORLD_START_SPRITE_OFFSET && iForegroundSprite <= WORLD_START_SPRITE_OFFSET + 1 {
                    let rSrc = SDL_Rect { x: ((iForegroundSprite - WORLD_START_SPRITE_OFFSET + 10) as i32) << shift, y: 4 << shift, w: ts, h: ts };
                    blit_surface(special, &rSrc, surface, &mut r);
                } else if iForegroundSprite >= WORLD_FOREGROUND_SPRITE_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_SPRITE_OFFSET + 179 {
                    let iSprite = (iForegroundSprite - WORLD_FOREGROUND_SPRITE_OFFSET) as i32;
                    let rSrc = SDL_Rect { x: (iSprite % 12) << shift, y: (iSprite / 12) << shift, w: ts, h: ts };
                    blit_surface(rm.spr_worldforeground[sheet].get_surface(), &rSrc, surface, &mut r);
                } else if iForegroundSprite >= WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET + 29 {
                    let iSprite = (iForegroundSprite - WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET) as i32;
                    let rSrc = SDL_Rect { x: (if iSprite >= 15 { 16 << shift } else { 12 << shift }) + iAnimationFrame, y: (iSprite % 15) << shift, w: ts, h: ts };
                    blit_surface(rm.spr_worldforeground[sheet].get_surface(), &rSrc, surface, &mut r);
                }

                //Draw doors
                let iType = tile.iType;
                if iType >= 2 && iType <= 5 {
                    let rSrc = SDL_Rect { x: (iType as i32 + 8) << shift, y: 6 << shift, w: ts, h: ts };
                    blit_surface(special, &rSrc, surface, &mut r);
                }
            }
        }
    }

    pub fn get_world_size(&self, w: &mut i16, h: &mut i16) {
        *w = self.iWidth;
        *h = self.iHeight;
    }

    pub fn get_player_current_tile(&self) -> Vec2s {
        self.player.currentTile
    }
    pub fn get_player_dest_tile(&self) -> Vec2s {
        self.player.destTile
    }
    pub fn get_player_state(&self) -> i16 {
        self.player.iState
    }
    pub fn get_player_position(&self) -> Vec2s {
        self.player.pos
    }

    pub fn set_player_sprite(&mut self, iPlayerSprite: i16) {
        self.player.set_sprite(iPlayerSprite);
    }

    pub fn set_player_position(&mut self, iPlayerCol: i16, iPlayerRow: i16) {
        self.player.set_position(iPlayerCol, iPlayerRow);
    }

    pub fn move_player(&mut self, iDirection: i16) {
        self.player.r#move(iDirection);
    }

    pub fn face_player(&mut self, iDirection: i16) {
        self.player.face_direction(iDirection);
    }

    pub fn is_vehicle_moving(&self) -> bool {
        self.vehicles.iter().any(|vehicle| vehicle.fEnabled && vehicle.iState > 0)
    }

    pub fn get_vehicle_in_player_tile(&self, vehicleIndex: &mut i16) -> i16 {
        for i in 0..self.vehicles.len() {
            let vehicle = &self.vehicles[i];

            if !vehicle.fEnabled {
                continue;
            }

            if vehicle.currentTile == self.player.currentTile {
                *vehicleIndex = i as i16;
                return vehicle.iActionId;
            }
        }

        *vehicleIndex = -1;
        -1
    }

    pub fn move_vehicles(&mut self) {
        for i in 0..self.vehicles.len() {
            if self.vehicles[i].fEnabled {
                let vehicle: *mut WorldVehicle = &mut self.vehicles[i];
                unsafe { (*vehicle).r#move() };
            }
        }
    }

    pub fn remove_vehicle(&mut self, iVehicleIndex: i16) {
        self.vehicles[iVehicleIndex as usize].fEnabled = false;
    }

    pub fn num_vehicles_in_tile(&self, iTile: Vec2s) -> usize {
        self.vehicles.iter().filter(|vehicle| vehicle.fEnabled && vehicle.currentTile == iTile).count()
    }

    pub fn get_vehicle_stage_score(&self, iVehicleIndex: i16) -> i16 {
        unsafe { game_values.tourstops[self.vehicles[iVehicleIndex as usize].iActionId as usize].iPoints }
    }

    pub fn get_warp_in_player_tile(&self, iWarpCol: &mut i16, iWarpRow: &mut i16) -> bool {
        let iWarp = self.tiles.at(self.player.currentTile.x as usize, self.player.currentTile.y as usize).iWarp;
        if iWarp < 0 {
            return false;
        }

        let pos = self.warps[iWarp as usize].get_other_side(self.player.currentTile);
        *iWarpCol = pos.x;
        *iWarpRow = pos.y;
        true
    }

    pub fn move_bridges(&mut self) {
        for iRow in 0..self.iHeight {
            for iCol in 0..self.iWidth {
                let (c, r) = (iCol as usize, iRow as usize);
                let ct = self.tiles.at(c, r).iConnectionType;
                if ct == 12 {
                    self.tiles.at_mut(c, r).iConnectionType = 13;
                    self.set_tile_connections(iCol, iRow);
                    self.set_tile_connections(iCol - 1, iRow);
                    self.set_tile_connections(iCol + 1, iRow);
                } else if ct == 13 {
                    self.tiles.at_mut(c, r).iConnectionType = 12;
                    self.set_tile_connections(iCol, iRow);
                    self.set_tile_connections(iCol - 1, iRow);
                    self.set_tile_connections(iCol + 1, iRow);
                } else if ct == 14 {
                    self.tiles.at_mut(c, r).iConnectionType = 15;
                    self.set_tile_connections(iCol, iRow);
                    self.set_tile_connections(iCol, iRow - 1);
                    self.set_tile_connections(iCol, iRow + 1);
                } else if ct == 15 {
                    self.tiles.at_mut(c, r).iConnectionType = 14;
                    self.set_tile_connections(iCol, iRow);
                    self.set_tile_connections(iCol, iRow - 1);
                    self.set_tile_connections(iCol, iRow + 1);
                }

                let tile = self.tiles.at_mut(c, r);
                if tile.iForegroundSprite == WORLD_BRIDGE_SPRITE_OFFSET {
                    tile.iForegroundSprite = WORLD_BRIDGE_SPRITE_OFFSET + 1;
                } else if tile.iForegroundSprite == WORLD_BRIDGE_SPRITE_OFFSET + 1 {
                    tile.iForegroundSprite = WORLD_BRIDGE_SPRITE_OFFSET;
                } else if tile.iForegroundSprite == WORLD_BRIDGE_SPRITE_OFFSET + 2 {
                    tile.iForegroundSprite = WORLD_BRIDGE_SPRITE_OFFSET + 3;
                } else if tile.iForegroundSprite == WORLD_BRIDGE_SPRITE_OFFSET + 3 {
                    tile.iForegroundSprite = WORLD_BRIDGE_SPRITE_OFFSET + 2;
                }
            }
        }
    }

    pub fn is_touching_door(&self, iCol: i16, iRow: i16, doors: &mut [bool; 4]) {
        let (c, r) = (iCol as usize, iRow as usize);
        let tile = self.tiles.at(c, r);

        if iCol > 0 && tile.iCompleted >= -1 {
            let iType = self.tiles.at(c - 1, r).iType - 2;

            if iType >= 0 && iType <= 3 {
                doors[iType as usize] = true;
            }
        }

        if iCol < self.iWidth - 1 && tile.iCompleted >= -1 {
            let iType = self.tiles.at(c + 1, r).iType - 2;

            if iType >= 0 && iType <= 3 {
                doors[iType as usize] = true;
            }
        }

        if iRow > 0 && tile.iCompleted >= -1 {
            let iType = self.tiles.at(c, r - 1).iType - 2;

            if iType >= 0 && iType <= 3 {
                doors[iType as usize] = true;
            }
        }

        // The C++ compares the column against the height here.
        if iCol < self.iHeight - 1 && tile.iCompleted >= -1 {
            let iType = self.tiles.at(c, r + 1).iType - 2;

            if iType >= 0 && iType <= 3 {
                doors[iType as usize] = true;
            }
        }
    }

    pub fn is_door(&self, iCol: i16, iRow: i16) -> bool {
        if iCol >= 0 && iRow >= 0 && iCol < self.iWidth && iRow < self.iHeight {
            let iType = self.tiles.at(iCol as usize, iRow as usize).iType;
            if iType >= 2 && iType <= 5 {
                return true;
            }
        }

        false
    }

    pub fn use_key(&mut self, iKeyType: i16, iCol: i16, iRow: i16, fCloud: bool) -> i16 {
        let mut iDoorsOpened: i16 = 0;

        let (c, r) = (iCol as usize, iRow as usize);
        let iCompleted = self.tiles.at(c, r).iCompleted;

        if iCol > 0 && (iCompleted >= -1 || fCloud) && self.tiles.at(c - 1, r).iType - 2 == iKeyType {
            self.tiles.at_mut(c - 1, r).iType = 0;
            iDoorsOpened |= 1;
        }

        if iCol < self.iWidth - 1 && (iCompleted >= -1 || fCloud) && self.tiles.at(c + 1, r).iType - 2 == iKeyType {
            self.tiles.at_mut(c + 1, r).iType = 0;
            iDoorsOpened |= 2;
        }

        if iRow > 0 && (iCompleted >= -1 || fCloud) && self.tiles.at(c, r - 1).iType - 2 == iKeyType {
            self.tiles.at_mut(c, r - 1).iType = 0;
            iDoorsOpened |= 4;
        }

        if iRow < self.iHeight - 1 && (iCompleted >= -1 || fCloud) && self.tiles.at(c, r + 1).iType - 2 == iKeyType {
            self.tiles.at_mut(c, r + 1).iType = 0;
            iDoorsOpened |= 8;
        }

        iDoorsOpened
    }

    pub fn get_vehicle_boundary(&self, iCol: i16, iRow: i16) -> i16 {
        if iCol >= 0 && iRow >= 0 && iCol < self.iWidth && iRow < self.iHeight {
            return self.tiles.at(iCol as usize, iRow as usize).iVehicleBoundary;
        }

        0
    }

    //Implements breadth first search to find a stage or vehicle of interest
    pub fn get_next_interesting_move(&self, iCol: i16, iRow: i16) -> i16 {
        let currentTile = self.tiles.at(iCol as usize, iRow as usize);

        //Look for stages or vehicles, but not bonus houses
        if (currentTile.iType >= 6 && currentTile.iCompleted == -2) || self.num_vehicles_in_tile(Vec2s::new(iCol, iRow)) > 0 {
            return 4; //Signal to press select on this tile
        }

        let iCurrentId = currentTile.iID;

        let mut next: VecDeque<&WorldMapTile> = VecDeque::new();
        let mut visitedTiles: BTreeMap<i16, i16> = BTreeMap::new();
        visitedTiles.insert(currentTile.iID, -1);
        next.push_back(currentTile);

        while let Some(&tile) = next.front() {
            next.pop_front();

            //Look for stages or vehicles, but not bonus houses
            if (tile.iType >= 6 && tile.iCompleted == -2) || self.num_vehicles_in_tile(Vec2s::new(tile.iCol, tile.iRow)) > 0 {
                let mut iBackTileDirection: i16 = *visitedTiles.entry(tile.iID).or_insert(0);
                let mut iBackTileId: i16 = tile.iID;

                loop {
                    if iBackTileDirection == 0 {
                        iBackTileId -= self.iWidth;
                    } else if iBackTileDirection == 1 {
                        iBackTileId += self.iWidth;
                    } else if iBackTileDirection == 2 {
                        iBackTileId -= 1;
                    } else if iBackTileDirection == 3 {
                        iBackTileId += 1;
                    } else if iBackTileDirection == 4 {
                        let target = Vec2s::new(iBackTileId % self.iWidth, iBackTileId / self.iWidth);
                        let pos = self.warps[self.tiles.at(iCol as usize, iRow as usize).iWarp as usize].get_other_side(target);
                        iBackTileId = self.tiles.at(pos.x as usize, pos.y as usize).iID;
                    }

                    if iBackTileId == iCurrentId {
                        if iBackTileDirection == 0 || iBackTileDirection == 1 {
                            return 1 - iBackTileDirection;
                        } else if iBackTileDirection == 2 || iBackTileDirection == 3 {
                            return 5 - iBackTileDirection;
                        } else {
                            return iBackTileDirection;
                        }
                    }

                    iBackTileDirection = *visitedTiles.entry(iBackTileId).or_insert(0);
                }
            }

            for iNeighbor in 0..4usize {
                if tile.fConnection[iNeighbor] {
                    if iNeighbor == 0 && tile.iRow > 0 {
                        let topTile = self.tiles.at(tile.iCol as usize, (tile.iRow - 1) as usize);

                        //Stop at door tiles
                        if topTile.iType >= 2 && topTile.iType <= 5 {
                            continue;
                        }

                        if !visitedTiles.contains_key(&topTile.iID) {
                            visitedTiles.insert(topTile.iID, 1);
                            next.push_back(topTile);
                        }
                    } else if iNeighbor == 1 && tile.iRow < self.iHeight - 1 {
                        let bottomTile = self.tiles.at(tile.iCol as usize, (tile.iRow + 1) as usize);

                        //Stop at door tiles
                        if bottomTile.iType >= 2 && bottomTile.iType <= 5 {
                            continue;
                        }

                        if !visitedTiles.contains_key(&bottomTile.iID) {
                            visitedTiles.insert(bottomTile.iID, 0);
                            next.push_back(bottomTile);
                        }
                    } else if iNeighbor == 2 && tile.iCol > 0 {
                        let leftTile = self.tiles.at((tile.iCol - 1) as usize, tile.iRow as usize);

                        //Stop at door tiles
                        if leftTile.iType >= 2 && leftTile.iType <= 5 {
                            continue;
                        }

                        if !visitedTiles.contains_key(&leftTile.iID) {
                            visitedTiles.insert(leftTile.iID, 3);
                            next.push_back(leftTile);
                        }
                    } else if iNeighbor == 3 && tile.iCol < self.iWidth - 1 {
                        let rightTile = self.tiles.at((tile.iCol + 1) as usize, tile.iRow as usize);

                        //Stop at door tiles
                        if rightTile.iType >= 2 && rightTile.iType <= 5 {
                            continue;
                        }

                        if !visitedTiles.contains_key(&rightTile.iID) {
                            visitedTiles.insert(rightTile.iID, 2);
                            next.push_back(rightTile);
                        }
                    }
                }

                if tile.iWarp >= 0 {
                    let pos = self.warps[tile.iWarp as usize].get_other_side(Vec2s::new(tile.iCol, tile.iRow));
                    let warpTile = self.tiles.at(pos.x as usize, pos.y as usize);

                    //Stop at door tiles
                    if warpTile.iType >= 2 && warpTile.iType <= 5 {
                        continue;
                    }

                    if !visitedTiles.contains_key(&warpTile.iID) {
                        visitedTiles.insert(warpTile.iID, 4);
                        next.push_back(warpTile);
                    }
                }
            }
        }

        -1
    }

    pub fn set_initial_powerups(&self) {
        unsafe {
            for iTeam in 0..4usize {
                game_values.worldpowerupcount[iTeam] = self.iNumInitialBonuses;

                for iItem in 0..self.iNumInitialBonuses as usize {
                    game_values.worldpowerups[iTeam][iItem] = self.iInitialBonuses[iItem];
                }
            }
        }
    }

    pub fn get_music_category(&self) -> WorldMusicCategory {
        self.iMusicCategory
    }
    pub fn get_world_name(&self) -> &str {
        &self.worldName
    }
}
