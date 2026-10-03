//! Port of src/leveleditor/leveleditor.cpp

use crate::common::cmd_args as cmd;
use crate::common::editor_harness;
use crate::common::file_io::BinaryFile;
use crate::common::file_list::BackgroundList;
use crate::common::game::ensure_settings_dir;
use crate::common::game_values::{default_powerup_setting, TITLESTRING};
use crate::common::gfx::color::colors;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::{gfxSprite, SpriteBuilder};
use crate::common::gfx::{gfx_changefullscreen, gfx_flipscreen, gfx_init, gfx_settitle, gfx_show_catched_error};
use crate::common::global::g_szMusicCategoryNames;
use crate::common::global_constants::*;
use crate::common::map::{
    draw_map_hazard, draw_platform, read_type_full, CMap, MapBlock, MapHazard, MapItem, MapItemType, TilesetTile, Warp, WarpEnterDirection,
    WARP_UNDEFINED,
};
use crate::common::map_list::MapList;
use crate::common::math::vec2::Vec2f;
use crate::common::moving_platform_paths::{EllipsePath, MovingPlatformPathTrait, PlatformPathType, StraightPath, StraightPathContinuous};
use crate::common::movingplatform::MovingPlatform;
use crate::common::path::{concat, convert_path, file_exists, get_filename_from_path, get_home_directory, get_name_from_file_name, initialize_paths};
use crate::common::resource_manager::CResourceManager;
use crate::common::tile_types::{next_tile_type, prev_tile_type, TileType};
use crate::common::tileset_manager::{CTileset, CTilesetManager};
use crate::globals::*;
use crate::smw::fps_limiter::FPSLimiter;
use sdl2::sys::image::{IMG_Load, IMG_SavePNG};
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::{
    SDL_ConvertSurfaceFormat, SDL_CreateRGBSurface, SDL_Event, SDL_EventType, SDL_FillRect, SDL_FreeSurface, SDL_GetError, SDL_GetScancodeFromKey, SDL_KeyCode,
    SDL_Keycode, SDL_MapRGB, SDL_PixelFormatEnum, SDL_PollEvent, SDL_Rect, SDL_SetColorKey, SDL_Surface, SDL_UpperBlit, SDL_UpperBlitScaled, SDL_bool, SDL_Keymod,
    SDL_BUTTON_LEFT, SDL_BUTTON_MIDDLE, SDL_BUTTON_RIGHT,
};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::Path;
use std::ptr::{null, null_mut};

const MAPTITLESTRING: &str = "Level Editor";

const EDITOR_EDIT: i32 = 0;
const EDITOR_TILES: i32 = 1;
const EDITOR_QUIT: i32 = 2;
const SAVE_AS: i32 = 3;
const FIND: i32 = 4;
const CLEAR_MAP: i32 = 5;
const EDITOR_BLOCKS: i32 = 6;
const NEW_MAP: i32 = 7;
const SAVE: i32 = 8;
const EDITOR_WARP: i32 = 9;
const EDITOR_EYECANDY: i32 = 10;
const DISPLAY_HELP: i32 = 11;
const EDITOR_PLATFORM: i32 = 12;
const EDITOR_TILETYPE: i32 = 13;
const EDITOR_BACKGROUNDS: i32 = 14;
const EDITOR_MAPITEMS: i32 = 15;
const EDITOR_ANIMATION: i32 = 16;
const EDITOR_PROPERTIES: i32 = 17;
const EDITOR_MODEITEMS: i32 = 18;
const EDITOR_MAPHAZARDS: i32 = 19;

const MAX_PLATFORMS: i32 = 32;
const MAX_PLATFORM_VELOCITY: i32 = 16;
const NUMTILETYPES: i32 = 19;
const UI_PLATFORM_ROWS: i32 = 4;
const UI_PLATFORM_COLS: i32 = 8;

const MW: usize = MAPWIDTH as usize;
const MH: usize = MAPHEIGHT as usize;

const SDL_QUIT_EV: u32 = SDL_EventType::SDL_QUIT as u32;
const SDL_KEYDOWN_EV: u32 = SDL_EventType::SDL_KEYDOWN as u32;
const SDL_KEYUP_EV: u32 = SDL_EventType::SDL_KEYUP as u32;
const SDL_MOUSEBUTTONDOWN_EV: u32 = SDL_EventType::SDL_MOUSEBUTTONDOWN as u32;
const SDL_MOUSEBUTTONUP_EV: u32 = SDL_EventType::SDL_MOUSEBUTTONUP as u32;
const SDL_MOUSEMOTION_EV: u32 = SDL_EventType::SDL_MOUSEMOTION as u32;

const BUTTON_LEFT: u8 = SDL_BUTTON_LEFT as u8;
const BUTTON_MIDDLE: u8 = SDL_BUTTON_MIDDLE as u8;
const BUTTON_RIGHT: u8 = SDL_BUTTON_RIGHT as u8;

#[inline(always)]
fn k(key: SDL_KeyCode) -> SDL_Keycode {
    key as SDL_Keycode
}

/// `SDL_BUTTON(X)`
#[inline(always)]
const fn sdl_button(x: u32) -> u32 {
    1 << (x - 1)
}

const SDL_BUTTON_LMASK: u32 = sdl_button(SDL_BUTTON_LEFT);
const SDL_BUTTON_RMASK: u32 = sdl_button(SDL_BUTTON_RIGHT);

#[derive(Clone, Copy)]
pub struct EditorMapTile {
    pub tile: [TilesetTile; MAPLAYERS as usize],
    pub block: MapBlock,
    pub warp: Warp,
    pub nospawn: [bool; NUMSPAWNAREATYPES as usize],
    pub tiletype: TileType,
    pub item: i32,
}

impl EditorMapTile {
    /// Static storage: every member starts zeroed.
    fn zeroed() -> Self {
        EditorMapTile {
            tile: [TilesetTile::default(); MAPLAYERS as usize],
            block: MapBlock { iType: 0, iSettings: [0; NUM_BLOCK_SETTINGS as usize], fHidden: false },
            warp: Warp::default(),
            nospawn: [false; NUMSPAWNAREATYPES as usize],
            tiletype: TileType::NonSolid,
            item: 0,
        }
    }
}

pub static mut event: SDL_Event = unsafe { std::mem::zeroed() };

pub static mut set_type: TileType = TileType::Solid;
pub static mut set_tile_rows: i32 = 0;
pub static mut set_tile_cols: i32 = 0;
pub static mut set_tile_tileset: i32 = 0;
pub static mut set_tile_start_x: i32 = 0;
pub static mut set_tile_start_y: i32 = 0;
pub static mut set_tile_end_x: i32 = 0;
pub static mut set_tile_end_y: i32 = 0;
pub static mut set_tile_drag: bool = false;

pub static mut view_tileset_x: i32 = 0;
pub static mut view_tileset_y: i32 = 0;

pub static mut view_animated_tileset_x: i32 = 0;

pub static mut set_block: i32 = 0;
pub static mut set_block_switch_on: i32 = 0;
pub static mut set_tiletype: TileType = TileType::NonSolid;
pub static mut set_mapitem: i32 = 0;

pub static mut set_direction: i32 = 0;
pub static mut set_connection: i32 = 0;

pub static mut edit_mode: i32 = 1;
pub static mut selected_layer: i32 = 0;
pub static mut nospawn_mode: i32 = 0;

pub static mut move_mode: i32 = 0;
pub static mut move_start_x: i32 = 0;
pub static mut move_start_y: i32 = 0;
pub static mut move_offset_x: i32 = 0;
pub static mut move_offset_y: i32 = 0;
pub static mut move_nodrag: bool = false;
pub static mut move_replace: bool = true;

pub static mut move_drag_start_x: i32 = 0;
pub static mut move_drag_start_y: i32 = 0;
pub static mut move_drag_offset_x: i32 = 0;
pub static mut move_drag_offset_y: i32 = 0;

pub static mut state: i32 = 0;
pub static mut selectedtiles: [[bool; MH]; MW] = [[false; MH]; MW];
pub static mut moveselectedtiles: [[bool; MH]; MW] = [[false; MH]; MW];
pub static mut copiedtiles: Global<[[EditorMapTile; MH]; MW]> = Global::uninit();
pub static mut copiedlayer: i32 = 0;

pub static mut mouse_x: i32 = 0;
pub static mut mouse_y: i32 = 0;

pub fn bound_to_window_w(x: i32) -> i32 {
    0.max(x.min(640 - 1))
}

pub fn bound_to_window_h(y: i32) -> i32 {
    0.max(y.min(480 - 1))
}

pub fn bound_mouse_motion_coords() {
    unsafe {
        mouse_x = bound_to_window_w(event.motion.x);
        mouse_y = bound_to_window_h(event.motion.y);
    }
}

pub struct MapPlatform {
    pub tiles: Vec<TilesetTile>,
    pub types: Vec<TileType>,
    pub iVelocity: i16,
    pub iStartX: i16,
    pub iStartY: i16,
    pub iEndX: i16,
    pub iEndY: i16,

    pub iPathType: PlatformPathType,

    pub fAngle: f32,
    pub fRadiusX: f32,
    pub fRadiusY: f32,

    pub iDrawLayer: i16,

    pub rIcon: [SDL_Rect; 2],
    pub preview: *mut SDL_Surface,
    pub _alias: Aliased,
}

impl MapPlatform {
    pub fn new() -> Self {
        MapPlatform { _alias: Aliased::new(),
            tiles: vec![TilesetTile::default(); MW * MH],
            types: vec![TileType::NonSolid; MW * MH],
            iVelocity: 0,
            iStartX: 0,
            iStartY: 0,
            iEndX: 0,
            iEndY: 0,
            iPathType: PlatformPathType::Straight,
            fAngle: 0.0,
            fRadiusX: 0.0,
            fRadiusY: 0.0,
            iDrawLayer: 0,
            rIcon: [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2],
            preview: null_mut(),
        }
    }

    pub fn update_preview(&mut self) {
        unsafe {
            if self.preview.is_null() {
                self.preview = SDL_CreateRGBSurface((*screen).flags, 160, 120, (*(*screen).format).BitsPerPixel as i32, 0, 0, 0, 0);
                SDL_SetColorKey(self.preview, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*self.preview).format, 255, 0, 255));
            }

            SDL_FillRect(self.preview, null(), SDL_MapRGB((*self.preview).format, 255, 0, 255));

            for iPlatformX in 0..MAPWIDTH as i16 {
                for iPlatformY in 0..MAPHEIGHT as i16 {
                    let tile = self.tiles[iPlatformX as usize * MH + iPlatformY as usize];

                    let mut bltrect = SDL_Rect { x: (iPlatformX as i32) << 3, y: (iPlatformY as i32) << 3, w: THUMBTILESIZE, h: THUMBTILESIZE };
                    if tile.iID >= 0 {
                        let src = g_tilesetmanager.rect(2, tile.iCol, tile.iRow);
                        SDL_UpperBlit(g_tilesetmanager.tileset(tile.iID as usize).surface(2), src, self.preview, &mut bltrect);
                    } else if tile.iID as i32 == TILESETANIMATED {
                        let src = g_tilesetmanager.rect(2, (tile.iCol as i32 * 4) as i16, tile.iRow);
                        SDL_UpperBlit(rm.spr_tileanimation[2].get_surface(), src, self.preview, &mut bltrect);
                    } else if tile.iID as i32 == TILESETUNKNOWN {
                        //Draw unknown tile
                        let src = g_tilesetmanager.rect(2, 0, 0);
                        SDL_UpperBlit(rm.spr_unknowntile[2].get_surface(), src, self.preview, &mut bltrect);
                    }
                }
            }
        }
    }
}

impl Drop for MapPlatform {
    fn drop(&mut self) {
        if !self.preview.is_null() {
            unsafe { SDL_FreeSurface(self.preview) };
        }
    }
}

pub static mut animatedtiletypes: Vec<TileType> = Vec::new();

/// `SDL_GetKeyboardState(NULL)`, which the replay harness replaces while it is active.
fn get_keyboard_state() -> *const u8 {
    editor_harness::keyboard_state()
}

/// `SDL_GetMouseState(NULL, NULL)`, which the replay harness replaces while it is active.
fn get_mouse_state() -> u32 {
    editor_harness::mouse_state()
}

pub fn check_key(keystate: *const u8, key: SDL_KeyCode) -> bool {
    unsafe { *keystate.add(SDL_GetScancodeFromKey(key as SDL_Keycode) as usize) != 0 }
}

pub static mut s_platform: *mut SDL_Surface = null_mut();
pub static mut s_platformpathbuttons: *mut SDL_Surface = null_mut();
pub static mut s_maphazardbuttons: *mut SDL_Surface = null_mut();

pub static mut viewblocks: bool = true;
pub static mut view_only_layer: bool = false;
pub static mut viewwarps: bool = true;
pub static mut ignoreclick: bool = false;

pub static mut findstring: String = String::new();

pub static mut g_iNumPlatforms: i16 = 0;
pub static mut g_Platforms: Global<Vec<MapPlatform>> = Global::uninit();

pub static mut backgroundlist: Ptr<BackgroundList> = Ptr::null();
pub static mut g_musiccategorydisplaytimer: i16 = 0;

pub static mut g_messagedisplaytimer: i16 = 0;
pub static mut g_szMessageTitle: String = String::new();
pub static mut g_szMessageLine: [String; 3] = [String::new(), String::new(), String::new()];

pub fn set_tileset_tile(tile: &mut TilesetTile, iTileset: i16, iCol: i16, iRow: i16) {
    tile.iID = iTileset;
    tile.iCol = iCol;
    tile.iRow = iRow;
}

pub fn clear_tileset_tile(tile: &mut TilesetTile) {
    set_tileset_tile(tile, TILESETNONE as i16, 0, 0);
}

pub static mut g_fFullScreen: bool = false;

fn load_surface(path: &str) -> *mut SDL_Surface {
    let c = CString::new(path).unwrap();
    unsafe { IMG_Load(c.as_ptr()) }
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

//main main main
pub fn main() {
    crate::globals::init_globals();
    unsafe {
        copiedtiles.init([[EditorMapTile::zeroed(); MH]; MW]);
        g_Platforms.init((0..MAX_PLATFORMS).map(|_| MapPlatform::new()).collect());
    }

    let argv: Vec<String> = std::env::args().collect();
    let cmd = cmd::parse_args(&argv);
    if !cmd.success {
        std::process::exit(1);
    }
    if cmd.show_help {
        let title = format!("{} {}", TITLESTRING, MAPTITLESTRING);
        cmd::print_help(&title, "");
        return;
    }
    if cmd.debug {
        cmd::show_windows_console();
    }

    if !cmd.data_root.is_empty() {
        unsafe { RootDataDirectory = cmd.data_root.clone() };
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around inner_main().
    let result = std::panic::catch_unwind(inner_main);
    if let Err(payload) = result {
        let what = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        gfx_show_catched_error(&what);
        std::process::exit(1);
    }
}

pub fn inner_main() {
    unsafe {
        ensure_settings_dir();

        /* This must occur before any data files are loaded */
        initialize_paths();

        let mut done: bool;

        println!("-------------------------------------------------------------------------------");
        println!(" {} {}", TITLESTRING, MAPTITLESTRING);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        {
            let options_path = get_home_directory() + "leveleditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "rb");
            if editor_settings.is_open() {
                g_fFullScreen = editor_settings.read_bool();
                findstring = editor_settings.read_string_long(FILEBUFSIZE as usize);
            }
        }

        gfx_init(640, 480, g_fFullScreen);
        blitdest = screen;

        rm = Ptr::new_box(CResourceManager::new());
        g_tilesetmanager = Ptr::new_box(CTilesetManager::new());
        g_tilesetmanager.init(&convert_path("gfx/Classic/tilesets"));
        g_map = Ptr::from_box(CMap::new());
        filterslist = Ptr::new_box(crate::common::file_list::FiltersList::new());
        maplist = Ptr::new_box(MapList::new(false));
        backgroundlist = Ptr::new_box(BackgroundList::new());

        //Add all of the maps that are world only so we can edit them
        maplist.add_world_maps();
        let title = format!("{} {}", TITLESTRING, MAPTITLESTRING);
        gfx_settitle(&title);

        println!("\n---------------- loading graphics ----------------");

        rm.spr_tiletypes = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_tile_types.png")).without_color_key().create();
        rm.spr_transparenttiles = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_transparent_tiles.png")).with_alpha(160).create();

        rm.spr_backgroundlevel = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_background_levels.png")).create();
        rm.spr_tilesetlevel = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_tileset_levels.png")).create();

        rm.spr_eyecandy = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_eyecandy.png")).create();

        s_platform = load_surface(&convert_path("gfx/leveleditor/leveleditor_platform.png"));
        s_platformpathbuttons = load_surface(&convert_path("gfx/leveleditor/leveleditor_pathtype_buttons.png"));
        s_maphazardbuttons = load_surface(&convert_path("gfx/leveleditor/leveleditor_maphazard_buttons.png"));

        rm.spr_warps[0] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_warp.png")).create();
        rm.spr_warps[1] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_warp_preview.png")).create();
        rm.spr_warps[2] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_warp_thumbnail.png")).create();

        rm.spr_platformpath = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_platform_path.png")).with_alpha(128).create();

        rm.spr_selectedtile = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_selectedtile.png")).with_color_key(colors::BLACK).with_alpha(128).create();
        rm.spr_nospawntile = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_nospawntile.png")).with_color_key(colors::BLACK).with_alpha(128).create();
        rm.spr_noitemspawntile = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_noitemspawntile.png")).with_color_key(colors::BLACK).with_alpha(128).create();
        rm.spr_platformstarttile = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_platformstarttile.png")).with_color_key(colors::BLACK).with_alpha(64).with_wrapping(640).create();
        rm.spr_platformendtile = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_selectedtile.png")).with_color_key(colors::BLACK).with_alpha(64).with_wrapping(640).create();

        rm.spr_mapitems[0] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_mapitems.png")).create();
        rm.spr_mapitems[1] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_mapitems_preview.png")).create();
        rm.spr_mapitems[2] = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_mapitems_thumbnail.png")).create();

        rm.spr_dialog = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_dialog.png")).with_alpha(255).create();
        rm.menu_shade = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_shade.png")).with_alpha(128).create();

        rm.spr_tileanimation[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/tile_animation.png")).create();
        rm.spr_tileanimation[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/tile_animation_preview.png")).create();
        rm.spr_tileanimation[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/tile_animation_thumbnail.png")).create();

        rm.spr_blocks[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/blocks.png")).create();
        rm.spr_blocks[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/blocks_preview.png")).create();
        rm.spr_blocks[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/blocks_thumbnail.png")).create();

        rm.spr_unknowntile[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/unknown_tile.png")).create();
        rm.spr_unknowntile[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/unknown_tile_preview.png")).create();
        rm.spr_unknowntile[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/tilesets/unknown_tile_thumbnail.png")).create();

        rm.spr_powerups = SpriteBuilder::new(convert_path("gfx/packs/Classic/powerups/large.png")).create();
        rm.spr_powerupselector = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_powerup_selector.png")).with_alpha(128).create();
        rm.spr_hidden_marker = SpriteBuilder::new(convert_path("gfx/leveleditor/leveleditor_hidden_marker.png")).create();

        rm.spr_flagbases = SpriteBuilder::new(convert_path("gfx/packs/Classic/modeobjects/flagbases.png")).create();
        rm.spr_racegoals = SpriteBuilder::new(convert_path("gfx/packs/Classic/modeobjects/racegoal.png")).create();

        rm.spr_hazard_fireball[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/fireball.png")).create();
        rm.spr_hazard_fireball[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/fireball_preview.png")).create();
        rm.spr_hazard_fireball[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/fireball_thumbnail.png")).create();

        rm.spr_hazard_rotodisc[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/rotodisc.png")).create();
        rm.spr_hazard_rotodisc[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/rotodisc_preview.png")).create();
        rm.spr_hazard_rotodisc[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/rotodisc_thumbnail.png")).create();

        rm.spr_hazard_bulletbill[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/bulletbill.png")).create();
        rm.spr_hazard_bulletbill[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/bulletbill_preview.png")).create();
        rm.spr_hazard_bulletbill[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/bulletbill_thumbnail.png")).create();

        rm.spr_hazard_flame[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/flame.png")).create();
        rm.spr_hazard_flame[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/flame_preview.png")).create();
        rm.spr_hazard_flame[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/flame_thumbnail.png")).create();

        rm.spr_hazard_pirhanaplant[0] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/pirhanaplant.png")).create();
        rm.spr_hazard_pirhanaplant[1] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/pirhanaplant_preview.png")).create();
        rm.spr_hazard_pirhanaplant[2] = SpriteBuilder::new(convert_path("gfx/packs/Classic/hazards/pirhanaplant_thumbnail.png")).create();

        rm.spr_number_icons = SpriteBuilder::new(convert_path("gfx/packs/Classic/awards/killsinrownumbers.png")).create();

        for i in 0..3usize {
            rm.spr_hazard_fireball[i].set_wrap((640 >> i) as i16);
            rm.spr_hazard_rotodisc[i].set_wrap((640 >> i) as i16);
            rm.spr_hazard_flame[i].set_wrap((640 >> i) as i16);
            rm.spr_hazard_pirhanaplant[i].set_wrap((640 >> i) as i16);
        }
        if SDL_SetColorKey(s_platform, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*s_platform).format, 255, 0, 255)) < 0 {
            println!("\n ERROR: Couldn't set ColorKey + RLE: {}", sdl_error());
        }

        if SDL_SetColorKey(s_platformpathbuttons, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*s_platformpathbuttons).format, 255, 0, 255)) < 0 {
            println!("\n ERROR: Couldn't set ColorKey + RLE: {}", sdl_error());
        }

        if SDL_SetColorKey(s_maphazardbuttons, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*s_maphazardbuttons).format, 255, 0, 255)) < 0 {
            println!("\n ERROR: Couldn't set ColorKey + RLE: {}", sdl_error());
        }

        rm.menu_font_small = gfxFont::from_path(&convert_path("gfx/packs/Classic/fonts/font_small.png"));
        rm.menu_font_large = gfxFont::from_path(&convert_path("gfx/packs/Classic/fonts/font_large.png"));

        println!("\n---------------- load map ----------------");

        //Setup Platforms
        const UI_PLATFORM_START_X: i32 = (640 - UI_PLATFORM_COLS * 32 - (UI_PLATFORM_COLS - 1) * 10/* spacing */) / 2;
        const UI_PLATFORM_START_Y: i32 = (480 - UI_PLATFORM_ROWS * 32 - (UI_PLATFORM_ROWS - 1) * 10/* spacing */) / 2;
        for iPlatform in 0..MAX_PLATFORMS as usize {
            let p = &mut g_Platforms[iPlatform];
            p.rIcon[0].x = (iPlatform as i32 % 8) * 32;
            p.rIcon[0].y = (iPlatform as i32 / 8) * 32 + 224;
            p.rIcon[0].w = 32;
            p.rIcon[0].h = 32;

            p.rIcon[1].x = (iPlatform as i32 % 8) * 42 + UI_PLATFORM_START_X;
            p.rIcon[1].y = (iPlatform as i32 / 8) * 42 + UI_PLATFORM_START_Y;
            p.rIcon[1].w = 32;
            p.rIcon[1].h = 32;

            for iCol in 0..MW {
                for iRow in 0..MH {
                    clear_tileset_tile(&mut p.tiles[iCol * MH + iRow]);
                    p.types[iCol * MH + iRow] = TileType::NonSolid;
                }
            }

            p.iVelocity = 4;
            p.iDrawLayer = 2; //Default to drawing in the middle layer
        }

        read_animated_tile_type_file(&convert_path("gfx/packs/Classic/tilesets/tile_animation_tileset.tls"));

        let fs = findstring.clone();
        maplist.find(&fs);
        loadcurrentmap();
        findstring.clear(); //clear out the find string so that pressing "f" will give you the find dialog

        println!("\n---------------- ready, steady, go! ----------------");

        resetselectedtiles();

        crate::common::game_values::CGameValues::init(&mut game_values); // Needed for FPSLimiter

        editor_harness::init();
        editor_harness::set_dumper(dump_editor_state);

        println!("entering level editor loop...");
        done = false;
        while !done {
            FPSLimiter::instance().frame_start();

            match state {
                EDITOR_EDIT => {
                    move_nodrag = false;
                    state = editor_edit();
                }
                EDITOR_TILES => state = editor_tiles(),
                EDITOR_BLOCKS => state = editor_blocks(),
                EDITOR_MAPITEMS => state = editor_mapitems(),
                EDITOR_MODEITEMS => state = editor_modeitems(),
                EDITOR_WARP => state = editor_warp(),
                EDITOR_EYECANDY => state = editor_eyecandy(),
                EDITOR_PLATFORM => state = editor_platforms(),
                EDITOR_MAPHAZARDS => state = editor_maphazards(),
                EDITOR_TILETYPE => state = editor_tiletype(),
                EDITOR_BACKGROUNDS => state = editor_backgrounds(),
                EDITOR_ANIMATION => state = editor_animation(),
                EDITOR_QUIT => done = true,
                DISPLAY_HELP => state = display_help(),
                SAVE_AS => state = save_as(),
                FIND => state = find(),
                CLEAR_MAP => state = clear_map(),
                NEW_MAP => state = newmap(),
                SAVE => state = savecurrentmap(),
                _ => println!(" PANIC: WEIRD STATE: {}", state),
            }

            editor_harness::frame_delay(0);
            FPSLimiter::instance().before_flip();
            gfx_flipscreen();
            FPSLimiter::instance().after_flip();
        }

        println!("\n---------------- save map ----------------");

        save_map(&convert_path("maps/ZZleveleditor.map"));

        {
            let options_path = get_home_directory() + "leveleditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "wb");
            if editor_settings.is_open() {
                editor_settings.write_bool(g_fFullScreen);
                let current = maplist.current_filename().to_string();
                editor_settings.write_string_long(&current);
            }
        }

        write_animated_tile_type_file(&convert_path("gfx/packs/Classic/tilesets/tile_animation_tileset.tls"));
        g_tilesetmanager.save_tilesets();

        println!("\n---------------- shutdown ----------------");
    }
}

pub fn calculate_tile_type(x: i16, y: i16) -> TileType {
    unsafe {
        let mut r#type = TileType::NonSolid;
        let mut k: i16 = MAPLAYERS as i16 - 1;
        while k >= 0 {
            let tile = g_map.mapdata[x as usize][y as usize][k as usize];

            let mut iTileType = TileType::NonSolid;
            if tile.iID >= 0 {
                iTileType = g_tilesetmanager.tileset(tile.iID as usize).tile_type(tile.iCol as usize, tile.iRow as usize);
            } else if tile.iID as i32 == TILESETANIMATED {
                iTileType = animatedtiletypes[(tile.iRow as i32 + ((tile.iCol as i32) << 5)) as usize];
            }

            if iTileType != TileType::NonSolid {
                r#type = iTileType;
                break;
            }
            k -= 1;
        }

        r#type
    }
}

pub fn update_tile_type(x: i16, y: i16) {
    unsafe {
        g_map.mapdatatop[x as usize][y as usize] = calculate_tile_type(x, y);
    }
}

pub fn tile_type_is_modified(x: i16, y: i16) -> bool {
    unsafe { g_map.mapdatatop[x as usize][y as usize] != calculate_tile_type(x, y) }
}

pub fn adjust_map_items(x: i16, y: i16) {
    const IGNORED_TILES: [TileType; 3] = [TileType::NonSolid, TileType::SolidOnTop, TileType::IceOnTop];

    unsafe {
        if let Some(it) = g_map.mapitems.iter().position(|item: &MapItem| item.ix == x && item.iy == y) {
            let tiletype = g_map.mapdatatop[x as usize][y as usize];
            let objtype = g_map.objectdata[x as usize][y as usize].iType;

            let tiletypeValid = !IGNORED_TILES.contains(&tiletype);
            let objtypeValid = objtype != -1;
            if tiletypeValid || objtypeValid {
                g_map.mapitems.remove(it);
            }
        }
    }
}

/// Removes map items located at the XY position.
pub fn remove_map_item_at(x: i16, y: i16) {
    unsafe {
        g_map.mapitems.retain(|item: &MapItem| !(item.ix == x && item.iy == y));
    }
}

pub static mut fExiting: bool = false;
pub static mut fSelectedYes: bool = false;

pub static mut editor_edit_initialized: bool = false;
pub fn init_editor_edit() {
    unsafe {
        if editor_edit_initialized {
            return;
        }

        fExiting = false;
        fSelectedYes = false;
        g_musiccategorydisplaytimer = 0;

        editor_edit_initialized = true;
    }
}

/// The block-painting branch shared by the button-down and motion handlers of `editor_edit`.
unsafe fn paint_block(iClickX: i16, iClickY: i16) {
    let b = &mut g_map.objectdata[iClickX as usize][iClickY as usize];
    b.iType = set_block as i16;
    b.fHidden = false;

    if set_block == 1 || set_block == 15 {
        for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
            g_map.objectdata[iClickX as usize][iClickY as usize].iSettings[iSetting] = default_powerup_setting(0, iSetting);
        }
    } else if (11..=14).contains(&set_block) {
        g_map.objectdata[iClickX as usize][iClickY as usize].iSettings[0] = set_block_switch_on as i16;
    }
}

pub fn editor_edit() -> i32 {
    init_editor_edit();

    unsafe {
        if fExiting {
            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                if event.type_ == SDL_KEYDOWN_EV {
                    let key = event.key.keysym.sym;

                    if key == k(SDLK_LEFT) {
                        fSelectedYes = true;
                    } else if key == k(SDLK_RIGHT) {
                        fSelectedYes = false;
                    } else if event.key.keysym.sym == k(SDLK_KP_ENTER) || event.key.keysym.sym == k(SDLK_RETURN) {
                        if fSelectedYes {
                            return EDITOR_QUIT;
                        }

                        fExiting = false;
                    }
                }
            }
        } else {
            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                let keystate = get_keyboard_state();

                match event.type_ {
                    SDL_QUIT_EV => return EDITOR_QUIT,

                    SDL_KEYDOWN_EV => {
                        let key = event.key.keysym.sym;

                        if key == k(SDLK_ESCAPE) {
                            if g_musiccategorydisplaytimer > 0 {
                                g_musiccategorydisplaytimer = 0;
                            } else if edit_mode != 1 {
                                edit_mode = 1;
                            } else {
                                fSelectedYes = false;
                                fExiting = true;
                            }
                        }

                        if event.key.keysym.mod_ & (SDL_Keymod::KMOD_LALT as u16 | SDL_Keymod::KMOD_RALT as u16) != 0 {
                            if event.key.keysym.sym == k(SDLK_RETURN) {
                                g_fFullScreen = !g_fFullScreen;
                                gfx_changefullscreen(g_fFullScreen);
                                blitdest = screen;
                            }
                        }

                        if key >= k(SDLK_1) && key <= k(SDLK_4) {
                            if edit_mode == 4 {
                                //no spawn zones
                                nospawn_mode = key - k(SDLK_1) + 1;
                            }
                        }

                        if key == k(SDLK_INSERT) {
                            takescreenshot();
                        }

                        if key == k(SDLK_t) {
                            editor_edit_initialized = false;
                            return EDITOR_TILES;
                        }

                        if key == k(SDLK_i) {
                            if edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        let v = !g_map.nospawn[nospawn_mode as usize][iCol as usize][iRow as usize];
                                        set_no_spawn(nospawn_mode as i16, iCol, iRow, v);
                                    }
                                }
                            } else {
                                editor_edit_initialized = false;
                                return EDITOR_BLOCKS;
                            }
                        }

                        if key == k(SDLK_a) {
                            if edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        set_no_spawn(nospawn_mode as i16, iCol, iRow, true);
                                    }
                                }
                            } else {
                                editor_edit_initialized = false;
                                return EDITOR_ANIMATION;
                            }
                        }

                        if key == k(SDLK_o) {
                            editor_edit_initialized = false;
                            return EDITOR_MAPITEMS;
                        }

                        if key == k(SDLK_j) {
                            editor_edit_initialized = false;
                            return EDITOR_MODEITEMS;
                        }

                        if key == k(SDLK_h) {
                            editor_edit_initialized = false;
                            return EDITOR_MAPHAZARDS;
                        }

                        if key == k(SDLK_k) {
                            let iMouseX = mouse_x / TILESIZE;
                            let iMouseY = mouse_y / TILESIZE;

                            let iType = g_map.objectdata[iMouseX as usize][iMouseY as usize].iType;
                            if iType == 1 || iType == 15 || iType == 4 || iType == 5 || iType == 17 || iType == 18 || iType == 3 {
                                editor_properties(iMouseX as i16, iMouseY as i16);
                            }
                        }

                        //if 'B' is pressed, rotate backgrounds
                        if key == k(SDLK_b) {
                            editor_edit_initialized = false;
                            return EDITOR_BACKGROUNDS;
                        }

                        if key == k(SDLK_g) {
                            backgroundlist.next();

                            rm.spr_background = SpriteBuilder::new(backgroundlist.current_path()).without_color_key().create();
                            g_map.szBackgroundFile = get_filename_from_path(&backgroundlist.current_path().to_string_lossy());

                            if !check_key(keystate, SDLK_LSHIFT) && !check_key(keystate, SDLK_RSHIFT) {
                                //Set music to background default
                                set_music_category_from_background();
                            }
                        }

                        if key == k(SDLK_r) {
                            if g_musiccategorydisplaytimer > 0 && {
                                g_map.musicCategoryID += 1;
                                g_map.musicCategoryID as i32 >= MAXMUSICCATEGORY
                            } {
                                g_map.musicCategoryID = 0;
                            }

                            g_musiccategorydisplaytimer = 90;
                        }

                        if key == k(SDLK_s) {
                            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                                return SAVE_AS;
                            }

                            return SAVE;
                        }

                        if key == k(SDLK_f) {
                            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) || findstring.is_empty() {
                                return FIND;
                            }

                            findcurrentstring();
                        }

                        if key == k(SDLK_DELETE) && (check_key(keystate, SDLK_LCTRL) || check_key(keystate, SDLK_RCTRL)) {
                            return CLEAR_MAP;
                        }

                        if key == k(SDLK_DELETE) || key == k(SDLK_BACKSPACE) {
                            if edit_mode == 3 {
                                clearselectedmaptiles();
                                resetselectedtiles();
                            } else if edit_mode == 6 {
                                for iRow in 0..MH {
                                    for iCol in 0..MW {
                                        g_map.mapdatatop[iCol][iRow] = TileType::NonSolid;
                                    }
                                }
                            }
                        }

                        if key == k(SDLK_n) {
                            if edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        set_no_spawn(nospawn_mode as i16, iCol, iRow, false);
                                    }
                                }
                            } else {
                                return NEW_MAP;
                            }
                        }

                        if key == k(SDLK_v) {
                            viewblocks = !viewblocks;
                        }

                        if key == k(SDLK_e) {
                            editor_edit_initialized = false;
                            return EDITOR_EYECANDY;
                        }

                        if key == k(SDLK_w) {
                            editor_edit_initialized = false;
                            return EDITOR_WARP;
                        }

                        if key == k(SDLK_l) {
                            editor_edit_initialized = false;
                            return EDITOR_TILETYPE;
                        }

                        if key == k(SDLK_F1) {
                            return DISPLAY_HELP;
                        }

                        if key == k(SDLK_PAGEUP) {
                            loop {
                                maplist.prev(false);
                                if maplist.is_valid() {
                                    break;
                                }
                            }

                            loadcurrentmap();
                        }

                        if key == k(SDLK_PAGEDOWN) {
                            loop {
                                maplist.next(false);
                                if maplist.is_valid() {
                                    break;
                                }
                            }

                            loadcurrentmap();
                        }

                        if key == k(SDLK_y) {
                            selected_layer += 1;
                            if selected_layer >= MAPLAYERS {
                                selected_layer = 0;
                            }
                        }

                        if key == k(SDLK_u) {
                            view_only_layer = !view_only_layer;
                        }

                        if key == k(SDLK_m) {
                            if edit_mode == 3 {
                                move_replace = !move_replace;
                            }

                            edit_mode = 3;
                        }

                        if key == k(SDLK_x) {
                            edit_mode = 4;
                            nospawn_mode = 0;
                        }

                        if key == k(SDLK_z) {
                            edit_mode = 5;
                        }

                        if key == k(SDLK_LCTRL) {
                            move_nodrag = true;
                        }

                        if key == k(SDLK_p) {
                            editor_edit_initialized = false;
                            return EDITOR_PLATFORM;
                        }

                        if key == k(SDLK_c) {
                            if edit_mode == 3 {
                                if copyselectedtiles() {
                                    move_mode = 3;
                                    getcenterselection(&mut move_start_x, &mut move_start_y);
                                }
                            }
                        }
                    }

                    SDL_KEYUP_EV => {
                        if event.key.keysym.sym == k(SDLK_LCTRL) {
                            move_nodrag = false;
                        }
                    }

                    SDL_MOUSEBUTTONDOWN_EV => {
                        let iClickX = (event.button.x / TILESIZE) as i16;
                        let iClickY = (event.button.y / TILESIZE) as i16;

                        if event.button.button == BUTTON_LEFT && !ignoreclick {
                            if edit_mode == 0 {
                                //paint selected blocks
                                paint_block(iClickX, iClickY);

                                adjust_map_items(iClickX, iClickY);
                            } else if edit_mode == 1 {
                                //paint selected tile(s)
                                for i in 0..set_tile_cols as i16 {
                                    let iLocalX = (iClickX as i32 + i as i32) as i16;

                                    for j in 0..set_tile_rows as i16 {
                                        let iLocalY = (iClickY as i32 + j as i32) as i16;

                                        if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                                            let fNeedUpdate = !tile_type_is_modified(iLocalX, iLocalY);
                                            set_tileset_tile(
                                                &mut g_map.mapdata[iLocalX as usize][iLocalY as usize][selected_layer as usize],
                                                set_tile_tileset as i16,
                                                (set_tile_start_x + i as i32) as i16,
                                                (set_tile_start_y + j as i32) as i16,
                                            );

                                            if fNeedUpdate {
                                                update_tile_type(iLocalX, iLocalY);
                                            }

                                            adjust_map_items(iLocalX, iLocalY);
                                        }
                                    }
                                }
                            } else if edit_mode == 2 {
                                //warps
                                g_map.warpdata[iClickX as usize][iClickY as usize].direction = set_direction as WarpEnterDirection;
                                g_map.warpdata[iClickX as usize][iClickY as usize].connection = set_connection as i16;
                            } else if edit_mode == 3 {
                                //move tiles
                                if move_mode == 3 {
                                    move_mode = 0;

                                    pasteselectedtiles(move_offset_x, move_offset_y);

                                    copymoveselection();
                                    resetselectedtiles();
                                    pastemoveselection(move_offset_x, move_offset_y);

                                    move_offset_x = 0;
                                    move_offset_y = 0;
                                } else {
                                    if selectedtiles[iClickX as usize][iClickY as usize] {
                                        if copyselectedtiles() {
                                            move_mode = 1;
                                            clearselectedmaptiles();
                                        }
                                    } else {
                                        if !check_key(keystate, SDLK_LSHIFT) && !check_key(keystate, SDLK_RSHIFT) && !check_key(keystate, SDLK_LCTRL) {
                                            resetselectedtiles();
                                        }

                                        if move_nodrag {
                                            selectedtiles[iClickX as usize][iClickY as usize] = true;
                                        } else {
                                            move_mode = 2;
                                            move_drag_start_x = iClickX as i32;
                                            move_drag_start_y = iClickY as i32;
                                        }
                                    }

                                    move_start_x = iClickX as i32;
                                    move_start_y = iClickY as i32;
                                }
                            } else if edit_mode == 4 {
                                //no player spawn areas
                                set_no_spawn(nospawn_mode as i16, iClickX, iClickY, true);
                            } else if edit_mode == 5 {
                                // no item spawn areas
                                g_map.nospawn[5][iClickX as usize][iClickY as usize] = true;
                            } else if edit_mode == 6 {
                                //tile types
                                g_map.mapdatatop[iClickX as usize][iClickY as usize] = set_tiletype;
                                adjust_map_items(iClickX, iClickY);
                            } else if edit_mode == 7 {
                                //map items
                                if (g_map.mapitems.len() as i32) < MAXMAPITEMS {
                                    let mut fTileNotAvailable = false;
                                    for item in g_map.mapitems.iter() {
                                        if item.ix == iClickX && item.iy == iClickY {
                                            fTileNotAvailable = true;
                                            break;
                                        }
                                    }
                                    let top = g_map.mapdatatop[iClickX as usize][iClickY as usize];
                                    if top != TileType::NonSolid && top != TileType::SolidOnTop && top != TileType::IceOnTop {
                                        fTileNotAvailable = true;
                                    }
                                    if !fTileNotAvailable {
                                        let mapitem = MapItem { itype: set_mapitem as MapItemType, ix: iClickX, iy: iClickY };
                                        g_map.mapitems.push(mapitem);
                                    }
                                }
                            } else if edit_mode == 8 {
                                //animated tiles
                                paint_animated_tiles(iClickX, iClickY);
                            }
                        } else if event.button.button == BUTTON_RIGHT {
                            erase_at(iClickX, iClickY, true);
                        }
                    }

                    SDL_MOUSEMOTION_EV => {
                        bound_mouse_motion_coords();
                        let iClickX = (bound_to_window_w(event.motion.x) / TILESIZE) as i16;
                        let iClickY = (bound_to_window_h(event.motion.y) / TILESIZE) as i16;

                        if event.motion.state == sdl_button(SDL_BUTTON_LEFT) && !ignoreclick {
                            if edit_mode == 0 {
                                paint_block(iClickX, iClickY);

                                adjust_map_items(iClickX, iClickY);
                            } else if edit_mode == 1 {
                                for i in 0..set_tile_cols as i16 {
                                    let iLocalX = (iClickX as i32 + i as i32) as i16;

                                    for j in 0..set_tile_rows as i16 {
                                        let iLocalY = (iClickY as i32 + j as i32) as i16;

                                        if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                                            let fNeedUpdate = !tile_type_is_modified(iLocalX, iLocalY);

                                            set_tileset_tile(
                                                &mut g_map.mapdata[iLocalX as usize][iLocalY as usize][selected_layer as usize],
                                                set_tile_tileset as i16,
                                                (set_tile_start_x + i as i32) as i16,
                                                (set_tile_start_y + j as i32) as i16,
                                            );

                                            if fNeedUpdate {
                                                update_tile_type(iLocalX, iLocalY);
                                            }

                                            // (sic) the C++ passes iLocalY twice here.
                                            adjust_map_items(iLocalY, iLocalY);
                                        }
                                    }
                                }
                            } else if edit_mode == 2 {
                                g_map.warpdata[iClickX as usize][iClickY as usize].direction = set_direction as WarpEnterDirection;
                                g_map.warpdata[iClickX as usize][iClickY as usize].connection = set_connection as i16;
                            } else if edit_mode == 3 {
                                if move_mode == 0 {
                                    selectedtiles[iClickX as usize][iClickY as usize] = true;
                                }
                            } else if edit_mode == 4 {
                                set_no_spawn(nospawn_mode as i16, iClickX, iClickY, true);
                            } else if edit_mode == 5 {
                                g_map.nospawn[5][iClickX as usize][iClickY as usize] = true;
                            } else if edit_mode == 6 {
                                g_map.mapdatatop[iClickX as usize][iClickY as usize] = set_tiletype;
                                adjust_map_items(iClickX, iClickY);
                            } else if edit_mode == 8 {
                                paint_animated_tiles(iClickX, iClickY);
                            }
                        } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) {
                            erase_at(iClickX, iClickY, false);
                        }
                    }

                    SDL_MOUSEBUTTONUP_EV => {
                        let iClickX = event.button.x / TILESIZE;
                        let iClickY = event.button.y / TILESIZE;

                        if event.button.button == BUTTON_LEFT {
                            ignoreclick = false;

                            if move_mode == 1 {
                                pasteselectedtiles(move_offset_x, move_offset_y);

                                copymoveselection();
                                resetselectedtiles();
                                pastemoveselection(move_offset_x, move_offset_y);

                                move_offset_x = 0;
                                move_offset_y = 0;
                            } else if move_mode == 2 {
                                let left = if move_drag_start_x < iClickX { move_drag_start_x } else { iClickX };
                                let top = if move_drag_start_y < iClickY { move_drag_start_y } else { iClickY };
                                let right = if move_drag_start_x < iClickX { iClickX } else { move_drag_start_x };
                                let bottom = if move_drag_start_y < iClickY { iClickY } else { move_drag_start_y };

                                for kk in top..=bottom {
                                    for j in left..=right {
                                        selectedtiles[j as usize][kk as usize] = true;
                                    }
                                }
                            }

                            move_mode = 0;
                        }
                    }

                    _ => {}
                }
            }

            if move_mode == 1 || move_mode == 3 {
                move_offset_x = (mouse_x / TILESIZE) - move_start_x;
                move_offset_y = (mouse_y / TILESIZE) - move_start_y;
            } else if move_mode == 2 {
                move_drag_offset_x = mouse_x / TILESIZE;
                move_drag_offset_y = mouse_y / TILESIZE;
            }
        }

        if maplist.is_valid() {
            drawmap(false, TILESIZE as i16, false);
        } else {
            SDL_FillRect(screen, null(), 0x0);
            rm.menu_font_large.draw_centered(320, 200, "Map has been deleted.");
        }

        //Ask if you are sure you want to exit
        if fExiting {
            rm.spr_dialog.draw_src(224, 176, &rect(0, 0, 192, 128));
            rm.menu_font_large.draw_centered(320, 195, "Exit");
            rm.menu_font_large.draw_centered(320, 220, "Are You Sure?");
            rm.menu_font_large.draw_centered(282, 254, "Yes");
            rm.menu_font_large.draw_centered(356, 254, "No");

            rm.spr_dialog.draw_src(if fSelectedYes { 250 } else { 326 }, 250, &rect(192, 0, 64, 32));
        } else {
            if edit_mode == 0 {
                rm.menu_font_small.draw(0, 0, "Block Mode");
            } else if edit_mode == 1 || edit_mode == 8 {
                let mut modestring = String::new();

                if edit_mode == 1 {
                    modestring.push_str("Tile Mode - ");
                } else {
                    modestring.push_str("Animated Tile Mode - ");
                }

                if selected_layer == 0 {
                    modestring.push_str("Bottom Background");
                } else if selected_layer == 1 {
                    modestring.push_str("Top Background");
                } else if selected_layer == 2 {
                    modestring.push_str("Bottom Foreground");
                } else if selected_layer == 3 {
                    modestring.push_str("Top Foreground");
                }

                if view_only_layer {
                    modestring.push_str(" Only");
                }

                rm.menu_font_small.draw(0, 0, &modestring);

                draw_layer_indicator();
            } else if edit_mode == 2 {
                rm.menu_font_small.draw(0, 0, "Warp Mode");
            } else if edit_mode == 3 {
                for kk in 0..MAPHEIGHT {
                    for j in 0..MAPWIDTH {
                        if selectedtiles[j as usize][kk as usize] {
                            rm.spr_selectedtile.draw((j + move_offset_x) * TILESIZE, (kk + move_offset_y) * TILESIZE);
                        }
                    }
                }

                //Draw dragging selection
                if move_mode == 2 {
                    let left = if move_drag_start_x < move_drag_offset_x { move_drag_start_x } else { move_drag_offset_x };
                    let top = if move_drag_start_y < move_drag_offset_y { move_drag_start_y } else { move_drag_offset_y };
                    let right = if move_drag_start_x < move_drag_offset_x { move_drag_offset_x } else { move_drag_start_x };
                    let bottom = if move_drag_start_y < move_drag_offset_y { move_drag_offset_y } else { move_drag_start_y };

                    for kk in top..=bottom {
                        for j in left..=right {
                            if !selectedtiles[j as usize][kk as usize] {
                                rm.spr_selectedtile.draw(j * TILESIZE, kk * TILESIZE);
                            }
                        }
                    }
                }

                if move_replace {
                    rm.menu_font_small.draw(0, 0, "Move Mode - Replace");
                } else {
                    rm.menu_font_small.draw(0, 0, "Move Mode - Merge");
                }

                draw_layer_indicator();
            } else if edit_mode == 4 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g_map.nospawn[nospawn_mode as usize][j][kk] {
                            rm.spr_nospawntile.draw_src(j as i32 * TILESIZE, kk as i32 * TILESIZE, &rect(nospawn_mode * 32, 0, 32, 32));
                        }

                        if nospawn_mode > 0 {
                            if g_map.nospawn[0][j][kk] {
                                rm.spr_nospawntile.draw_src(j as i32 * TILESIZE, kk as i32 * TILESIZE, &rect(0, 0, 32, 32));
                            }
                        }
                    }
                }

                rm.menu_font_small.draw(0, 480 - (rm.menu_font_small.get_height() << 1), "No Player Spawn: [x] Global, [1-4] Team Spawn Zone");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[a] All, [n] None, [i] Invert");
            } else if edit_mode == 5 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g_map.nospawn[5][j][kk] {
                            rm.spr_noitemspawntile.draw(j as i32 * TILESIZE, kk as i32 * TILESIZE);
                        }
                    }
                }

                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "No Item Spawn");
            } else if edit_mode == 6 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g_map.mapdatatop[j][kk] != TileType::NonSolid {
                            rm.spr_transparenttiles.draw_src(
                                j as i32 * TILESIZE,
                                kk as i32 * TILESIZE,
                                &rect(prev_tile_type(g_map.mapdatatop[j][kk]).0 as i32 * TILESIZE, 0, TILESIZE, TILESIZE),
                            );
                        }
                    }
                }

                rm.menu_font_small.draw(0, 0, "Tile Type Mode");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Press [Delete] To Clear All Tile Types");
            } else if edit_mode == 7 {
                rm.menu_font_small.draw(0, 0, "Map Item Mode");
            }

            rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

            if g_musiccategorydisplaytimer > 0 {
                g_musiccategorydisplaytimer -= 1;

                rm.spr_dialog.draw_src(224, 176, &rect(0, 0, 192, 128));
                rm.menu_font_small.draw_centered(320, 195, "Music Category");
                rm.menu_font_large.draw_centered(320, 220, g_szMusicCategoryNames[g_map.musicCategoryID as usize]);

                rm.menu_font_small.draw_centered(320, 255, "Press 'R' Again");
                rm.menu_font_small.draw_centered(320, 270, "To Change");
            }

            draw_message();
        }

        EDITOR_EDIT
    }
}

unsafe fn draw_layer_indicator() {
    if view_only_layer {
        rm.spr_backgroundlevel.draw_src(2, 18 + (3 - selected_layer) * 18, &rect(selected_layer * 16, (3 - selected_layer) * 18, 16, 16));
    } else {
        rm.spr_backgroundlevel.draw_src(2, 18, &rect(selected_layer * 16, 0, 16, 70));
    }
}

/// `strncmp(name, szBackgroundFile, strlen(name))` over the music category names.
unsafe fn set_music_category_from_background() {
    for iCategory in 0..MAXMUSICCATEGORY as i16 {
        if g_map.szBackgroundFile.as_bytes().starts_with(g_szMusicCategoryNames[iCategory as usize].as_bytes()) {
            g_map.musicCategoryID = iCategory;
            break;
        }
    }
}

unsafe fn paint_animated_tiles(iClickX: i16, iClickY: i16) {
    for i in 0..set_tile_cols as i16 {
        let iLocalX = (iClickX as i32 + i as i32) as i16;

        for j in 0..set_tile_rows as i16 {
            let iLocalY = (iClickY as i32 + j as i32) as i16;

            if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                let fNeedUpdate = !tile_type_is_modified(iLocalX, iLocalY);
                set_tileset_tile(
                    &mut g_map.mapdata[iLocalX as usize][iLocalY as usize][selected_layer as usize],
                    TILESETANIMATED as i16,
                    (set_tile_start_y + j as i32) as i16,
                    (set_tile_start_x + i as i32) as i16,
                );

                if fNeedUpdate {
                    update_tile_type(iLocalX, iLocalY);
                }
            }
        }
    }
}

/// The right-button handlers of `editor_edit`; only the button-down one removes map items.
unsafe fn erase_at(iClickX: i16, iClickY: i16, fButtonDown: bool) {
    let (x, y) = (iClickX as usize, iClickY as usize);
    if edit_mode == 0 {
        g_map.objectdata[x][y].iType = -1;
    } else if edit_mode == 1 || edit_mode == 8 {
        let fNeedUpdate = !tile_type_is_modified(iClickX, iClickY);
        g_map.mapdata[x][y][selected_layer as usize].iID = TILESETNONE as i16;

        if fNeedUpdate {
            update_tile_type(iClickX, iClickY);
        }
    } else if edit_mode == 2 {
        g_map.warpdata[x][y].direction = WARP_UNDEFINED;
        g_map.warpdata[x][y].connection = -1;
    } else if edit_mode == 3 {
        if fButtonDown {
            if move_mode == 3 {
                move_mode = 0;
                resetselectedtiles();
                move_offset_x = 0;
                move_offset_y = 0;
            } else if selectedtiles[x][y] {
                selectedtiles[x][y] = false;
            } else {
                resetselectedtiles();
            }
        } else if move_mode == 0 {
            selectedtiles[x][y] = false;
        }
    } else if edit_mode == 4 {
        set_no_spawn(nospawn_mode as i16, iClickX, iClickY, false);
    } else if edit_mode == 5 {
        g_map.nospawn[5][x][y] = false;
    } else if edit_mode == 6 {
        g_map.mapdatatop[x][y] = TileType::NonSolid;
    } else if edit_mode == 7 && fButtonDown {
        remove_map_item_at(iClickX, iClickY);
    }
}

pub fn draw_message() {
    unsafe {
        if g_messagedisplaytimer > 0 {
            g_messagedisplaytimer -= 1;

            rm.spr_dialog.draw_src(224, 176, &rect(0, 0, 192, 128));
            rm.menu_font_large.draw_centered(320, 195, &g_szMessageTitle);
            rm.menu_font_large.draw_centered(320, 220, &g_szMessageLine[0]);
            rm.menu_font_large.draw_centered(320, 240, &g_szMessageLine[1]);
            rm.menu_font_large.draw_centered(320, 260, &g_szMessageLine[2]);
        }
    }
}

pub fn set_no_spawn(nospawnmode: i16, col: i16, row: i16, value: bool) {
    unsafe {
        if !value && nospawnmode > 0 && g_map.nospawn[0][col as usize][row as usize] {
            return;
        }

        g_map.nospawn[nospawnmode as usize][col as usize][row as usize] = value;

        if nospawnmode == 0 {
            for imode in 1..5usize {
                g_map.nospawn[imode][col as usize][row as usize] = value;
            }
        }
    }
}

fn in_moved_selection(i: i32, j: i32) -> bool {
    unsafe {
        (move_mode == 1 || move_mode == 3)
            && i - move_offset_x >= 0
            && i - move_offset_x < MAPWIDTH
            && j - move_offset_y >= 0
            && j - move_offset_y < MAPHEIGHT
            && selectedtiles[(i - move_offset_x) as usize][(j - move_offset_y) as usize]
    }
}

pub fn drawlayer(layer: i32, fUseCopied: bool, iBlockSize: i16) {
    unsafe {
        let iTilesetIndex: i16 = if iBlockSize as i32 == TILESIZE {
            0
        } else if iBlockSize as i32 == PREVIEWTILESIZE {
            1
        } else {
            2
        };

        //draw left to right full vertical
        for i in 0..MAPWIDTH as i16 {
            for j in 0..MAPHEIGHT as i16 {
                let mut tile: Option<TilesetTile> = None;
                if in_moved_selection(i as i32, j as i32) {
                    if fUseCopied {
                        tile = Some(copiedtiles[(i as i32 - move_offset_x) as usize][(j as i32 - move_offset_y) as usize].tile[layer as usize]);
                    }
                } else if !fUseCopied {
                    tile = Some(g_map.mapdata[i as usize][j as usize][layer as usize]);
                }

                let tile = match tile {
                    Some(t) if t.iID as i32 != TILESETNONE => t,
                    _ => continue,
                };

                if tile.iID >= 0 {
                    g_tilesetmanager.draw(screen, tile.iID, iTilesetIndex, tile.iCol, tile.iRow, i, j);
                } else if tile.iID as i32 == TILESETANIMATED {
                    let mut iSrcCol: i16 = ((tile.iCol as i32) << 2) as i16;
                    let mut iSrcRow: i16 = tile.iRow;

                    if iSrcCol > 31 || iSrcCol < 0 || iSrcRow > 31 || iSrcRow < 0 {
                        iSrcCol = 0;
                        iSrcRow = 0;
                    }

                    let src = g_tilesetmanager.rect(iTilesetIndex, iSrcCol, iSrcRow);
                    let dst = g_tilesetmanager.rect(iTilesetIndex, i, j);
                    SDL_UpperBlit(rm.spr_tileanimation[iTilesetIndex as usize].get_surface(), src, screen, dst);
                } else if tile.iID as i32 == TILESETUNKNOWN {
                    let src = g_tilesetmanager.rect(iTilesetIndex, 0, 0);
                    let dst = g_tilesetmanager.rect(iTilesetIndex, i, j);
                    SDL_UpperBlit(rm.spr_unknowntile[iTilesetIndex as usize].get_surface(), src, screen, dst);
                }
            }
        }
    }
}

pub fn drawmap(fScreenshot: bool, iBlockSize: i16, fWithPlatforms: bool) {
    unsafe {
        let bs = iBlockSize as i32;
        if bs != TILESIZE {
            let srcrect = rect(0, 0, 640, 480);
            let dstrect = rect(0, 0, bs * 20, bs * 15);

            rm.spr_background.draw_stretch(&srcrect, blitdest, &dstrect);
        } else {
            rm.spr_background.draw(0, 0);
        }

        if (view_only_layer && selected_layer == 0) || !view_only_layer {
            drawlayer(0, false, iBlockSize);
        }

        if !fScreenshot {
            if (view_only_layer && copiedlayer == 0) || !view_only_layer {
                drawlayer(0, true, iBlockSize);
            }
        }

        if (view_only_layer && selected_layer == 1) || !view_only_layer {
            drawlayer(1, false, iBlockSize);
        }

        if !fScreenshot {
            if (view_only_layer && copiedlayer == 1) || !view_only_layer {
                drawlayer(1, true, iBlockSize);
            }
        }

        if fWithPlatforms {
            g_map.draw_platforms(0);
        }

        if (viewblocks && !view_only_layer) || fScreenshot {
            let iTilesizeIndex: i16 = if bs == 32 {
                0
            } else if bs == 16 {
                1
            } else {
                2
            };

            let mut rSrcBlock = rect(0, 0, bs, bs);

            for j in 0..MAPHEIGHT {
                for i in 0..MAPWIDTH {
                    let block: MapBlock = if in_moved_selection(i, j) {
                        copiedtiles[(i - move_offset_x) as usize][(j - move_offset_y) as usize].block
                    } else {
                        g_map.objectdata[i as usize][j as usize]
                    };

                    let blocktype = block.iType as i32;

                    if blocktype > -1 {
                        //Don't screenshot hidden blocks
                        if fScreenshot && g_map.objectdata[i as usize][j as usize].fHidden {
                            continue;
                        }

                        if blocktype < 7 {
                            rSrcBlock.x = blocktype * bs;
                            rSrcBlock.y = 0;
                        } else if (7..=10).contains(&blocktype) {
                            //On/Off Blocks
                            rSrcBlock.x = blocktype * bs;
                            rSrcBlock.y = bs * (1 - g_map.iSwitches[((blocktype - 7) % 4) as usize] as i32);
                        } else if (11..=14).contains(&blocktype) {
                            //Switched Blocks
                            rSrcBlock.x = blocktype * bs;
                            rSrcBlock.y = bs * (1 - block.iSettings[0] as i32);
                        } else if (15..=19).contains(&blocktype) {
                            rSrcBlock.x = (blocktype - 15) * bs;
                            rSrcBlock.y = bs;
                        } else if (20..=29).contains(&blocktype) {
                            rSrcBlock.x = (blocktype - 20) * bs;
                            rSrcBlock.y = bs << 1;
                        }

                        let dst = g_tilesetmanager.rect(iTilesizeIndex, i as i16, j as i16);
                        SDL_UpperBlit(rm.spr_blocks[iTilesizeIndex as usize].get_surface(), &rSrcBlock, screen, dst);
                    }
                }
            }
        }

        if fWithPlatforms {
            g_map.draw_platforms(1);
        }

        if !view_only_layer || fScreenshot {
            let idx = if bs == TILESIZE {
                0
            } else if bs == PREVIEWTILESIZE {
                1
            } else {
                2
            };
            for item in g_map.mapitems.clone().iter() {
                rm.spr_mapitems[idx].draw_src(item.ix as i32 * bs, item.iy as i32 * bs, &rect(item.itype as i32 * bs, 0, bs, bs));
            }

            if !fScreenshot {
                for j in 0..MAPHEIGHT {
                    for i in 0..MAPWIDTH {
                        let iNewX = (i - move_offset_x) as i16;
                        let iNewY = (j - move_offset_y) as i16;

                        if (move_mode == 1 || move_mode == 3)
                            && iNewX >= 0
                            && (iNewX as i32) < MAPWIDTH
                            && iNewY >= 0
                            && (iNewY as i32) < MAPHEIGHT
                            && selectedtiles[iNewX as usize][iNewY as usize]
                        {
                            let item = copiedtiles[iNewX as usize][iNewY as usize].item;
                            if item >= 0 {
                                rm.spr_mapitems[0].draw_src(i << 5, j << 5, &rect(item << 5, 0, TILESIZE, TILESIZE));
                            }
                        }
                    }
                }
            }
        }

        if fWithPlatforms {
            g_map.draw_platforms(2);
        }

        if (view_only_layer && selected_layer == 2) || !view_only_layer {
            drawlayer(2, false, iBlockSize);
        }

        if !fScreenshot {
            if (view_only_layer && copiedlayer == 2) || !view_only_layer {
                drawlayer(2, true, iBlockSize);
            }
        }

        if (view_only_layer && selected_layer == 3) || !view_only_layer {
            drawlayer(3, false, iBlockSize);
        }

        if !fScreenshot {
            if (view_only_layer && copiedlayer == 3) || !view_only_layer {
                drawlayer(3, true, iBlockSize);
            }
        }

        if fWithPlatforms {
            g_map.draw_platforms(3);
        }

        if (viewwarps && !view_only_layer) || fScreenshot {
            for j in 0..MAPHEIGHT {
                for i in 0..MAPWIDTH {
                    let warp: Warp = if in_moved_selection(i, j) {
                        copiedtiles[(i - move_offset_x) as usize][(j - move_offset_y) as usize].warp
                    } else {
                        g_map.warpdata[i as usize][j as usize]
                    };

                    if warp.connection != -1 {
                        let rSrcWarp = rect(warp.connection as i32 * bs, warp.direction as i32 * bs, bs, bs);
                        let rDstWarp = rect(i * bs, j * bs, bs, bs);

                        let idx = if bs == TILESIZE {
                            0
                        } else if bs == PREVIEWTILESIZE {
                            1
                        } else {
                            2
                        };
                        rm.spr_warps[idx].draw_src_to(&rSrcWarp, screen, &rDstWarp);
                    }
                }
            }
        }

        if fWithPlatforms {
            g_map.draw_platforms(4);
        }
    }
}

pub static mut r: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };

pub fn editor_warp() -> i32 {
    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    edit_mode = 2; //change to edit mode using warps
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if event.button.button == BUTTON_LEFT {
                        if event.button.x / TILESIZE < 10 && event.button.y / TILESIZE < 4 {
                            set_direction = event.button.y / TILESIZE;
                            set_connection = event.button.x / TILESIZE;
                        }

                        edit_mode = 2; //change to edit mode using warps

                        //The user must release the mouse button before trying to add a tile
                        ignoreclick = true;

                        return EDITOR_EDIT;
                    }
                }

                _ => {}
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        r.x = 0;
        r.y = 0;
        r.w = 640;
        r.h = 480;

        rm.spr_warps[0].draw_to(screen, &r);
        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        draw_message();
        EDITOR_WARP
    }
}

const NUM_EYECANDY: i16 = 7;
const szEyecandyNames: [&str; NUM_EYECANDY as usize] = ["Clouds", "Ghosts", "Leaves", "Snow", "Fish", "Rain", "Bubbles"];
const szLayerNames: [&str; 3] = ["Back Layer", "Mid Layer", "Top Layer"];

pub fn editor_eyecandy() -> i32 {
    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            let ty = event.type_;
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                if event.key.keysym.sym == k(SDLK_ESCAPE) || event.key.keysym.sym == k(SDLK_e) {
                    return EDITOR_EDIT;
                }
                fall = true;
            }

            if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                if event.button.button == BUTTON_LEFT {
                    let mut ix: i16 = 165;
                    for iLayer in 0..3usize {
                        for kk in 0..NUM_EYECANDY {
                            if event.button.x >= ix as i32
                                && event.button.x < ix as i32 + 90
                                && event.button.y >= kk as i32 * 65 + 20
                                && event.button.y < kk as i32 * 65 + 72
                            {
                                let mask: i16 = 1 << kk;
                                if g_map.eyecandy[iLayer] & mask != 0 {
                                    g_map.eyecandy[iLayer] &= !mask;
                                } else {
                                    g_map.eyecandy[iLayer] |= mask;
                                }
                            }
                        }

                        ix += 110;
                    }
                }
            } else if ty == SDL_MOUSEMOTION_EV {
                bound_mouse_motion_coords();
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        let mut ix: i16 = 165;
        for iLayer in 0..3usize {
            for kk in 0..NUM_EYECANDY {
                let iy: i16 = kk * 65 + 20;

                if mouse_x >= ix as i32 && mouse_x < ix as i32 + 90 && mouse_y >= iy as i32 && mouse_y < iy as i32 + 52 {
                    rm.spr_powerupselector.draw_src(ix as i32, iy as i32, &rect(0, 0, 90, 52));
                } else {
                    rm.spr_powerupselector.draw_src(ix as i32, iy as i32, &rect(0, 52, 90, 52));
                }

                rm.spr_eyecandy.draw_src(ix as i32 + 10, iy as i32 + 10, &rect((kk as i32) << 5, 0, 32, 32));

                let mask: i16 = 1 << kk;
                if g_map.eyecandy[iLayer] & mask != 0 {
                    rm.spr_hidden_marker.draw(ix as i32 + 57, iy as i32 + 16);
                }
            }

            rm.menu_font_small.draw_centered(ix as i32 + 45, 10, szLayerNames[iLayer]);
            ix += 110;
        }

        for kk in 0..NUM_EYECANDY {
            rm.menu_font_small.draw_centered(320, kk as i32 * 65 + 62, szEyecandyNames[kk as usize]);
        }

        rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Eyecandy: [e] Exit, [LMB] Choose Eyecandy");

        draw_message();
        EDITOR_EYECANDY
    }
}

/// Returns the index of the block setting under (x, y); C++ returns a pointer into `iSettings`.
pub fn get_block_property(x: i32, y: i32, iSettingIndex: Option<&mut i16>) -> Option<usize> {
    for iSetting in 0..NUM_BLOCK_SETTINGS as i16 {
        let ix: i16 = (iSetting % 6) * 100 + 35;
        let iy: i16 = (iSetting / 6) * 62 + 65;

        if x >= ix as i32 - 10 && x < ix as i32 + 80 && y >= iy as i32 - 10 && y < iy as i32 + 42 {
            if let Some(idx) = iSettingIndex {
                *idx = iSetting;
            }

            return Some(iSetting as usize);
        }
    }

    None
}

pub static mut iBlockType: i16 = 0;
pub static mut editor_properties_initialized: bool = false;
pub fn init_editor_properties(iBlockCol: i16, iBlockRow: i16) {
    unsafe {
        if editor_properties_initialized {
            return;
        }

        iBlockType = g_map.objectdata[iBlockCol as usize][iBlockRow as usize].iType;

        editor_properties_initialized = true;
    }
}

pub fn editor_properties(iBlockCol: i16, iBlockRow: i16) -> i32 {
    init_editor_properties(iBlockCol, iBlockRow);
    unsafe {
        let (bc, br) = (iBlockCol as usize, iBlockRow as usize);
        loop {
            FPSLimiter::instance().frame_start();
            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                let ty = event.type_;
                let mut fall = false;

                if ty == SDL_QUIT_EV {
                    return EDITOR_QUIT;
                }

                if ty == SDL_KEYDOWN_EV {
                    let sym = event.key.keysym.sym;
                    if sym == k(SDLK_ESCAPE) || sym == k(SDLK_k) {
                        editor_properties_initialized = false;
                        return EDITOR_EDIT;
                    } else if (iBlockType == 1 || iBlockType == 15) && ((sym >= k(SDLK_0) && sym <= k(SDLK_9)) || sym == k(SDLK_BACKQUOTE) || sym == k(SDLK_d)) {
                        let mut iSettingIndex: i16 = 0;
                        let piSetting = get_block_property(mouse_x, mouse_y, Some(&mut iSettingIndex));

                        //If shift is held, set all powerups to this setting
                        let mut iValue: i16 = (sym - k(SDLK_0)) as i16;

                        if sym == k(SDLK_0) {
                            iValue = 10;
                        } else if sym == k(SDLK_BACKQUOTE) {
                            iValue = 0;
                        } else if sym == k(SDLK_d) {
                            iValue = default_powerup_setting(0, iSettingIndex as usize);
                        }

                        let keystate = get_keyboard_state();
                        if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                            for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                                if event.key.keysym.sym == k(SDLK_d) {
                                    iValue = default_powerup_setting(0, iSetting);
                                }

                                g_map.objectdata[bc][br].iSettings[iSetting] = iValue;
                            }
                        } else if let Some(s) = piSetting {
                            g_map.objectdata[bc][br].iSettings[s] = iValue;
                        }
                    }
                    fall = true;
                }

                if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                    let mut iHiddenCheckboxY: i16 = 0;
                    if iBlockType == 1 || iBlockType == 15 {
                        let mut iSettingIndex: i16 = 0;
                        let piSetting = get_block_property(event.button.x, event.button.y, Some(&mut iSettingIndex));

                        if let Some(s) = piSetting {
                            let iMouseState = get_mouse_state() as u8 as u32;
                            let setting = &mut g_map.objectdata[bc][br].iSettings[s];

                            if (event.button.button == BUTTON_RIGHT && (iMouseState & SDL_BUTTON_LMASK) != 0)
                                || (event.button.button == BUTTON_LEFT && (iMouseState & SDL_BUTTON_RMASK) != 0)
                            {
                                *setting = default_powerup_setting(0, iSettingIndex as usize);
                            } else if event.button.button == BUTTON_LEFT {
                                if *setting < 10 {
                                    *setting += 1;
                                }
                            } else if event.button.button == BUTTON_RIGHT {
                                if *setting > 0 {
                                    *setting -= 1;
                                }
                            }
                        }

                        iHiddenCheckboxY = 365;
                    } else if iBlockType == 4 || iBlockType == 5 || iBlockType == 17 || iBlockType == 18 || iBlockType == 3 {
                        iHiddenCheckboxY = 214;
                    }

                    if event.button.x >= 270 && event.button.x < 370 && event.button.y >= iHiddenCheckboxY as i32 && event.button.y < iHiddenCheckboxY as i32 + 52 {
                        if event.button.button == BUTTON_LEFT {
                            g_map.objectdata[bc][br].fHidden = !g_map.objectdata[bc][br].fHidden;
                        }
                    }

                    if iBlockType == 1 || iBlockType == 15 {
                        if event.button.x >= 390 && event.button.x < 490 && event.button.y >= iHiddenCheckboxY as i32 && event.button.y < iHiddenCheckboxY as i32 + 52 {
                            if event.button.button == BUTTON_LEFT {
                                if g_map.objectdata[bc][br].iSettings[0] >= 0 {
                                    g_map.objectdata[bc][br].iSettings[0] = -1;
                                } else {
                                    g_map.objectdata[bc][br].iSettings[0] = default_powerup_setting(0, 0);
                                }
                            }
                        }
                    }
                } else if ty == SDL_MOUSEMOTION_EV {
                    bound_mouse_motion_coords();
                }
            }

            drawmap(false, TILESIZE as i16, false);
            rm.menu_shade.draw(0, 0);

            let mut iHiddenCheckboxY: i16 = 0;

            if iBlockType == 1 || iBlockType == 15 {
                let fUseGame = g_map.objectdata[bc][br].iSettings[0] == -1;

                for iSetting in 0..NUM_BLOCK_SETTINGS as i16 {
                    let ix: i16 = (iSetting % 6) * 100 + 35;
                    let iy: i16 = (iSetting / 6) * 62 + 65;
                    let (ix, iy) = (ix as i32, iy as i32);

                    if mouse_x >= ix - 10 && mouse_x < ix + 80 && mouse_y >= iy - 10 && mouse_y < iy + 42 && !fUseGame {
                        rm.spr_powerupselector.draw_src(ix - 10, iy - 10, &rect(0, 0, 90, 52));
                    } else {
                        rm.spr_powerupselector.draw_src(ix - 10, iy - 10, &rect(0, 52, 90, 52));
                    }

                    rm.spr_powerups.draw_src(ix, iy, &rect((iSetting as i32) << 5, 0, 32, 32));

                    let szNum = if fUseGame { "X".to_string() } else { format!("{}", g_map.objectdata[bc][br].iSettings[iSetting as usize]) };

                    rm.menu_font_large.draw_centered(ix + 55, iy + 5, &szNum);
                }

                iHiddenCheckboxY = 365;

                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 3, "Block Property Mode");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 2, "[~, 0-9] Set Value [LMB] Increase [RMB] Decrease [D] Default");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[Shift] + [0-9 or D] Set All To Value");
            } else if iBlockType == 4 || iBlockType == 5 || iBlockType == 17 || iBlockType == 18 || iBlockType == 3 {
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Block Property Mode");
                iHiddenCheckboxY = 214;
            }

            let hy = iHiddenCheckboxY as i32;
            if mouse_x >= 270 && mouse_x < 370 && mouse_y >= hy && mouse_y < hy + 52 {
                rm.spr_powerupselector.draw_src(270, hy, &rect(90, 0, 100, 52));
            } else {
                rm.spr_powerupselector.draw_src(270, hy, &rect(90, 52, 100, 52));
            }

            rm.menu_font_large.draw_centered(320, hy + 3, "Hidden");

            if g_map.objectdata[bc][br].fHidden {
                rm.spr_hidden_marker.draw(310, hy + 27);
            }

            //Display "Use Game" option
            if iBlockType == 1 || iBlockType == 15 {
                if mouse_x >= 390 && mouse_x < 490 && mouse_y >= hy && mouse_y < hy + 52 {
                    rm.spr_powerupselector.draw_src(390, hy, &rect(90, 0, 100, 52));
                } else {
                    rm.spr_powerupselector.draw_src(390, hy, &rect(90, 52, 100, 52));
                }

                rm.menu_font_large.draw_centered(440, hy + 3, "Use Game");

                if g_map.objectdata[bc][br].iSettings[0] == -1 {
                    rm.spr_hidden_marker.draw(430, hy + 27);
                }
            }

            rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

            draw_message();
            editor_harness::frame_delay(0);
            FPSLimiter::instance().before_flip();
            gfx_flipscreen();
            FPSLimiter::instance().after_flip();
        }
    }
}

pub static mut rNewButton: [SDL_Rect; 2] = [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2];
pub static mut rTypeButton: [[SDL_Rect; 4]; 8] = [[SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 4]; 8];

const szPathNames: [&str; 3] = ["Line Segment", "Continuous", "Ellipse"];

const PLATFORM_EDIT_STATE_SELECT: i16 = 0;
const PLATFORM_EDIT_STATE_PATH_TYPE: i16 = 1;
const PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE: i16 = 2;
const PLATFORM_EDIT_STATE_EDIT: i16 = 3;
const PLATFORM_EDIT_STATE_PATH: i16 = 4;
const PLATFORM_EDIT_STATE_TEST: i16 = 5;
const PLATFORM_EDIT_STATE_TILETYPE: i16 = 6;
const PLATFORM_EDIT_STATE_ANIMATED: i16 = 7;
const PLATFORM_EDIT_STATE_MOVE: i16 = 8;

pub static mut iPlatformEditState: i16 = PLATFORM_EDIT_STATE_SELECT;
pub static mut iPlatformSwitchState: i16 = 0;
pub static mut iPlatformSwitchIndex: i16 = 0;
pub static mut iEditPlatform: i16 = 0;
pub static mut iPlatformTop: i16 = 0;
pub static mut iPlatformLeft: i16 = 0;
pub static mut iPlatformWidth: i16 = 0;
pub static mut iPlatformHeight: i16 = 0;

pub static mut iPlatformPreview: i16 = -1;

pub fn editor_platforms_update_layout() {
    const padding: i32 = 32;
    const btn_size: i32 = 32;
    const btn_spacing: i32 = 10;

    const grid_area_w: i32 = UI_PLATFORM_COLS * btn_size + (UI_PLATFORM_COLS - 1) * btn_spacing;
    const grid_area_max_h: i32 = UI_PLATFORM_ROWS * btn_size + (UI_PLATFORM_ROWS - 1) * btn_spacing;

    unsafe {
        let rows = ((g_iNumPlatforms as i32 + 1).min(MAX_PLATFORMS) + UI_PLATFORM_COLS - 1) / UI_PLATFORM_COLS;
        let grid_area_h = rows * btn_size + (rows - 1) * btn_spacing;
        let title_area_h = padding + rm.menu_font_small.get_height();
        let newbtn_area_h = padding + if (g_iNumPlatforms as i32) < MAX_PLATFORMS { 24 + rNewButton[0].h } else { 0 };

        r.w = 2 * padding + grid_area_w;
        r.h = title_area_h + grid_area_h + newbtn_area_h;
        r.x = (640 - r.w) / 2;
        r.y = (480 - grid_area_max_h) / 2 - title_area_h;

        rNewButton[1].x = r.x + (r.w >> 1) - (rNewButton[0].w >> 1);
        rNewButton[1].y = r.y + r.h - padding - rNewButton[0].h;
    }
}

pub static mut editor_platforms_initialized: bool = false;
pub fn init_editor_platforms() {
    unsafe {
        if editor_platforms_initialized {
            return;
        }

        rNewButton[0] = rect(0, 352, 76, 32);

        rNewButton[1].w = rNewButton[0].w;
        rNewButton[1].h = rNewButton[0].h;

        editor_platforms_update_layout();

        for iType in 0..3usize {
            rTypeButton[iType][0] = rect(0, 0, 192, 32);
            rTypeButton[iType][1] = rect(320 - (rTypeButton[iType][0].w >> 1), 180 + iType as i32 * 40, 192, 32);
            rTypeButton[iType][2] = rect(24 * iType as i32, 32, 24, 24);
            rTypeButton[iType][3] = rect(rTypeButton[iType][1].x + 8, rTypeButton[iType][1].y + 4, 24, 24);
        }

        editor_platforms_initialized = true;
    }
}

pub fn editor_platforms_draw_background_section(src_area: &SDL_Rect, dst_area: &SDL_Rect) {
    let mut offset_y = 0;
    while offset_y < dst_area.h {
        let h = (dst_area.h - offset_y).min(src_area.h);

        let mut offset_x = 0;
        while offset_x < dst_area.w {
            let w = (dst_area.w - offset_x).min(src_area.w);

            let src = rect(src_area.x, src_area.y, w, h);
            let mut dst = rect(dst_area.x + offset_x, dst_area.y + offset_y, w, h);
            unsafe { SDL_UpperBlit(s_platform, &src, screen, &mut dst) };

            offset_x += w;
        }

        offset_y += h;
    }
}

pub fn editor_platforms_draw_background() {
    const corner: i32 = 24;

    unsafe {
        let mut src = rect(0, 0, corner, corner);
        let mut dst = rect(r.x, r.y, corner, corner);

        const src_w: [i32; 3] = [corner, 256 - 2 * corner, corner];
        const src_h: [i32; 3] = [corner, 224 - 2 * corner, corner];

        let dst_w: [i32; 3] = [corner, r.w - 2 * corner, corner];
        let dst_h: [i32; 3] = [corner, r.h - 2 * corner, corner];

        for row in 0..3 {
            src.x = 0;
            src.h = src_h[row];

            dst.x = r.x;
            dst.h = dst_h[row];

            for col in 0..3 {
                src.w = src_w[col];
                dst.w = dst_w[col];
                editor_platforms_draw_background_section(&src, &dst);
                src.x += src_w[col];
                dst.x += dst_w[col];
            }

            src.y += src.h;
            dst.y += dst.h;
        }
    }
}

fn platform_edit_or_animated_or_tiletype() -> bool {
    unsafe {
        PLATFORM_EDIT_STATE_EDIT == iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == iPlatformEditState || PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState
    }
}

/// The nested `while (editor_x() == EDITOR_X)` loops the platform editor runs for tiles, animation and tile types.
unsafe fn run_nested_editor(f: fn() -> i32, stay: i32) {
    FPSLimiter::instance().frame_start();
    while f() == stay {
        editor_harness::frame_delay(0);
        FPSLimiter::instance().before_flip();
        gfx_flipscreen();
        FPSLimiter::instance().after_flip();
        FPSLimiter::instance().frame_start();
    }
}

pub fn editor_platforms() -> i32 {
    init_editor_platforms();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            let ty = event.type_;
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                let sym = event.key.keysym.sym;
                let ep = iEditPlatform as usize;
                if sym == k(SDLK_s) {
                    savecurrentmap();
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_9) {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState && sym - k(SDLK_1) < g_iNumPlatforms as i32 {
                        iEditPlatform = (sym - k(SDLK_1)) as i16;
                        iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    } else if (PLATFORM_EDIT_STATE_PATH_TYPE == iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == iPlatformEditState)
                        && sym >= k(SDLK_1)
                        && sym <= k(SDLK_3)
                    {
                        g_Platforms[ep].iPathType = path_type_from((sym - k(SDLK_1)) as u8);

                        if PLATFORM_EDIT_STATE_PATH_TYPE == iPlatformEditState {
                            iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                            set_platform_to_defaults(iEditPlatform);
                        } else {
                            iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                        }
                    } else if PLATFORM_EDIT_STATE_MOVE == iPlatformEditState && sym - k(SDLK_1) < g_iNumPlatforms as i32 {
                        if iPlatformSwitchState == 1 {
                            switch_platforms(iPlatformSwitchIndex, (sym - k(SDLK_1)) as i16);

                            iPlatformPreview = -1;

                            iPlatformSwitchState = 0;
                            iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                        } else {
                            iPlatformSwitchState = 1;
                            iPlatformSwitchIndex = (sym - k(SDLK_1)) as i16;
                        }
                    }
                } else if sym == k(SDLK_n) {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState && (g_iNumPlatforms as i32) < MAX_PLATFORMS {
                        iEditPlatform = g_iNumPlatforms;
                        g_iNumPlatforms += 1;
                        editor_platforms_update_layout();
                        iPlatformEditState = PLATFORM_EDIT_STATE_PATH_TYPE;
                    }
                } else if sym == k(SDLK_m) {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState {
                        iPlatformPreview = -1;
                        iPlatformEditState = PLATFORM_EDIT_STATE_MOVE;
                        iPlatformSwitchState = 0;
                    }
                } else if sym == k(SDLK_t) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(editor_tiles, EDITOR_TILES);
                        iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState {
                        iPlatformEditState = PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE;
                    }
                } else if sym == k(SDLK_a) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(editor_animation, EDITOR_ANIMATION);
                        iPlatformEditState = PLATFORM_EDIT_STATE_ANIMATED;
                    }
                } else if sym == k(SDLK_l) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(editor_tiletype, EDITOR_TILETYPE);
                        iPlatformEditState = PLATFORM_EDIT_STATE_TILETYPE;
                    }
                } else if sym == k(SDLK_y) {
                    //Change the draw layer
                    if platform_edit_or_animated_or_tiletype() {
                        g_Platforms[ep].iDrawLayer += 1;
                        if g_Platforms[ep].iDrawLayer > 4 {
                            g_Platforms[ep].iDrawLayer = 0;
                        }
                    }
                } else if sym == k(SDLK_c) {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState {
                        iPlatformEditState = PLATFORM_EDIT_STATE_TEST;
                        insert_platforms_into_map();
                        g_map.reset_platforms();
                    }
                } else if sym == k(SDLK_DELETE) {
                    if platform_edit_or_animated_or_tiletype() {
                        //Copy platforms into empty spot
                        let mut iPlatform = iEditPlatform as usize;
                        while (iPlatform as i32) < g_iNumPlatforms as i32 - 1 {
                            for iCol in 0..MW {
                                for iRow in 0..MH {
                                    g_Platforms[iPlatform].tiles[iCol * MH + iRow] = g_Platforms[iPlatform + 1].tiles[iCol * MH + iRow];
                                    g_Platforms[iPlatform].types[iCol * MH + iRow] = g_Platforms[iPlatform + 1].types[iCol * MH + iRow];
                                }
                            }

                            g_Platforms[iPlatform].iPathType = g_Platforms[iPlatform + 1].iPathType;

                            g_Platforms[iPlatform].iVelocity = g_Platforms[iPlatform + 1].iVelocity;
                            g_Platforms[iPlatform].iStartX = g_Platforms[iPlatform + 1].iStartX;
                            g_Platforms[iPlatform].iStartY = g_Platforms[iPlatform + 1].iStartY;
                            g_Platforms[iPlatform].iEndX = g_Platforms[iPlatform + 1].iEndX;
                            g_Platforms[iPlatform].iEndY = g_Platforms[iPlatform + 1].iEndY;

                            g_Platforms[iPlatform].fAngle = g_Platforms[iPlatform + 1].fAngle;
                            g_Platforms[iPlatform].fRadiusX = g_Platforms[iPlatform + 1].fRadiusX;
                            g_Platforms[iPlatform].fRadiusY = g_Platforms[iPlatform + 1].fRadiusY;

                            g_Platforms[iPlatform].iDrawLayer = g_Platforms[iPlatform + 1].iDrawLayer;
                            iPlatform += 1;
                        }

                        g_iNumPlatforms -= 1;
                        iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_ESCAPE) {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState {
                        editor_platforms_initialized = false;
                        return EDITOR_EDIT;
                    } else if PLATFORM_EDIT_STATE_EDIT == iPlatformEditState
                        || PLATFORM_EDIT_STATE_ANIMATED == iPlatformEditState
                        || PLATFORM_EDIT_STATE_MOVE == iPlatformEditState
                        || PLATFORM_EDIT_STATE_TEST == iPlatformEditState
                    {
                        iPlatformPreview = -1;
                        g_Platforms[ep].update_preview();
                        iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                        //Fix menu offset
                        editor_platforms_update_layout();
                    } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState || PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState {
                        iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    }
                } else if sym == k(SDLK_KP_MINUS) || sym == k(SDLK_MINUS) {
                    if platform_edit_or_animated_or_tiletype() {
                        let p = &mut g_Platforms[ep];
                        if p.iPathType == PlatformPathType::Ellipse {
                            if p.iVelocity > -10 {
                                p.iVelocity -= 1;
                            }

                            if p.iVelocity == 0 {
                                p.iVelocity -= 1;
                            }
                        } else if p.iVelocity > 2 {
                            p.iVelocity -= 1;
                        }
                    }
                } else if sym == k(SDLK_KP_PLUS) || sym == k(SDLK_EQUALS) {
                    if platform_edit_or_animated_or_tiletype() {
                        let p = &mut g_Platforms[ep];
                        if p.iPathType == PlatformPathType::Ellipse {
                            if p.iVelocity < 10 {
                                p.iVelocity += 1;
                            }

                            if p.iVelocity == 0 {
                                p.iVelocity += 1;
                            }
                        } else if (p.iVelocity as i32) < MAX_PLATFORM_VELOCITY {
                            p.iVelocity += 1;
                        }
                    }
                } else if sym == k(SDLK_p) {
                    if platform_edit_or_animated_or_tiletype() {
                        iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                        calculate_platform_dims(iEditPlatform, &mut iPlatformLeft, &mut iPlatformTop, &mut iPlatformWidth, &mut iPlatformHeight);
                        insert_platforms_into_map();
                    }
                }
            } else if ty == SDL_MOUSEBUTTONDOWN_EV {
                let ep = iEditPlatform as usize;
                if event.button.button == BUTTON_LEFT && !ignoreclick {
                    if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == iPlatformEditState {
                        //check clicks on existing platforms
                        for iPlatform in 0..g_iNumPlatforms as i32 {
                            let ic = g_Platforms[iPlatform as usize].rIcon[1];
                            if event.button.x >= ic.x && event.button.x < ic.x + ic.w && event.button.y >= ic.y && event.button.y < ic.y + ic.h {
                                if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState {
                                    iEditPlatform = iPlatform as i16;
                                    iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                                    ignoreclick = true;
                                } else if PLATFORM_EDIT_STATE_MOVE == iPlatformEditState {
                                    if iPlatformSwitchState == 1 {
                                        switch_platforms(iPlatformSwitchIndex, iPlatform as i16);

                                        iPlatformPreview = -1;

                                        iPlatformSwitchState = 0;
                                        iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                                    } else {
                                        iPlatformSwitchState = 1;
                                        iPlatformSwitchIndex = iPlatform as i16;
                                    }
                                }
                            }
                        }

                        if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState
                            && (g_iNumPlatforms as i32) < MAX_PLATFORMS
                            && event.button.x >= rNewButton[1].x
                            && event.button.x < rNewButton[1].x + rNewButton[1].w
                            && event.button.y >= rNewButton[1].y
                            && event.button.y < rNewButton[1].y + rNewButton[1].h
                        {
                            //Create a new platform then edit it

                            iEditPlatform = g_iNumPlatforms;
                            g_iNumPlatforms += 1;
                            editor_platforms_update_layout();
                            iPlatformEditState = PLATFORM_EDIT_STATE_PATH_TYPE;
                            ignoreclick = true;
                        }
                    } else if PLATFORM_EDIT_STATE_PATH_TYPE == iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == iPlatformEditState {
                        if !ignoreclick {
                            let iClickX = event.button.x as i16 as i32;
                            let iClickY = event.button.y as i16 as i32;

                            for iType in 0..3usize {
                                let tb = rTypeButton[iType][1];
                                if iClickX >= tb.x && iClickX < tb.x + tb.w && iClickY >= tb.y && iClickY < tb.y + tb.h {
                                    ignoreclick = true;

                                    g_Platforms[ep].iPathType = path_type_from(iType as u8);

                                    if PLATFORM_EDIT_STATE_PATH_TYPE == iPlatformEditState {
                                        iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                                        set_platform_to_defaults(iEditPlatform);
                                    } else {
                                        iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                                    }

                                    break;
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_EDIT == iPlatformEditState {
                        if !ignoreclick {
                            let ix = (event.button.x / TILESIZE) as i16;
                            let iy = (event.button.y / TILESIZE) as i16;

                            for i in 0..set_tile_cols as i16 {
                                for j in 0..set_tile_rows as i16 {
                                    let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                    if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                        let idx = (cx * MAPHEIGHT + cy) as usize;
                                        let tile = &mut g_Platforms[ep].tiles[idx];
                                        set_tileset_tile(tile, set_tile_tileset as i16, (set_tile_start_x + i as i32) as i16, (set_tile_start_y + j as i32) as i16);
                                        let t = *tile;
                                        g_Platforms[ep].types[idx] = g_tilesetmanager.tileset(t.iID as usize).tile_type(t.iCol as usize, t.iRow as usize);
                                    }
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_ANIMATED == iPlatformEditState {
                        //animated tiles
                        if !ignoreclick {
                            let ix = (event.button.x / TILESIZE) as i16;
                            let iy = (event.button.y / TILESIZE) as i16;

                            for i in 0..set_tile_cols as i16 {
                                for j in 0..set_tile_rows as i16 {
                                    let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                    if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                        let idx = (cx * MAPHEIGHT + cy) as usize;
                                        let tile = &mut g_Platforms[ep].tiles[idx];
                                        set_tileset_tile(tile, TILESETANIMATED as i16, (set_tile_start_y + j as i32) as i16, (set_tile_start_x + i as i32) as i16);
                                        let t = *tile;
                                        g_Platforms[ep].types[idx] = animatedtiletypes[(t.iRow as i32 + ((t.iCol as i32) << 5)) as usize];
                                    }
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState {
                        if !ignoreclick {
                            let ix = event.button.x / TILESIZE;
                            let iy = event.button.y / TILESIZE;

                            g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = set_tiletype;
                        }
                    } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState {
                        let keystate = get_keyboard_state();
                        if g_Platforms[ep].iPathType == PlatformPathType::Ellipse && (check_key(keystate, SDLK_z) || check_key(keystate, SDLK_x) || check_key(keystate, SDLK_c)) {
                            update_platform_path_radius(
                                iEditPlatform,
                                event.button.x as i16,
                                event.button.y as i16,
                                check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT),
                                check_key(keystate, SDLK_z),
                                check_key(keystate, SDLK_c),
                            );
                        } else {
                            update_platform_path_start(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    }
                } else {
                    if event.button.button == BUTTON_RIGHT {
                        let ix = event.button.x / TILESIZE;
                        let iy = event.button.y / TILESIZE;

                        if PLATFORM_EDIT_STATE_EDIT == iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == iPlatformEditState {
                            clear_tileset_tile(&mut g_Platforms[ep].tiles[(ix * MAPHEIGHT + iy) as usize]);
                            g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = TileType::NonSolid;
                        } else if PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState {
                            g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = TileType::NonSolid;
                        } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState {
                            let keystate = get_keyboard_state();
                            if g_Platforms[ep].iPathType == PlatformPathType::Straight {
                                update_platform_path_end(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                            } else if g_Platforms[ep].iPathType == PlatformPathType::StraightContinuous || g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                                update_platform_path_angle(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                            }
                        }
                    }
                    fall = true;
                }
            }

            if fall || ty == SDL_MOUSEMOTION_EV {
                let ep = iEditPlatform as usize;
                bound_mouse_motion_coords();
                let ix = (bound_to_window_w(event.motion.x) / TILESIZE) as i16;
                let iy = (bound_to_window_h(event.motion.y) / TILESIZE) as i16;

                if PLATFORM_EDIT_STATE_EDIT == iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == iPlatformEditState {
                    if event.motion.state == sdl_button(SDL_BUTTON_LEFT) && !ignoreclick {
                        for i in 0..set_tile_cols as i16 {
                            for j in 0..set_tile_rows as i16 {
                                let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                    let idx = (cx * MAPHEIGHT + cy) as usize;
                                    let tile = &mut g_Platforms[ep].tiles[idx];

                                    if PLATFORM_EDIT_STATE_EDIT == iPlatformEditState {
                                        set_tileset_tile(tile, set_tile_tileset as i16, (set_tile_start_x + i as i32) as i16, (set_tile_start_y + j as i32) as i16);
                                        let t = *tile;
                                        g_Platforms[ep].types[idx] = g_tilesetmanager.tileset(t.iID as usize).tile_type(t.iCol as usize, t.iRow as usize);
                                    } else {
                                        set_tileset_tile(tile, TILESETANIMATED as i16, (set_tile_start_y + j as i32) as i16, (set_tile_start_x + i as i32) as i16);
                                        let t = *tile;
                                        g_Platforms[ep].types[idx] = animatedtiletypes[(t.iRow as i32 + ((t.iCol as i32) << 5)) as usize];
                                    }
                                }
                            }
                        }
                    } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) {
                        let idx = (ix as i32 * MAPHEIGHT + iy as i32) as usize;
                        clear_tileset_tile(&mut g_Platforms[ep].tiles[idx]);
                        g_Platforms[ep].types[idx] = TileType::NonSolid;
                    }
                } else if PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState {
                    let idx = (ix as i32 * MAPHEIGHT + iy as i32) as usize;
                    if event.motion.state == sdl_button(SDL_BUTTON_LEFT) && !ignoreclick {
                        g_Platforms[ep].types[idx] = set_tiletype;
                    } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) {
                        g_Platforms[ep].types[idx] = TileType::NonSolid;
                    }
                } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState {
                    if event.motion.state == sdl_button(SDL_BUTTON_LEFT) {
                        let keystate = get_keyboard_state();
                        if g_Platforms[ep].iPathType == PlatformPathType::Ellipse && (check_key(keystate, SDLK_z) || check_key(keystate, SDLK_x) || check_key(keystate, SDLK_c)) {
                            update_platform_path_radius(
                                iEditPlatform,
                                event.button.x as i16,
                                event.button.y as i16,
                                check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT),
                                check_key(keystate, SDLK_z),
                                check_key(keystate, SDLK_c),
                            );
                        } else {
                            update_platform_path_start(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) {
                        let keystate = get_keyboard_state();
                        if g_Platforms[ep].iPathType == PlatformPathType::Straight {
                            update_platform_path_end(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        } else if g_Platforms[ep].iPathType == PlatformPathType::StraightContinuous || g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                            update_platform_path_angle(iEditPlatform, event.button.x as i16, event.button.y as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    }
                }
                //Display platform preview
                else if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == iPlatformEditState {
                    iPlatformPreview = -1;
                    for iPlatform in 0..g_iNumPlatforms as i32 {
                        let ic = g_Platforms[iPlatform as usize].rIcon[1];
                        if event.button.x >= ic.x && event.button.x < ic.x + ic.w && event.button.y >= ic.y && event.button.y < ic.y + ic.h {
                            iPlatformPreview = iPlatform as i16;
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if event.button.button == BUTTON_LEFT {
                    ignoreclick = false;
                }
            }
        }

        if PLATFORM_EDIT_STATE_TEST != iPlatformEditState {
            drawmap(false, TILESIZE as i16, false);
            rm.menu_shade.draw(0, 0);
        }

        let ep = iEditPlatform as usize;
        if PLATFORM_EDIT_STATE_SELECT == iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == iPlatformEditState {
            editor_platforms_draw_background();

            rm.menu_font_small.draw_centered(320, r.y + 18, "Platforms");

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Platform Mode: [esc] Exit  [c] Check Paths, [1-8] Select, [n] New");
            rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

            for iPlatform in 0..g_iNumPlatforms as usize {
                let p = &mut g_Platforms[iPlatform];
                let src = p.rIcon[0];
                SDL_UpperBlit(s_platform, &src, screen, &mut p.rIcon[1]);
            }

            if (g_iNumPlatforms as i32) < MAX_PLATFORMS && PLATFORM_EDIT_STATE_SELECT == iPlatformEditState {
                let src = rNewButton[0];
                SDL_UpperBlit(s_platform, &src, screen, &mut rNewButton[1]);
            }

            if PLATFORM_EDIT_STATE_MOVE == iPlatformEditState {
                if iPlatformSwitchState == 0 {
                    rm.menu_font_small.draw_centered(320, 280, "Select First");
                    rm.menu_font_small.draw_centered(320, 300, "Platform To Switch");
                } else {
                    rm.menu_font_small.draw_centered(320, 280, "Select Second");
                    rm.menu_font_small.draw_centered(320, 300, "Platform To Switch");
                }
            }

            if iPlatformPreview >= 0 {
                display_platform_preview(iPlatformPreview, event.button.x as i16, event.button.y as i16);
            }
        } else if PLATFORM_EDIT_STATE_PATH_TYPE == iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == iPlatformEditState {
            //Draw path options
            for iType in 0..3usize {
                let s0 = rTypeButton[iType][0];
                SDL_UpperBlit(s_platformpathbuttons, &s0, screen, &mut rTypeButton[iType][1]);
                let s2 = rTypeButton[iType][2];
                SDL_UpperBlit(s_platformpathbuttons, &s2, screen, &mut rTypeButton[iType][3]);

                rm.menu_font_large.draw(rTypeButton[iType][1].x + 36, rTypeButton[iType][1].y + 6, szPathNames[iType]);
            }

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Path Type");
        } else if platform_edit_or_animated_or_tiletype() {
            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 3, "Edit Platform");
            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 2, "[esc] Exit  [t] Tiles  [a] Animation [l] Types [del] Delete  [p] Path");
            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[+/-] Velocity  [y] Draw Layer");
            draw_platform_unfinished(iEditPlatform, PLATFORM_EDIT_STATE_TILETYPE == iPlatformEditState);

            if g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                let iVelMarkerX: i16 = (198 + (g_Platforms[ep].iVelocity as i32 + 10) * 12) as i16;

                let rVel = [rect(0, 400, 244, 17), rect(198, 10, 244, 17)];
                let mut d = rVel[1];
                SDL_UpperBlit(s_platform, &rVel[0], screen, &mut d);

                let rMarker = [rect(244, 400, 8, 18), rect(iVelMarkerX as i32, 10, 8, 18)];
                let mut d = rMarker[1];
                SDL_UpperBlit(s_platform, &rMarker[0], screen, &mut d);

                rm.menu_font_small.draw_right_justified(198, 10, "Counter");
                rm.menu_font_small.draw(442, 10, "Clockwise");
            } else {
                let iVelMarkerX: i16 = (220 + (g_Platforms[ep].iVelocity as i32 - 1) * 12) as i16;

                let rVel = [rect(12, 384, 172, 13), rect(234, 10, 172, 13)];
                let mut d = rVel[1];
                SDL_UpperBlit(s_platform, &rVel[0], screen, &mut d);

                let rMarker = [rect(184, 384, 8, 16), rect(iVelMarkerX as i32, 8, 8, 16)];
                let mut d = rMarker[1];
                SDL_UpperBlit(s_platform, &rMarker[0], screen, &mut d);

                rm.menu_font_small.draw_right_justified(234, 10, "Slow");
                rm.menu_font_small.draw(406, 10, "Fast");
            }

            rm.spr_number_icons.draw_src(619, 5, &rect((g_Platforms[ep].iDrawLayer as i32) << 4, 48, 16, 16));
        } else if PLATFORM_EDIT_STATE_PATH == iPlatformEditState {
            let platform = &g_Platforms[ep];
            let tiledata = g_map.platforms[ep].iTileData.clone();
            draw_platform(
                platform.iPathType,
                &tiledata,
                platform.iStartX,
                platform.iStartY,
                platform.iEndX,
                platform.iEndY,
                platform.fAngle,
                platform.fRadiusX,
                platform.fRadiusY,
                0,
                iPlatformWidth,
                iPlatformHeight,
                false,
                true,
            );

            if platform.iPathType == PlatformPathType::Straight {
                rm.menu_font_small.draw(0, 480 - (rm.menu_font_small.get_height() << 1), "Edit Path");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[esc] Exit  [LMB] Set Start Point  [RMB] Set End Point [t] Path Type");
            } else if platform.iPathType == PlatformPathType::StraightContinuous {
                rm.menu_font_small.draw(0, 480 - (rm.menu_font_small.get_height() << 1), "Edit Path: [esc] Exit  [LMB] Set Start Point  [RMB] Set Angle");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[SHIFT + LMB] Location Snap [SHIFT + RMB] Angle Snap [t] Path Type");
            } else if platform.iPathType == PlatformPathType::Ellipse {
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 4, "Edit Path: [esc] Exit  [LMB] Set Center [SHIFT + LMB] Center Snap");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 3, "[X + LMB] Set X Radius [SHIFT + X + LMB] X Radius Snap");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 2, "[Z + LMB] Set Y Radius [SHIFT + Z + LMB] Y Radius Snap");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[C + LMB] Set Circular Radius [SHIFT + C + LMB] Circular Radius Snap [t] Path Type");
            }
        } else if PLATFORM_EDIT_STATE_TEST == iPlatformEditState {
            g_map.update_platforms();

            //Platforms are drawn inside drawmap(...)
            drawmap(false, TILESIZE as i16, true);

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Check Paths: [esc] Exit");
        }

        draw_message();
        EDITOR_PLATFORM
    }
}

fn path_type_from(v: u8) -> PlatformPathType {
    match v {
        0 => PlatformPathType::Straight,
        1 => PlatformPathType::StraightContinuous,
        2 => PlatformPathType::Ellipse,
        _ => PlatformPathType::Falling,
    }
}

pub fn display_platform_preview(iPlatformId: i16, iMouseX: i16, iMouseY: i16) {
    unsafe {
        let srcRect = rect(0, 0, 160, 120);
        let mut dstRect = rect(iMouseX as i32, iMouseY as i32, 160, 120);
        SDL_UpperBlit(g_Platforms[iPlatformId as usize].preview, &srcRect, screen, &mut dstRect);
    }
}

pub fn switch_platforms(iPlatformId1: i16, iPlatformId2: i16) {
    unsafe {
        let mut tempPlatform = MapPlatform::new();
        let (a, b) = (iPlatformId1 as usize, iPlatformId2 as usize);
        copy_platform(&mut tempPlatform, &g_Platforms[a]);
        let from = Ptr::from_mut(&mut g_Platforms[b]);
        copy_platform(&mut g_Platforms[a], &from);
        copy_platform(&mut g_Platforms[b], &tempPlatform);

        g_Platforms[a].update_preview();
        g_Platforms[b].update_preview();
    }
}

pub fn copy_platform(toPlatform: &mut MapPlatform, fromPlatform: &MapPlatform) {
    toPlatform.fAngle = fromPlatform.fAngle;
    toPlatform.fRadiusX = fromPlatform.fRadiusX;
    toPlatform.fRadiusY = fromPlatform.fRadiusY;
    toPlatform.iDrawLayer = fromPlatform.iDrawLayer;
    toPlatform.iEndX = fromPlatform.iEndX;
    toPlatform.iEndY = fromPlatform.iEndY;
    toPlatform.iPathType = fromPlatform.iPathType;
    toPlatform.iStartX = fromPlatform.iStartX;
    toPlatform.iStartY = fromPlatform.iStartY;
    toPlatform.iVelocity = fromPlatform.iVelocity;

    for iRow in 0..MH {
        for iCol in 0..MW {
            toPlatform.tiles[iCol * MH + iRow] = fromPlatform.tiles[iCol * MH + iRow];
            toPlatform.types[iCol * MH + iRow] = fromPlatform.types[iCol * MH + iRow];
        }
    }
}

pub fn update_platform_path_start(iEditPlatform_: i16, mut iClickX: i16, mut iClickY: i16, fSnapToTile: bool) {
    unsafe {
        if !ignoreclick {
            if fSnapToTile {
                //Round off coord to nearest tile
                iClickX &= !15;
                iClickY &= !15;
            }

            g_Platforms[iEditPlatform_ as usize].iStartX = iClickX;
            g_Platforms[iEditPlatform_ as usize].iStartY = iClickY;
        }
    }
}

pub fn update_platform_path_end(iEditPlatform_: i16, mut iClickX: i16, mut iClickY: i16, fSnapToTile: bool) {
    unsafe {
        if !ignoreclick {
            if fSnapToTile {
                //Round off coord to nearest tile
                iClickX &= !15;
                iClickY &= !15;
            }

            g_Platforms[iEditPlatform_ as usize].iEndX = iClickX;
            g_Platforms[iEditPlatform_ as usize].iEndY = iClickY;
        }
    }
}

/// The snap used by the platform path and map hazard editors: round to the nearest of 16 sectors.
fn snap_angle(mut fAngle: f32) -> f32 {
    let dSector: f32 = TWO_PI / 16.0;
    fAngle += TWO_PI / 32.0;

    if fAngle > TWO_PI {
        fAngle -= TWO_PI;
    }

    for iSector in 0..16i16 {
        if fAngle >= dSector * iSector as f32 && fAngle < dSector * (iSector + 1) as f32 {
            fAngle = dSector * iSector as f32;
            break;
        }
    }
    fAngle
}

pub fn update_platform_path_angle(iEditPlatform_: i16, iClickX: i16, iClickY: i16, fSnapToAngle: bool) {
    unsafe {
        if !ignoreclick {
            let p = &mut g_Platforms[iEditPlatform_ as usize];
            let iDiffX: i32 = iClickX as i32 - p.iStartX as i32;
            let iDiffY: i32 = iClickY as i32 - p.iStartY as i32;

            let mut fAngle: f32 = (iDiffY as f32).atan2(iDiffX as f32);

            if fAngle < 0.0 {
                fAngle += TWO_PI;
            }

            if fSnapToAngle {
                fAngle = snap_angle(fAngle);
            }

            p.fAngle = fAngle;
        }
    }
}

pub fn update_platform_path_radius(iEditPlatform_: i16, iClickX: i16, iClickY: i16, fSnapToRadius: bool, fIsRadiusX: bool, fSetCircle: bool) {
    unsafe {
        if !ignoreclick {
            let p = &mut g_Platforms[iEditPlatform_ as usize];
            if fSetCircle {
                let iDiffX: i16 = (iClickX as i32 - p.iStartX as i32) as i16;
                let iDiffY: i16 = (iClickY as i32 - p.iStartY as i32) as i16;

                let mut iRadius: i16 = ((iDiffX as i32 * iDiffX as i32 + iDiffY as i32 * iDiffY as i32) as f64).sqrt() as i16;

                if fSnapToRadius {
                    iRadius &= !15;
                }

                p.fRadiusX = iRadius as f32;
                p.fRadiusY = p.fRadiusX;
            } else if fIsRadiusX {
                let mut iRadiusX: i16 = (iClickX as i32 - p.iStartX as i32).abs() as i16;

                if fSnapToRadius {
                    iRadiusX &= !15;
                }

                p.fRadiusX = iRadiusX as f32;
            } else {
                let mut iRadiusY: i16 = (iClickY as i32 - p.iStartY as i32).abs() as i16;

                if fSnapToRadius {
                    iRadiusY &= !15;
                }

                p.fRadiusY = iRadiusY as f32;
            }
        }
    }
}

/// `draw_platform`: draws the unfinished platform (complete map width x height grid).
pub fn draw_platform_unfinished(iPlatform: i16, fDrawTileTypes: bool) {
    unsafe {
        let p = iPlatform as usize;
        for iCol in 0..MAPWIDTH as i16 {
            for iRow in 0..MAPHEIGHT as i16 {
                let idx = iCol as usize * MH + iRow as usize;
                let tile = g_Platforms[p].tiles[idx];

                if tile.iID >= 0 {
                    g_tilesetmanager.draw(screen, tile.iID, 0, tile.iCol, tile.iRow, iCol, iRow);
                } else if tile.iID as i32 == TILESETANIMATED {
                    let mut iSrcCol: i16 = ((tile.iCol as i32) << 2) as i16;
                    let mut iSrcRow: i16 = tile.iRow;

                    if iSrcCol > 31 || iSrcCol < 0 || iSrcRow > 31 || iSrcRow < 0 {
                        iSrcCol = 0;
                        iSrcRow = 0;
                    }

                    let src = g_tilesetmanager.rect(0, iSrcCol, iSrcRow);
                    let dst = g_tilesetmanager.rect(0, iCol, iRow);
                    SDL_UpperBlit(rm.spr_tileanimation[0].get_surface(), src, screen, dst);
                } else if tile.iID as i32 == TILESETUNKNOWN {
                    let src = g_tilesetmanager.rect(0, 0, 0);
                    let dst = g_tilesetmanager.rect(0, iCol, iRow);
                    SDL_UpperBlit(rm.spr_unknowntile[0].get_surface(), src, screen, dst);
                }

                if fDrawTileTypes {
                    let t = g_Platforms[p].types[idx];
                    rm.spr_transparenttiles.draw_src(iCol as i32 * TILESIZE, iRow as i32 * TILESIZE, &rect(prev_tile_type(t).0 as i32 * TILESIZE, 0, TILESIZE, TILESIZE));
                }
            }
        }
    }
}

const MAPHAZARD_EDIT_STATE_SELECT: i16 = 0;
const MAPHAZARD_EDIT_STATE_TYPE: i16 = 1;
const MAPHAZARD_EDIT_STATE_LOCATION: i16 = 2;
const MAPHAZARD_EDIT_STATE_PROPERTIES: i16 = 3;

pub static mut iEditState: i16 = MAPHAZARD_EDIT_STATE_SELECT;
pub static mut iEditMapHazard: i16 = 0;

pub static mut rBackground: [SDL_Rect; 2] = [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2];
pub static mut rIconRects: [[SDL_Rect; 2]; MAXMAPHAZARDS as usize] = [[SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2]; MAXMAPHAZARDS as usize];

const szHazardNames: [&str; 8] = ["Fireballs", "Rotodisc", "Bullet Bill", "Flame Thrower", "Green Pirhana", "Red Pirhana", "Tall Pirhana", "Short Pirhana"];

pub static mut editor_maphazards_initialized: bool = false;
pub fn init_editor_maphazards() {
    unsafe {
        if editor_maphazards_initialized {
            return;
        }

        rBackground[0] = rect(0, 0, 256, 224);
        rBackground[1] = rect(320 - (rBackground[0].w >> 1), 240 - (rBackground[0].h >> 1), 256, 224);

        rNewButton[0] = rect(0, 352, 76, 32);
        rNewButton[1] = rect(rBackground[1].x + (rBackground[1].w >> 1) - (rNewButton[0].w >> 1), rBackground[1].y + rBackground[1].h + 8, 76, 32);

        for iType in 0..8usize {
            rTypeButton[iType][0] = rect(0, 0, 192, 32);
            rTypeButton[iType][1] = rect(320 - (rTypeButton[iType][0].w >> 1), 84 + iType as i32 * 40, 192, 32);
            rTypeButton[iType][2] = rect(24 * iType as i32, 32, 24, 24);
            rTypeButton[iType][3] = rect(rTypeButton[iType][1].x + 8, rTypeButton[iType][1].y + 4, 24, 24);
        }

        //Setup Map Hazard Icons
        for iMapHazard in 0..MAXMAPHAZARDS {
            let i = iMapHazard as usize;
            rIconRects[i][0] = rect((iMapHazard % 8) * 32, (iMapHazard / 8) * 32 + 224, 32, 32);
            rIconRects[i][1] = rect((iMapHazard % 6) * 40 + 204, (iMapHazard / 6) * 40 + rBackground[1].y + 16, 32, 32);
        }

        editor_maphazards_initialized = true;
    }
}

pub fn editor_maphazards() -> i32 {
    init_editor_maphazards();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            let ty = event.type_;
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                let sym = event.key.keysym.sym;
                let eh = iEditMapHazard as usize;
                if sym == k(SDLK_s) {
                    savecurrentmap();
                } else if sym == k(SDLK_l) {
                    if MAPHAZARD_EDIT_STATE_SELECT != iEditState {
                        iEditState = MAPHAZARD_EDIT_STATE_LOCATION;
                    }
                } else if sym == k(SDLK_p) {
                    if MAPHAZARD_EDIT_STATE_SELECT != iEditState {
                        iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                    }
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_9) {
                    if MAPHAZARD_EDIT_STATE_SELECT == iEditState {
                        let iHazard = (sym - k(SDLK_1)) as i16;
                        if iHazard < g_map.maphazards.len() as i16 {
                            iEditMapHazard = iHazard;
                            iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                        }
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype == 0 || hazard.itype == 1 {
                            hazard.iparam[0] = (sym - k(SDLK_1) + 1) as i16;
                        }
                    }
                } else if sym == k(SDLK_DELETE) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState || MAPHAZARD_EDIT_STATE_LOCATION == iEditState {
                        //Copy platforms into empty spot
                        let mut iMapHazard = iEditMapHazard as usize;
                        while iMapHazard < g_map.maphazards.len().wrapping_sub(1) {
                            g_map.maphazards[iMapHazard].itype = g_map.maphazards[iMapHazard + 1].itype;
                            g_map.maphazards[iMapHazard].ix = g_map.maphazards[iMapHazard + 1].ix;
                            g_map.maphazards[iMapHazard].iy = g_map.maphazards[iMapHazard + 1].iy;

                            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                                g_map.maphazards[iMapHazard].iparam[iParam] = g_map.maphazards[iMapHazard + 1].iparam[iParam];
                                g_map.maphazards[iMapHazard].dparam[iParam] = g_map.maphazards[iMapHazard + 1].dparam[iParam];
                            }
                            iMapHazard += 1;
                        }

                        g_map.maphazards.pop();
                        iEditState = MAPHAZARD_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_ESCAPE) {
                    if MAPHAZARD_EDIT_STATE_SELECT == iEditState {
                        editor_maphazards_initialized = false;
                        return EDITOR_EDIT;
                    } else if MAPHAZARD_EDIT_STATE_LOCATION == iEditState || MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        iEditState = MAPHAZARD_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_KP_MINUS) || sym == k(SDLK_MINUS) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype == 0 || hazard.itype == 1 {
                            if hazard.dparam[0] > -0.05f32 {
                                hazard.dparam[0] -= 0.005f32;
                            }
                        } else if hazard.itype == 2 {
                            if hazard.dparam[0] > -10.0f32 {
                                hazard.dparam[0] -= 1.0f32;
                            }

                            if hazard.dparam[0] == 0.0f32 {
                                hazard.dparam[0] -= 1.0f32;
                            }
                        }
                    }
                } else if sym == k(SDLK_KP_PLUS) || sym == k(SDLK_EQUALS) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype == 0 || hazard.itype == 1 {
                            if hazard.dparam[0] < 0.05f32 {
                                hazard.dparam[0] += 0.005f32;
                            }
                        } else if hazard.itype == 2 {
                            if hazard.dparam[0] < 10.0f32 {
                                hazard.dparam[0] += 1.0f32;
                            }

                            if hazard.dparam[0] == 0.0f32 {
                                hazard.dparam[0] += 1.0f32;
                            }
                        }
                    }
                } else if sym == k(SDLK_COMMA) || sym == k(SDLK_LEFTBRACKET) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype >= 2 && hazard.itype <= 7 {
                            if hazard.iparam[0] > 30 {
                                hazard.iparam[0] -= 30;
                            }
                        }
                    }
                } else if sym == k(SDLK_PERIOD) || sym == k(SDLK_RIGHTBRACKET) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype >= 2 && hazard.itype <= 7 {
                            if hazard.iparam[0] < 480 {
                                hazard.iparam[0] += 30;
                            }
                        }
                    }
                } else if sym == k(SDLK_d) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = &mut g_map.maphazards[eh];

                        if hazard.itype >= 3 && hazard.itype <= 7 {
                            hazard.iparam[1] += 1;
                            if hazard.iparam[1] > 3 {
                                hazard.iparam[1] = 0;
                            }
                        } else if hazard.itype == 2 {
                            hazard.dparam[0] = -hazard.dparam[0];
                        }
                    }
                } else if sym == k(SDLK_n) {
                    if MAPHAZARD_EDIT_STATE_SELECT == iEditState {
                        iEditMapHazard = new_map_hazard();
                        iEditState = MAPHAZARD_EDIT_STATE_TYPE;
                    }
                }
            } else if ty == SDL_MOUSEBUTTONDOWN_EV {
                if event.button.button == BUTTON_LEFT && !ignoreclick {
                    let iClickX = event.button.x as i16;
                    let iClickY = event.button.y as i16;
                    let (cx, cy) = (iClickX as i32, iClickY as i32);

                    if MAPHAZARD_EDIT_STATE_SELECT == iEditState {
                        //check clicks on existing platforms
                        for iMapHazard in 0..g_map.maphazards.len() {
                            let ic = rIconRects[iMapHazard][1];
                            if cx >= ic.x && cx < ic.x + ic.w && cy >= ic.y && cy < ic.y + ic.h {
                                iEditMapHazard = iMapHazard as i16;
                                iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                                ignoreclick = true;

                                break;
                            }
                        }

                        //check click on the new button
                        if (g_map.maphazards.len() as i32) < MAXMAPHAZARDS
                            && cx >= rNewButton[1].x
                            && cx < rNewButton[1].x + rNewButton[1].w
                            && cy >= rNewButton[1].y
                            && cy < rNewButton[1].y + rNewButton[1].h
                        {
                            iEditMapHazard = new_map_hazard();
                            iEditState = MAPHAZARD_EDIT_STATE_TYPE;
                            ignoreclick = true;
                        }
                    } else if MAPHAZARD_EDIT_STATE_TYPE == iEditState && !ignoreclick {
                        for iType in 0..8usize {
                            let tb = rTypeButton[iType][1];
                            if cx >= tb.x && cx < tb.x + tb.w && cy >= tb.y && cy < tb.y + tb.h {
                                let hazard = &mut g_map.maphazards[iEditMapHazard as usize];
                                hazard.itype = iType as i16;
                                iEditState = MAPHAZARD_EDIT_STATE_LOCATION;
                                ignoreclick = true;

                                //Set some default values for the selected type
                                if iType == 0 {
                                    hazard.iparam[0] = 5; //Number of fireballs in string
                                    hazard.dparam[0] = 0.02f32; //Angular velocity
                                    hazard.dparam[1] = 0.0f32; //Start Angle
                                } else if iType == 1 {
                                    hazard.iparam[0] = 2; //Number of fireballs in string
                                    hazard.dparam[0] = 0.02f32; //Angular velocity
                                    hazard.dparam[1] = 0.0f32; //Start Angle
                                    hazard.dparam[2] = 112.0f32; //Radius
                                } else if iType == 2 {
                                    hazard.iparam[0] = 120; //Frequency
                                    hazard.dparam[0] = 2.0f32; //Velocity
                                } else if iType == 3 {
                                    hazard.iparam[0] = 120; //Frequency
                                    hazard.iparam[1] = 0; //Direction
                                } else if (4..=7).contains(&iType) {
                                    hazard.iparam[0] = 300; //Frequency
                                    hazard.iparam[1] = 0; //Direction
                                }

                                break;
                            }
                        }
                    } else if MAPHAZARD_EDIT_STATE_LOCATION == iEditState && !ignoreclick {
                        let hazard = &mut g_map.maphazards[iEditMapHazard as usize];
                        hazard.ix = iClickX / 16;
                        hazard.iy = iClickY / 16;
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState && !ignoreclick {
                        let hazard = Ptr::from_mut(&mut g_map.maphazards[iEditMapHazard as usize]);
                        let iType = hazard.itype;
                        if iType == 0 {
                            //Edit fireball string
                            adjust_map_hazard_radius(hazard.get(), iClickX, iClickY);
                        } else if iType == 1 {
                            //rotodisc
                            adjust_map_hazard_radius(hazard.get(), iClickX, iClickY);
                        }
                    }
                } else {
                    fall = true;
                }
            }

            if fall || ty == SDL_MOUSEMOTION_EV {
                bound_mouse_motion_coords();
                let iClickX = bound_to_window_w(event.motion.x) as i16;
                let iClickY = bound_to_window_h(event.motion.y) as i16;

                if event.motion.state == sdl_button(SDL_BUTTON_LEFT) && !ignoreclick {
                    if MAPHAZARD_EDIT_STATE_LOCATION == iEditState && !ignoreclick {
                        let hazard = &mut g_map.maphazards[iEditMapHazard as usize];
                        hazard.ix = iClickX / 16;
                        hazard.iy = iClickY / 16;
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
                        let hazard = Ptr::from_mut(&mut g_map.maphazards[iEditMapHazard as usize]);
                        let iType = hazard.itype;
                        if iType == 0 || iType == 1 {
                            adjust_map_hazard_radius(hazard.get(), iClickX, iClickY);
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if event.button.button == BUTTON_LEFT {
                    ignoreclick = false;
                }
            }
        }

        //Draw platform editing
        drawmap(false, TILESIZE as i16, false);

        rm.menu_shade.draw(0, 0);

        if MAPHAZARD_EDIT_STATE_SELECT == iEditState {
            let s = rBackground[0];
            SDL_UpperBlit(s_platform, &s, screen, &mut rBackground[1]);

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Map Hazard Mode: [esc] Exit");

            for iMapHazard in 0..g_map.maphazards.len() {
                let s = rIconRects[iMapHazard][0];
                SDL_UpperBlit(s_platform, &s, screen, &mut rIconRects[iMapHazard][1]);
            }

            if (g_map.maphazards.len() as i32) < MAXMAPHAZARDS {
                let s = rNewButton[0];
                SDL_UpperBlit(s_platform, &s, screen, &mut rNewButton[1]);
            }

            rm.menu_font_small.draw_centered(320, rBackground[1].y - 18, "Hazards");
        } else if MAPHAZARD_EDIT_STATE_TYPE == iEditState {
            //Draw map hazard options
            for iType in 0..8usize {
                let s0 = rTypeButton[iType][0];
                SDL_UpperBlit(s_maphazardbuttons, &s0, screen, &mut rTypeButton[iType][1]);
                let s2 = rTypeButton[iType][2];
                SDL_UpperBlit(s_maphazardbuttons, &s2, screen, &mut rTypeButton[iType][3]);

                rm.menu_font_large.draw(rTypeButton[iType][1].x + 36, rTypeButton[iType][1].y + 6, szHazardNames[iType]);
            }

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Choose Hazard Type");
        } else if MAPHAZARD_EDIT_STATE_LOCATION == iEditState {
            let hazard = g_map.maphazards[iEditMapHazard as usize];
            draw_map_hazard(&hazard, 0, true);
            draw_map_hazard_controls(&hazard);

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Location: [esc] Exit, [p] Properties, [LMB] Set Location");
        } else if MAPHAZARD_EDIT_STATE_PROPERTIES == iEditState {
            let hazard = g_map.maphazards[iEditMapHazard as usize];
            draw_map_hazard(&hazard, 0, true);
            draw_map_hazard_controls(&hazard);

            if hazard.itype == 0 || hazard.itype == 1 {
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 3, "Properties");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 2, "[esc] Exit, [l] Location, [+/-] Velocity, [LMB] Angle and Radius");

                if hazard.itype == 1 {
                    rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[Shift + LMB] Snap To Angle, [1-9] Number of Rotodiscs");
                } else {
                    rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[Shift + LMB] Snap To Angle");
                }
            } else if hazard.itype == 2 {
                rm.menu_font_small.draw(0, 480 - (rm.menu_font_small.get_height() << 1), "Properties: [esc] Exit, [l] Location, [d] Direction");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[-/+] Velocity, [[/]] or [</>] Frequency");
            } else if hazard.itype >= 3 && hazard.itype <= 7 {
                rm.menu_font_small.draw(0, 480 - (rm.menu_font_small.get_height() << 1), "Properties: [esc] Exit, [l] Location");
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[[/]] or [</>] Frequency, [d] direction");
            } else {
                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Properties: [esc] Exit, [l] Location");
            }
        }

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        draw_message();
        EDITOR_MAPHAZARDS
    }
}

pub fn new_map_hazard() -> i16 {
    unsafe {
        let mut hazard = MapHazard::default();
        hazard.itype = 0;
        hazard.ix = 10;
        hazard.iy = 7;

        for iMapHazard in 0..NUMMAPHAZARDPARAMS as usize {
            hazard.iparam[iMapHazard] = 0;
            hazard.dparam[iMapHazard] = 0.0;
        }

        g_map.maphazards.push(hazard);
        (g_map.maphazards.len() - 1) as i16
    }
}

pub fn draw_map_hazard_controls(hazard: &MapHazard) {
    unsafe {
        if hazard.itype == 0 || hazard.itype == 1 || hazard.itype == 2 {
            let iVelMarkerX: i16 = if hazard.itype == 2 {
                ((hazard.dparam[0] + 10.0f32) as i16 as i32 * 12 + 196) as i16
            } else {
                (((hazard.dparam[0] + 0.05f32) / 0.005f32) as i16 as i32 * 12 + 196) as i16
            };

            let rVel = [rect(0, 400, 244, 17), rect(198, 420, 244, 17)];
            let mut d = rVel[1];
            SDL_UpperBlit(s_platform, &rVel[0], screen, &mut d);

            let rMarker = [rect(244, 400, 8, 18), rect(iVelMarkerX as i32, 418, 8, 18)];
            let mut d = rMarker[1];
            SDL_UpperBlit(s_platform, &rMarker[0], screen, &mut d);

            if hazard.itype == 2 {
                rm.menu_font_small.draw_right_justified(190, 420, "Left");
                rm.menu_font_small.draw(450, 420, "Right");
            } else {
                rm.menu_font_small.draw_right_justified(190, 420, "Counter Clockwise");
                rm.menu_font_small.draw(450, 420, "Clockwise");
            }
        }

        if hazard.itype >= 2 && hazard.itype <= 7 {
            // Draw frequency for bullet bill and flame cannon
            let iFreqMarkerX: i16 = (((hazard.iparam[0] as i32 / 30) - 1) * 12 + 196) as i16;

            let rVel = [rect(0, 384, 184, 13), rect(198, 390, 184, 13)];
            let mut d = rVel[1];
            SDL_UpperBlit(s_platform, &rVel[0], screen, &mut d);

            let rMarker = [rect(244, 400, 8, 18), rect(iFreqMarkerX as i32, 388, 8, 18)];
            let mut d = rMarker[1];
            SDL_UpperBlit(s_platform, &rMarker[0], screen, &mut d);

            rm.menu_font_small.draw_right_justified(190, 390, "More Frequent");
            rm.menu_font_small.draw(388, 390, "Less Frequent");
        }
    }
}

pub fn adjust_map_hazard_radius(hazard: &mut MapHazard, iClickX: i16, iClickY: i16) {
    let iDiffX: i16 = (iClickX as i32 - (((hazard.ix as i32) << 4) + 16)) as i16;
    let iDiffY: i16 = (iClickY as i32 - (((hazard.iy as i32) << 4) + 16)) as i16;

    let radius: f32 = ((iDiffX as i32 * iDiffX as i32 + iDiffY as i32 * iDiffY as i32) as f64).sqrt() as f32;
    let mut angle: f32 = (iDiffY as f64).atan2(iDiffX as f64) as f32;

    if angle < 0.0f32 {
        angle += TWO_PI;
    }

    let keystate = get_keyboard_state();
    if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
        angle = snap_angle(angle);
    }

    if hazard.itype == 0 {
        if radius >= 24.0f32 {
            hazard.iparam[0] = (radius as i16 as i32 / 24 + 1) as i16;
            hazard.dparam[1] = angle;
        } else {
            hazard.iparam[0] = 1;
            hazard.dparam[1] = 0.0f32;
        }
    } else if hazard.itype == 1 {
        hazard.dparam[1] = angle;

        if radius > 32.0f32 {
            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                //Snap radius to every 16 pixels
                hazard.dparam[2] = ((((radius - 16.0f32) as i32) >> 4) << 4) as f32;
            } else {
                hazard.dparam[2] = radius - 16.0f32;
            }
        } else {
            hazard.dparam[2] = 16.0f32;
        }
    }
}

pub static mut tileset: Ptr<CTileset> = Ptr::null();
pub static mut view_tileset_repeat_direction: i16 = -1;
pub static mut view_tileset_repeat_timer: i16 = 0;

pub static mut editor_tiles_initialized: bool = false;
pub fn init_editor_tiles() {
    unsafe {
        if editor_tiles_initialized {
            return;
        }

        set_tile_drag = false;
        view_tileset_repeat_direction = -1;
        view_tileset_repeat_timer = 0;
        tileset = g_tilesetmanager.tileset(set_tile_tileset as usize);

        editor_tiles_initialized = true;
    }
}

pub fn editor_tiles() -> i32 {
    init_editor_tiles();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    if !set_tile_drag {
                        let sym = event.key.keysym.sym;
                        if sym >= k(SDLK_1) && sym <= k(SDLK_9) && (sym as i64) < k(SDLK_1) as i64 + g_tilesetmanager.count() as i64 {
                            set_tile_tileset = sym - k(SDLK_1);
                            tileset = g_tilesetmanager.tileset(set_tile_tileset as usize);
                            view_tileset_x = 0;
                            view_tileset_y = 0;
                        } else if sym == k(SDLK_PAGEUP) {
                            if set_tile_tileset > 0 {
                                set_tile_tileset -= 1;
                                tileset = g_tilesetmanager.tileset(set_tile_tileset as usize);
                                view_tileset_x = 0;
                                view_tileset_y = 0;
                            }
                        } else if sym == k(SDLK_PAGEDOWN) {
                            if (set_tile_tileset as i64) < g_tilesetmanager.count() as i64 - 1 {
                                set_tile_tileset += 1;
                                tileset = g_tilesetmanager.tileset(set_tile_tileset as usize);
                                view_tileset_x = 0;
                                view_tileset_y = 0;
                            }
                        } else if sym == k(SDLK_UP) {
                            if view_tileset_y > 0 {
                                view_tileset_y -= 1;
                                view_tileset_repeat_direction = 0;
                                view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_DOWN) {
                            if view_tileset_y < g_tilesetmanager.tileset(set_tile_tileset as usize).height() as i32 - 15 {
                                view_tileset_y += 1;
                                view_tileset_repeat_direction = 1;
                                view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_LEFT) {
                            if view_tileset_x > 0 {
                                view_tileset_x -= 1;
                                view_tileset_repeat_direction = 2;
                                view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_RIGHT) {
                            if view_tileset_x < g_tilesetmanager.tileset(set_tile_tileset as usize).width() as i32 - 20 {
                                view_tileset_x += 1;
                                view_tileset_repeat_direction = 3;
                                view_tileset_repeat_timer = 30;
                            }
                        } else {
                            edit_mode = 1; //change to edit mode using tiles
                            set_tile_drag = false;
                            editor_tiles_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_KEYUP_EV => {
                    let sym = event.key.keysym.sym;
                    if sym == k(SDLK_UP) || sym == k(SDLK_DOWN) || sym == k(SDLK_LEFT) || sym == k(SDLK_RIGHT) {
                        view_tileset_repeat_direction = -1;
                        view_tileset_repeat_timer = 0;
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    let iCol = (event.button.x / TILESIZE + view_tileset_x) as i16;
                    let iRow = (event.button.y / TILESIZE + view_tileset_y) as i16;

                    if event.button.button == BUTTON_LEFT {
                        let tileset_ = g_tilesetmanager.tileset(set_tile_tileset as usize);

                        if iCol < tileset_.width() && iRow < tileset_.height() {
                            set_tile_start_x = iCol as i32;
                            set_tile_start_y = iRow as i32;
                            set_tile_end_x = set_tile_start_x;
                            set_tile_end_y = set_tile_start_y;

                            set_tile_drag = true;
                        }
                    } else if event.button.button == BUTTON_RIGHT {
                        set_type = tileset.get().increment_tile_type(iCol as isize as usize, iRow as isize as usize);
                    } else if event.button.button == BUTTON_MIDDLE {
                        set_type = tileset.get().decrement_tile_type(iCol as isize as usize, iRow as isize as usize);
                    }
                }

                SDL_MOUSEBUTTONUP_EV => {
                    let iCol = (event.button.x / TILESIZE + view_tileset_x) as i16;
                    let iRow = (event.button.y / TILESIZE + view_tileset_y) as i16;

                    if event.button.button == BUTTON_LEFT {
                        if iCol < tileset.width() && iRow < tileset.height() {
                            set_tile_cols = set_tile_end_x - set_tile_start_x + 1;
                            set_tile_rows = set_tile_end_y - set_tile_start_y + 1;

                            set_tile_drag = false;
                            edit_mode = 1; //change to edit mode using tiles
                            editor_tiles_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_MOUSEMOTION_EV => {
                    bound_mouse_motion_coords();
                    let iCol = (bound_to_window_w(event.motion.x) / TILESIZE + view_tileset_x) as i16;
                    let iRow = (bound_to_window_h(event.motion.y) / TILESIZE + view_tileset_y) as i16;

                    if iCol < tileset.width() && iRow < tileset.height() {
                        if event.motion.state == sdl_button(SDL_BUTTON_LEFT) {
                            if (iCol as i32) < set_tile_start_x {
                                set_tile_start_x = iCol as i32;
                            }

                            if iCol as i32 > set_tile_end_x {
                                set_tile_end_x = iCol as i32;
                            }

                            if (iRow as i32) < set_tile_start_y {
                                set_tile_start_y = iRow as i32;
                            }

                            if iRow as i32 > set_tile_end_y {
                                set_tile_end_y = iRow as i32;
                            }
                        } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) || event.motion.state == sdl_button(SDL_BUTTON_MIDDLE) {
                            tileset.get().set_tile_type(iCol as isize as usize, iRow as isize as usize, set_type);
                        }
                    }
                }

                _ => {}
            }
        }

        //Allow auto-scrolling of tilesets when the arrow keys are held down
        if view_tileset_repeat_direction >= 0 && view_tileset_repeat_timer > 0 {
            view_tileset_repeat_timer -= 1;
            if view_tileset_repeat_timer <= 0 {
                view_tileset_repeat_timer = 5;

                let ts = g_tilesetmanager.tileset(set_tile_tileset as usize);
                if view_tileset_repeat_direction == 0 && view_tileset_y > 0 {
                    view_tileset_y -= 1;
                } else if view_tileset_repeat_direction == 1 && view_tileset_y < ts.height() as i32 - 15 {
                    view_tileset_y += 1;
                } else if view_tileset_repeat_direction == 2 && view_tileset_x > 0 {
                    view_tileset_x -= 1;
                } else if view_tileset_repeat_direction == 3 && view_tileset_x < ts.width() as i32 - 20 {
                    view_tileset_x += 1;
                }
            }
        }

        SDL_FillRect(screen, null(), 0xFF888888);

        let rectSrc = rect(
            view_tileset_x << 5,
            view_tileset_y << 5,
            if tileset.width() > 20 { 640 } else { (tileset.width() as i32) << 5 },
            if tileset.height() > 15 { 480 } else { (tileset.height() as i32) << 5 },
        );

        r = rect(0, 0, 640, 480);

        SDL_UpperBlit(g_tilesetmanager.tileset(set_tile_tileset as usize).surface(0), &rectSrc, screen, &mut r);
        rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), tileset.name());

        let mut i = view_tileset_x;
        while i < view_tileset_x + 20 && i < tileset.width() as i32 {
            let mut j = view_tileset_y;
            while j < view_tileset_y + 15 && j < tileset.height() as i32 {
                let t = tileset.tile_type(i as usize, j as usize);
                if t != TileType::NonSolid {
                    rm.spr_tiletypes.draw_src((i - view_tileset_x) << 5, (j - view_tileset_y) << 5, &rect((prev_tile_type(t).0 as i32) << 3, 0, 8, 8));
                }
                j += 1;
            }
            i += 1;
        }

        if set_tile_drag {
            for i in set_tile_start_x..=set_tile_end_x {
                for j in set_tile_start_y..=set_tile_end_y {
                    rm.spr_selectedtile.draw((i - view_tileset_x) << 5, (j - view_tileset_y) << 5);
                }
            }
        }

        draw_message();
        EDITOR_TILES
    }
}

pub fn editor_blocks() -> i32 {
    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    edit_mode = 0;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if event.button.button == BUTTON_LEFT {
                        let set_block_x = (event.button.x / TILESIZE) as i16;
                        let set_block_y = (event.button.y / TILESIZE) as i16;

                        //Set the selected block to one of the interaction blocks
                        if set_block_y == 0 && set_block_x >= 0 && set_block_x <= 6 {
                            set_block = set_block_x as i32;
                        } else if set_block_y == 0 && set_block_x >= 7 && set_block_x <= 11 {
                            set_block = set_block_x as i32 + 8;
                        } else if set_block_y >= 1 && set_block_y <= 2 && set_block_x >= 0 && set_block_x <= 3 {
                            //set the selected block to an on/off switch block

                            set_block = set_block_x as i32 + 7;
                            g_map.iSwitches[set_block_x as usize] = 2 - set_block_y;
                        } else if set_block_y >= 1 && set_block_y <= 2 && set_block_x >= 4 && set_block_x <= 7 {
                            //set the selected block to a switch block

                            set_block = set_block_x as i32 + 7;
                            set_block_switch_on = (set_block_y == 1) as i32;
                        } else if set_block_y == 3 && set_block_x >= 0 && set_block_x <= 9 {
                            //set the selected block to a weapon breakable block

                            set_block = set_block_x as i32 + 20;
                        }

                        edit_mode = 0;

                        //The user must release the mouse button before trying to add a tile
                        ignoreclick = true;
                        return EDITOR_EDIT;
                    }
                }

                _ => {}
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        rm.spr_blocks[0].draw_src_to(&rect(0, 0, 224, 32), screen, &rect(0, 0, 224, 32));
        rm.spr_blocks[0].draw_src_to(&rect(224, 0, 128, 64), screen, &rect(0, 32, 128, 64));
        rm.spr_blocks[0].draw_src_to(&rect(352, 0, 128, 64), screen, &rect(128, 32, 128, 64));
        rm.spr_blocks[0].draw_src_to(&rect(0, 32, 160, 32), screen, &rect(224, 0, 160, 32));
        rm.spr_blocks[0].draw_src_to(&rect(0, 64, 320, 32), screen, &rect(0, 96, 320, 32));

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        draw_message();
        EDITOR_BLOCKS
    }
}

pub fn editor_mapitems() -> i32 {
    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    edit_mode = 7;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if event.button.button == BUTTON_LEFT {
                        let set_item_x = (event.button.x / TILESIZE) as i16;
                        let set_item_y = (event.button.y / TILESIZE) as i16;

                        //Set the selected block to one of the interaction blocks
                        if set_item_y == 0 && set_item_x >= 0 && set_item_x <= 5 {
                            set_mapitem = set_item_x as i32;

                            edit_mode = 7;

                            //The user must release the mouse button before trying to add a tile
                            ignoreclick = true;
                            return EDITOR_EDIT;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        rm.spr_mapitems[0].draw_src(0, 0, &rect(0, 0, 192, 32));

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());
        rm.menu_font_small.draw_right_justified(0, 480 - rm.menu_font_small.get_height(), "Map Items");

        draw_message();
        EDITOR_MAPITEMS
    }
}

pub static mut modeitemmode: i16 = 0;
pub static mut dragmodeitem: i16 = -1;
pub static mut dragoffsetx: i16 = 0;
pub static mut dragoffsety: i16 = 0;

pub static mut editor_modeitems_initialized: bool = false;
pub fn init_editor_modeitems() {
    unsafe {
        if editor_modeitems_initialized {
            return;
        }

        modeitemmode = 0;
        dragmodeitem = -1;
        dragoffsetx = 0;
        dragoffsety = 0;

        editor_modeitems_initialized = true;
    }
}

pub fn editor_modeitems() -> i32 {
    init_editor_modeitems();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            let ty = event.type_;
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                dragmodeitem = -1;

                let sym = event.key.keysym.sym;
                if sym == k(SDLK_s) {
                    savecurrentmap();
                } else if sym == k(SDLK_ESCAPE) || sym == k(SDLK_j) {
                    editor_modeitems_initialized = false;
                    return EDITOR_EDIT;
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_2) {
                    modeitemmode = (sym - k(SDLK_1)) as i16;
                } else if sym == k(SDLK_r) {
                    //Set this mode item set to random
                    if modeitemmode == 0 {
                        if g_map.iNumRaceGoals == 0 {
                            g_map.iNumRaceGoals = MAXRACEGOALS as i16;
                        } else {
                            g_map.iNumRaceGoals = 0;
                        }
                    } else if modeitemmode == 1 {
                        if g_map.iNumFlagBases == 0 {
                            g_map.iNumFlagBases = 4;
                        } else {
                            g_map.iNumFlagBases = 0;
                        }
                    }
                }
                fall = true;
            }

            if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                if event.button.button == BUTTON_LEFT {
                    let iMouseX = event.button.x as i16;
                    let iMouseY = event.button.y as i16;

                    dragmodeitem = -1;
                    if modeitemmode == 0 {
                        for iGoal in 0..g_map.iNumRaceGoals {
                            let g = g_map.racegoallocations[iGoal as usize];
                            if iMouseX >= g.x && (iMouseX as i32) < g.x as i32 + 36 && iMouseY >= g.y && (iMouseY as i32) < g.y as i32 + 36 {
                                dragmodeitem = iGoal;
                                dragoffsetx = iMouseX - g.x;
                                dragoffsety = iMouseY - g.y;
                            }
                        }
                    } else if modeitemmode == 1 {
                        for iBase in 0..g_map.iNumFlagBases {
                            let b = g_map.flagbaselocations[iBase as usize];
                            if iMouseX >= b.x && (iMouseX as i32) < b.x as i32 + 32 && iMouseY >= b.y && (iMouseY as i32) < b.y as i32 + 32 {
                                dragmodeitem = iBase;
                                dragoffsetx = iMouseX - b.x;
                                dragoffsety = iMouseY - b.y;
                            }
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if event.button.button == BUTTON_LEFT {
                    dragmodeitem = -1;
                }
            } else if ty == SDL_MOUSEMOTION_EV {
                bound_mouse_motion_coords();
                if dragmodeitem >= 0 && event.motion.state == sdl_button(SDL_BUTTON_LEFT) {
                    let keystate = get_keyboard_state();
                    let fShiftDown = check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT);

                    let d = dragmodeitem as usize;
                    let loc = if modeitemmode == 0 {
                        Some(&mut g_map.racegoallocations[d])
                    } else if modeitemmode == 1 {
                        Some(&mut g_map.flagbaselocations[d])
                    } else {
                        None
                    };
                    if let Some(loc) = loc {
                        loc.x = (event.motion.x - dragoffsetx as i32) as i16;
                        loc.y = (event.motion.y - dragoffsety as i32) as i16;

                        if fShiftDown {
                            loc.x = ((event.motion.x >> 5) << 5) as i16;
                            loc.y = ((event.motion.y >> 5) << 5) as i16;
                        }
                    }
                }
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        //draw race goals
        if modeitemmode == 0 {
            if g_map.iNumRaceGoals == 0 {
                rm.menu_font_large.draw_centered(320, 200, "Race goals are set to random.");
                rm.menu_font_large.draw_centered(320, 220, "Press 'R' to manually set them.");
            } else {
                for iGoal in 0..g_map.iNumRaceGoals {
                    let g = g_map.racegoallocations[iGoal as usize];
                    rm.spr_racegoals.draw_src(g.x as i32 - 16, g.y as i32 - 18, &rect(0, 0, 68, 54));
                    let szNum = format!("{}", iGoal + 1);
                    rm.menu_font_large.draw_centered(g.x as i32 + 18, g.y as i32 + 6, &szNum);
                }
            }

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Set Race Goal Locations - Press [2] for Flag Bases");
        } else if modeitemmode == 1 {
            if g_map.iNumFlagBases == 0 {
                rm.menu_font_large.draw_centered(320, 200, "Flag bases are set to random.");
                rm.menu_font_large.draw_centered(320, 220, "Press 'R' to manually set them.");
            } else {
                for iBase in 0..g_map.iNumFlagBases {
                    let b = g_map.flagbaselocations[iBase as usize];
                    rm.spr_flagbases.draw_src(b.x as i32 - 8, b.y as i32 - 8, &rect(iBase as i32 * 48, 0, 48, 48));
                }
            }

            rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Set Flag Base Locations - Press [1] for Race Goals");
        }

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        draw_message();
        EDITOR_MODEITEMS
    }
}

pub fn editor_tiletype() -> i32 {
    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    edit_mode = 6;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if event.button.button == BUTTON_LEFT {
                        let iCol = (event.button.x / TILESIZE) as i16;
                        let iRow = (event.button.y / TILESIZE) as i16;

                        if (iCol as i32) < NUMTILETYPES - 1 && iRow == 0 {
                            set_tiletype = TileType::from_i32(iCol as i32 + 1);

                            edit_mode = 6;
                            ignoreclick = true;
                            return EDITOR_EDIT;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        rm.spr_transparenttiles.draw(0, 0);

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        draw_message();
        EDITOR_TILETYPE
    }
}

pub static mut iPage: i16 = 0;
pub static mut sBackgrounds: [*mut SDL_Surface; 16] = [null_mut(); 16];
pub static mut rSrc: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 160, h: 120 };
pub static mut rDst: [SDL_Rect; 16] = [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 16];

pub static mut editor_backgrounds_initialized: bool = false;
pub fn init_editor_backgrounds() {
    unsafe {
        if editor_backgrounds_initialized {
            return;
        }

        iPage = (backgroundlist.current_index() / 16) as i16;

        for iRectY in 0..4i32 {
            for iRectX in 0..4i32 {
                rDst[(iRectY * 4 + iRectX) as usize] = rect(iRectX * 160, iRectY * 120, 160, 120);
            }
        }

        for iSurface in 0..16usize {
            sBackgrounds[iSurface] = SDL_CreateRGBSurface((*screen).flags, 160, 120, 16, 0, 0, 0, 0);
        }

        load_background_page(iPage);

        editor_backgrounds_initialized = true;
    }
}

unsafe fn free_background_surfaces() {
    for iSurface in 0..16usize {
        SDL_FreeSurface(sBackgrounds[iSurface]);
    }
}

pub fn editor_backgrounds() -> i32 {
    init_editor_backgrounds();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => {
                    free_background_surfaces();

                    editor_backgrounds_initialized = false;
                    return EDITOR_EDIT;
                }

                SDL_KEYDOWN_EV => {
                    let sym = event.key.keysym.sym;
                    if sym == k(SDLK_ESCAPE) {
                        free_background_surfaces();

                        editor_backgrounds_initialized = false;
                        return EDITOR_EDIT;
                    } else if sym == k(SDLK_PAGEDOWN) || sym == k(SDLK_DOWN) {
                        if ((iPage as i32 + 1) * 16) < backgroundlist.count() as i32 {
                            iPage += 1;
                            load_background_page(iPage);
                        }
                    } else if sym == k(SDLK_PAGEUP) || sym == k(SDLK_UP) {
                        if (iPage as i32 - 1) * 16 >= 0 {
                            iPage -= 1;
                            load_background_page(iPage);
                        }
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if event.button.button == BUTTON_LEFT || event.button.button == BUTTON_RIGHT {
                        for iBackground in 0..16i32 {
                            if iPage as i32 * 16 + iBackground >= backgroundlist.count() as i32 {
                                break;
                            }

                            let d = rDst[iBackground as usize];
                            if event.button.x >= d.x && event.button.x < d.x + d.w && event.button.y >= d.y && event.button.y < d.y + d.h {
                                backgroundlist.set_current_index((iPage as i32 * 16 + iBackground) as usize);

                                rm.spr_background = SpriteBuilder::new(backgroundlist.current_path()).without_color_key().create();
                                g_map.szBackgroundFile = get_filename_from_path(&backgroundlist.current_path().to_string_lossy());

                                if event.button.button == BUTTON_LEFT {
                                    //Set music to background default
                                    set_music_category_from_background();
                                }

                                free_background_surfaces();

                                editor_backgrounds_initialized = false;
                                return EDITOR_EDIT;
                            }
                        }
                    }
                }

                SDL_MOUSEMOTION_EV => bound_mouse_motion_coords(),

                _ => {}
            }
        }

        let rc = rect(0, 0, 640, 480);
        SDL_FillRect(screen, &rc, 0x0);

        for iBackground in 0..16i32 {
            if iPage as i32 * 16 + iBackground >= backgroundlist.count() as i32 {
                break;
            }

            let s = rSrc;
            SDL_UpperBlit(sBackgrounds[iBackground as usize], &s, screen, &mut rDst[iBackground as usize]);
        }

        rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height() * 2, "[Page Up] next page, [Page Down] previous page");
        rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[LMB] choose background with music category, [RMB] choose just background");

        let iID = mouse_x / 160 + mouse_y / 120 * 4 + iPage as i32 * 16;

        if (iID as i64) < backgroundlist.count() as i64 {
            let name = backgroundlist.at(iID as usize).to_string_lossy().into_owned();
            rm.menu_font_small.draw(0, 0, &name);
        }

        draw_message();
        EDITOR_BACKGROUNDS
    }
}

pub static mut editor_animation_initialized: bool = false;
pub fn init_editor_animation() {
    unsafe {
        if editor_animation_initialized {
            return;
        }

        set_tile_drag = false;
        view_tileset_repeat_direction = -1;
        view_tileset_repeat_timer = 0;

        editor_animation_initialized = true;
    }
}

pub fn editor_animation() -> i32 {
    init_editor_animation();

    unsafe {
        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            let iCol = (event.button.x / TILESIZE + view_animated_tileset_x) as i16;
            let iRow = (event.button.y / TILESIZE) as i16;

            let fInValidTile = iRow >= 0 && iRow <= 7;

            match event.type_ {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    let sym = event.key.keysym.sym;
                    if sym == k(SDLK_ESCAPE) || sym == k(SDLK_a) {
                        if !set_tile_drag {
                            edit_mode = 8;
                            editor_animation_initialized = false;
                            return EDITOR_EDIT;
                        }
                    } else if sym == k(SDLK_LEFT) {
                        if view_animated_tileset_x > 0 {
                            view_animated_tileset_x -= 1;
                            view_tileset_repeat_direction = 2;
                            view_tileset_repeat_timer = 30;
                        }
                    } else if sym == k(SDLK_RIGHT) {
                        if view_animated_tileset_x < 12 {
                            view_animated_tileset_x += 1;
                            view_tileset_repeat_direction = 3;
                            view_tileset_repeat_timer = 30;
                        }
                    }
                }

                SDL_KEYUP_EV => {
                    let sym = event.key.keysym.sym;
                    if sym == k(SDLK_LEFT) || sym == k(SDLK_RIGHT) {
                        view_tileset_repeat_direction = -1;
                        view_tileset_repeat_timer = 0;
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    let idx = (iCol as i32 + ((iRow as i32) << 5)) as usize;
                    if event.button.button == BUTTON_LEFT {
                        if fInValidTile {
                            set_tile_start_x = iCol as i32;
                            set_tile_start_y = iRow as i32;
                            set_tile_end_x = set_tile_start_x;
                            set_tile_end_y = set_tile_start_y;

                            set_tile_drag = true;
                        }
                    } else if event.button.button == BUTTON_RIGHT {
                        animatedtiletypes[idx] = next_tile_type(animatedtiletypes[idx]);
                        set_type = animatedtiletypes[idx];
                    } else if event.button.button == BUTTON_MIDDLE {
                        animatedtiletypes[idx] = prev_tile_type(animatedtiletypes[idx]);
                        set_type = animatedtiletypes[idx];
                    }
                }

                SDL_MOUSEBUTTONUP_EV => {
                    if event.button.button == BUTTON_LEFT {
                        if fInValidTile {
                            set_tile_cols = set_tile_end_x - set_tile_start_x + 1;
                            set_tile_rows = set_tile_end_y - set_tile_start_y + 1;

                            set_tile_drag = false;
                            edit_mode = 8; //change to edit mode using tiles
                            editor_animation_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_MOUSEMOTION_EV => {
                    bound_mouse_motion_coords();
                    if fInValidTile {
                        if event.motion.state == sdl_button(SDL_BUTTON_LEFT) {
                            if (iCol as i32) < set_tile_start_x {
                                set_tile_start_x = iCol as i32;
                            }

                            if iCol as i32 > set_tile_end_x {
                                set_tile_end_x = iCol as i32;
                            }

                            if (iRow as i32) < set_tile_start_y {
                                set_tile_start_y = iRow as i32;
                            }

                            if iRow as i32 > set_tile_end_y {
                                set_tile_end_y = iRow as i32;
                            }
                        } else if event.motion.state == sdl_button(SDL_BUTTON_RIGHT) || event.motion.state == sdl_button(SDL_BUTTON_MIDDLE) {
                            animatedtiletypes[(iCol as i32 + ((iRow as i32) << 5)) as usize] = set_type;
                        }
                    }
                }

                _ => {}
            }
        }

        //Allow auto-scrolling of tilesets when the arrow keys are held down
        if view_tileset_repeat_direction >= 0 && view_tileset_repeat_timer > 0 {
            view_tileset_repeat_timer -= 1;
            if view_tileset_repeat_timer <= 0 {
                view_tileset_repeat_timer = 5;

                if view_tileset_repeat_direction == 2 && view_animated_tileset_x > 0 {
                    view_animated_tileset_x -= 1;
                } else if view_tileset_repeat_direction == 3 && view_animated_tileset_x < 12 {
                    view_animated_tileset_x += 1;
                }
            }
        }

        SDL_FillRect(screen, null(), 0xFF888888);

        for iCol in view_animated_tileset_x as i16..(view_animated_tileset_x + 20) as i16 {
            for iRow in 0..8i16 {
                let iDestX: i16 = ((iCol as i32 - view_animated_tileset_x) << 5) as i16;
                let iDestY: i16 = iRow << 5;
                let iSrcX: i16 = iRow << 7;
                let iSrcY: i16 = iCol << 5;

                rm.spr_tileanimation[0].draw_src(iDestX as i32, iDestY as i32, &rect(iSrcX as i32, iSrcY as i32, TILESIZE, TILESIZE));

                let t = animatedtiletypes[(iCol as i32 + ((iRow as i32) << 5)) as usize];
                if t != TileType::NonSolid {
                    rm.spr_tiletypes.draw_src(iDestX as i32, iDestY as i32, &rect((prev_tile_type(t).0 as i32) << 3, 0, 8, 8));
                }
            }
        }

        if set_tile_drag {
            for i in (set_tile_start_x - view_animated_tileset_x) as i16..=(set_tile_end_x - view_animated_tileset_x) as i16 {
                for j in set_tile_start_y as i16..=set_tile_end_y as i16 {
                    rm.spr_selectedtile.draw((i as i32) << 5, (j as i32) << 5);
                }
            }
        }

        rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());

        rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "Use Arrow Keys To Scroll");

        draw_message();
        EDITOR_ANIMATION
    }
}

pub fn load_background_page(iPage_: i16) {
    unsafe {
        let srcRectBackground = rect(0, 0, 640, 480);
        let mut dstRectBackground = rect(0, 0, 160, 120);

        for iIndex in 0..16i32 {
            if iPage_ as i32 * 16 + iIndex >= backgroundlist.count() as i32 {
                break;
            }

            let szFileName = backgroundlist.at((iPage_ as i32 * 16 + iIndex) as usize).to_string_lossy().into_owned();

            if szFileName.is_empty() {
                return;
            }

            let temp = load_surface(&szFileName);

            if temp.is_null() {
                println!("ERROR: Couldn't load thumbnail background: {}", sdl_error());
                return;
            }

            let sBackground = SDL_ConvertSurfaceFormat(temp, SDL_PixelFormatEnum::SDL_PIXELFORMAT_ARGB8888 as u32, 0);
            SDL_FreeSurface(temp);

            if sBackground.is_null() {
                println!("ERROR: Couldn't convert thumbnail background to display pixel format: {}", sdl_error());
                return;
            }

            SDL_FillRect(sBackgrounds[iIndex as usize], null(), 0x0);

            if (*sBackground).w != 640 || (*sBackground).h != 480 {
                println!("WARNING: Background {} is {}x{} but must be 640x480. Skipping.", szFileName, (*sBackground).w, (*sBackground).h);

                SDL_FreeSurface(sBackground);
                continue;
            }

            if SDL_UpperBlitScaled(sBackground, &srcRectBackground, sBackgrounds[iIndex as usize], &mut dstRectBackground) < 0 {
                eprintln!("SDL_SCALEBLIT error: {}", sdl_error());
                SDL_FreeSurface(sBackground);
                return;
            }

            SDL_FreeSurface(sBackground);
        }
    }
}

pub fn display_help() -> i32 {
    unsafe {
        drawmap(false, TILESIZE as i16, false);
        rm.menu_shade.draw(0, 0);

        let fh = rm.menu_font_small.get_height();
        let mut offsety = 10;
        let mut offsetx = 20;
        let mut line = |text: &str, gap: i32, offsety: &mut i32, offsetx: i32| {
            rm.menu_font_small.draw(offsetx, *offsety, text);
            *offsety += fh + gap;
        };

        line("Modes:", 2, &mut offsety, offsetx);
        line("[t] - Tile Mode", 2, &mut offsety, offsetx);
        line("[i] - Block Mode", 2, &mut offsety, offsetx);
        line("[o] - Map Item Mode", 2, &mut offsety, offsetx);
        line("[w] - Warp Mode", 2, &mut offsety, offsetx);
        line("[m] - Move Mode", 2, &mut offsety, offsetx);
        line("[l] - Tile Type Mode", 2, &mut offsety, offsetx);
        line("[p] - Platform Mode", 2, &mut offsety, offsetx);
        line("[x] - No Player Spawn Area", 2, &mut offsety, offsetx);
        line("[z] - No Item Spawn Area", 2, &mut offsety, offsetx);
        line("[h] - Hazard Mode", 2, &mut offsety, offsetx);
        line("[a] - Animated Tile Mode", 2, &mut offsety, offsetx);
        line("[j] - Race Goals Mode", 2, &mut offsety, offsetx);
        line("[k] - Block Properties", 12, &mut offsety, offsetx);

        line("Layers:", 2, &mut offsety, offsetx);
        line("[v] - Hide Blocks", 2, &mut offsety, offsetx);
        line("[y] - Select Active Tile Layer", 2, &mut offsety, offsetx);
        line("[u] - Hide Inactive Tile Layers", 2, &mut offsety, offsetx);
        line("[end] - Optimize Layers", 12, &mut offsety, offsetx);

        line("Miscellaneous:", 2, &mut offsety, offsetx);
        line("[b] - Background Thumbnails", 2, &mut offsety, offsetx);
        line("[g] - Change Backgrounds", 2, &mut offsety, offsetx);
        line("[r] - Change Music Category", 2, &mut offsety, offsetx);
        line("[e] - Change Floating Eyecandy", 2, &mut offsety, offsetx);
        line("[ctrl] + [delete] - Clear All", 2, &mut offsety, offsetx);
        line("[insert] - Take Screenshot", 2, &mut offsety, offsetx);

        offsetx = 305;
        offsety = 10;

        line("File:", 2, &mut offsety, offsetx);
        line("[n] - New Map", 2, &mut offsety, offsetx);
        line("[s] - Save Map", 2, &mut offsety, offsetx);
        line("[shift] + [s] - Save As", 2, &mut offsety, offsetx);
        line("[f] - Find Map", 2, &mut offsety, offsetx);
        line("[shift] + [f] - New Search", 2, &mut offsety, offsetx);
        line("[pageup] - Go To Previous Map", 2, &mut offsety, offsetx);
        line("[pagedown] - Go To Next Map", 20, &mut offsety, offsetx);

        line("Tile, Warp and Block Modes:", 2, &mut offsety, offsetx);
        line("[Left Mouse Button] - Place Item", 2, &mut offsety, offsetx);
        line("[Right Mouse Button] - Remove Item", 20, &mut offsety, offsetx);

        line("Move Mode:", 2, &mut offsety, offsetx);
        line("[Left Mouse Button] - Select Area", 2, &mut offsety, offsetx);
        line("[Right Mouse Button] - Unselect Area", 2, &mut offsety, offsetx);
        line("Select And Drag - Move Selections", 2, &mut offsety, offsetx);
        line("Hold [shift] - Multiple Selections", 2, &mut offsety, offsetx);
        line("Hold [ctrl] - Freehand Selections", 2, &mut offsety, offsetx);
        line("[delete] - Delete Selection", 2, &mut offsety, offsetx);
        line("[c] - Copy Selection", 20, &mut offsety, offsetx);

        line("Platforms:", 2, &mut offsety, offsetx);
        line("[p] - Path", 2, &mut offsety, offsetx);
        line("[+/-] - Change Speed", 2, &mut offsety, offsetx);
        line("[delete] - Delete", 2, &mut offsety, offsetx);
        rm.menu_font_small.draw(offsetx, offsety, "[alt] + [enter] - Full Screen/Window");

        //handle messages
        while SDL_PollEvent(&mut event) != 0 {
            match event.type_ {
                SDL_QUIT_EV => return 0,
                SDL_KEYDOWN_EV => return 0,
                _ => {}
            }
        }

        DISPLAY_HELP
    }
}

pub fn save_as() -> i32 {
    let mut fileName = String::new();
    let mut mapLocation = String::from("maps/");

    if dialog("Save As", "Enter name:", &mut fileName, 64) {
        mapLocation.push_str(&fileName);
        mapLocation.push_str(".map");
        save_map(&convert_path(&mapLocation));
        fileName.push_str(".map");
        unsafe {
            maplist.add(&fileName);
            maplist.find(&fileName);
        }
        loadcurrentmap();
    }

    0
}

unsafe fn draw_dialog(title: &str, instructions: &str, input: Option<&str>) {
    drawmap(false, TILESIZE as i16, false);
    rm.menu_shade.draw(0, 0);
    rm.spr_dialog.draw_src(224, 176, &rect(0, 0, 192, 128));
    rm.menu_font_large.draw_centered(320, 200, title);
    rm.menu_font_small.draw(240, 235, instructions);
    if let Some(input) = input {
        rm.menu_font_small.draw(240, 255, input);
    }
    rm.menu_font_small.draw_right_justified(640, 0, maplist.current_filename());
    gfx_flipscreen();
}

pub fn dialog(title: &str, instructions: &str, input: &mut String, inputsize: i32) -> bool {
    unsafe {
        let mut currentChar: u32 = 0;

        draw_dialog(title, instructions, None);

        loop {
            let framestart = sdl2::sys::SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event.type_ {
                    SDL_QUIT_EV => return false,

                    SDL_KEYDOWN_EV => {
                        let sym = event.key.keysym.sym;
                        if sym == k(SDLK_KP_ENTER) || sym == k(SDLK_RETURN) {
                            return true;
                        } else if sym == k(SDLK_ESCAPE) {
                            return false;
                        } else if sym == k(SDLK_BACKSPACE) {
                            if currentChar > 0 {
                                input.truncate((currentChar - 1) as usize);

                                draw_dialog(title, instructions, Some(input));

                                currentChar -= 1;
                            }
                        } else if ((48..=57).contains(&sym) || sym == 45 || sym == 32 || sym == 61 || (95..=122).contains(&sym)) && currentChar < (inputsize as u32) - 1 {
                            //insert character into fileName and onScreenText and increment current char
                            let mut key: u8 = sym as u8;

                            let keystate = get_keyboard_state();
                            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                                if sym == 45 {
                                    key = 95;
                                } else if (95..=122).contains(&sym) {
                                    key = key.wrapping_sub(32); //Capitalize
                                } else if sym == 48 {
                                    key = 41;
                                } else if sym == 49 {
                                    key = 33;
                                } else if sym == 50 {
                                    key = 64;
                                } else if sym == 51 {
                                    key = 35;
                                } else if sym == 52 {
                                    key = 36;
                                } else if sym == 53 {
                                    key = 37;
                                } else if sym == 54 {
                                    key = 94;
                                } else if sym == 55 {
                                    key = 38;
                                } else if sym == 57 {
                                    key = 40;
                                } else if sym == 61 {
                                    key = 43;
                                }
                            }

                            input.truncate(currentChar as usize);
                            input.push(key as char);
                            currentChar += 1;

                            draw_dialog(title, instructions, Some(input));
                        }
                    }

                    _ => {}
                }
            }

            let mut delay = WAITTIME - (sdl2::sys::SDL_GetTicks() as i32 - framestart);
            if delay < 0 {
                delay = 0;
            } else if delay > WAITTIME {
                delay = WAITTIME;
            }

            editor_harness::frame_delay(delay as u32);
        }
    }
}

pub fn find() -> i32 {
    let mut fileName = String::new();

    if dialog("Find Map", "Enter name:", &mut fileName, 64) {
        unsafe {
            findstring = fileName.clone();

            let fs = findstring.clone();
            if maplist.find(&fs) {
                loadcurrentmap();
            }
        }
    }

    0
}

pub fn clear_map() -> i32 {
    unsafe {
        g_map.clear_map();
        g_iNumPlatforms = 0;
    }

    println!("Map Cleared");
    0
}

pub fn loadcurrentmap() {
    unsafe {
        let filename = maplist.current_filename().to_string();
        g_map.load_map(&filename, read_type_full);

        if g_map.iNumRaceGoals == 0 {
            for iGoal in 0..MAXRACEGOALS as usize {
                g_map.racegoallocations[iGoal].x = (iGoal as i32 * 80 + 20) as i16;
                g_map.racegoallocations[iGoal].y = 18;
            }
        }

        if g_map.iNumFlagBases == 0 {
            for iBase in 0..4usize {
                g_map.flagbaselocations[iBase].x = (iBase as i32 * 80 + 20) as i16;
                g_map.flagbaselocations[iBase].y = 18;
            }
        }

        let filename = concat("gfx/packs/Classic/backgrounds/", &g_map.szBackgroundFile);
        let mut path = convert_path(&filename);
        backgroundlist.set_current_path(Path::new(&filename));

        if !file_exists(&path) {
            path = convert_path("gfx/packs/Classic/backgrounds/Land_Classic.png");
            backgroundlist.set_current_path(Path::new("gfx/packs/Classic/backgrounds/Land_Classic.png"));
        }

        rm.spr_background = SpriteBuilder::new(&path).without_color_key().create();

        g_iNumPlatforms = g_map.platforms.len() as i16;

        for iPlatform in 0..g_iNumPlatforms as usize {
            let mp = g_map.platforms[iPlatform];
            for iCol in 0..MW {
                for iRow in 0..MH {
                    if (iCol as i16) < mp.iTileWidth && (iRow as i16) < mp.iTileHeight {
                        g_Platforms[iPlatform].tiles[iCol * MH + iRow] = *mp.tile_at(iCol, iRow);
                        g_Platforms[iPlatform].types[iCol * MH + iRow] = mp.tile_type_at(iCol, iRow);
                    } else {
                        clear_tileset_tile(&mut g_Platforms[iPlatform].tiles[iCol * MH + iRow]);
                        g_Platforms[iPlatform].types[iCol * MH + iRow] = TileType::NonSolid;
                    }
                }
            }

            g_Platforms[iPlatform].iDrawLayer = mp.iDrawLayer;
            g_Platforms[iPlatform].iPathType = mp.pPath.path_type_id();

            let mut mpm = mp;
            let any = mpm.pPath.as_any();
            if let Some(path) = any.downcast_ref::<StraightPath>() {
                g_Platforms[iPlatform].iVelocity = (path.path().speed() * 4.0f32) as i32 as i16;
                g_Platforms[iPlatform].iStartX = path.start_pos().x as i32 as i16;
                g_Platforms[iPlatform].iStartY = path.start_pos().y as i32 as i16;
                g_Platforms[iPlatform].iEndX = path.end_pos().x as i32 as i16;
                g_Platforms[iPlatform].iEndY = path.end_pos().y as i32 as i16;
            } else if let Some(path) = any.downcast_ref::<StraightPathContinuous>() {
                g_Platforms[iPlatform].iVelocity = (path.path().speed() * 4.0f32) as i32 as i16;
                g_Platforms[iPlatform].iStartX = path.start_pos().x as i32 as i16;
                g_Platforms[iPlatform].iStartY = path.start_pos().y as i32 as i16;
                g_Platforms[iPlatform].fAngle = path.angle();
            } else if let Some(path) = any.downcast_ref::<EllipsePath>() {
                g_Platforms[iPlatform].iVelocity = (path.path().speed() / 0.0030f32) as i16;
                g_Platforms[iPlatform].fRadiusX = path.radius().x;
                g_Platforms[iPlatform].fRadiusY = path.radius().y;
                g_Platforms[iPlatform].iStartX = path.center_pos().x as i32 as i16;
                g_Platforms[iPlatform].iStartY = path.center_pos().y as i32 as i16;
                g_Platforms[iPlatform].fAngle = path.start_angle();
            }

            g_Platforms[iPlatform].update_preview();
        }
    }
}

pub fn set_platform_to_defaults(iPlatform: i16) {
    unsafe {
        let p = &mut g_Platforms[iPlatform as usize];
        for iCol in 0..MW {
            for iRow in 0..MH {
                clear_tileset_tile(&mut p.tiles[iCol * MH + iRow]);
                p.types[iCol * MH + iRow] = TileType::NonSolid;
            }
        }

        p.iVelocity = 4;
        p.iStartX = 320;
        p.iStartY = 240;
        p.iEndX = 352;
        p.iEndY = 240;

        p.fAngle = 0.0f32;
        p.fRadiusX = 128.0f32;
        p.fRadiusY = 128.0f32;

        p.iDrawLayer = 2;
    }
}

pub fn savecurrentmap() -> i32 {
    unsafe {
        g_messagedisplaytimer = 60;
        g_szMessageTitle = "Saved".to_string();
        g_szMessageLine[0] = "Your map has".to_string();
        g_szMessageLine[1] = "been saved.".to_string();
        g_szMessageLine[2] = String::new();

        let file = maplist.current_filename().to_string();
        save_map(&file);
    }
    0
}

pub fn insert_platforms_into_map() {
    unsafe {
        //First take the created platforms and move them into the actual map
        g_map.clear_platforms();

        g_map.platforms.reserve(g_iNumPlatforms as usize);

        for iPlatform in 0..g_iNumPlatforms {
            let (mut iTop, mut iLeft, mut iWidth, mut iHeight) = (0i16, 0i16, 0i16, 0i16);
            calculate_platform_dims(iPlatform, &mut iLeft, &mut iTop, &mut iWidth, &mut iHeight);

            let n = (iWidth as i32 * iHeight as i32) as usize;
            let mut tiles: Vec<TilesetTile> = vec![TilesetTile::default(); n];
            let mut types: Vec<TileType> = vec![TileType::NonSolid; n];

            let p = &g_Platforms[iPlatform as usize];
            for iCol in 0..iWidth as usize {
                for iRow in 0..iHeight as usize {
                    let cellIdx = (iCol + iLeft as usize) * MH + iRow + iTop as usize;
                    tiles[iCol * iHeight as usize + iRow] = p.tiles[cellIdx];
                    types[iCol * iHeight as usize + iRow] = p.types[cellIdx];
                }
            }

            let iDrawLayer = p.iDrawLayer;

            let fStartX = p.iStartX as f32;
            let fStartY = p.iStartY as f32;

            let path: Box<dyn MovingPlatformPathTrait> = if p.iPathType == PlatformPathType::Straight {
                let fVelocity = p.iVelocity as f32 * 0.26f32;
                let fEndX = p.iEndX as f32;
                let fEndY = p.iEndY as f32;
                Box::new(StraightPath::new(fVelocity, Vec2f::new(fStartX, fStartY), Vec2f::new(fEndX, fEndY), false))
            } else if p.iPathType == PlatformPathType::StraightContinuous {
                let fVelocity = p.iVelocity as f32 * 0.26f32;
                Box::new(StraightPathContinuous::new(fVelocity, Vec2f::new(fStartX, fStartY), p.fAngle, false))
            } else if p.iPathType == PlatformPathType::Ellipse {
                let fVelocity = p.iVelocity as f32 * 0.0030f32;
                let radiusX = p.fRadiusX;
                let radiusY = p.fRadiusY;
                Box::new(EllipsePath::new(fVelocity, p.fAngle, Vec2f::new(radiusX, radiusY), Vec2f::new(fStartX, fStartY), false))
            } else {
                panic!("MovingPlatform constructed with a NULL path");
            };

            g_map.add_permanent_platform(MovingPlatform::new(tiles, types, iWidth, iHeight, iDrawLayer, path, false));
        }
    }
}

pub fn save_map(file: &str) {
    insert_platforms_into_map();

    //Then save the rest of the map
    unsafe { g_map.save_map(file) };
}

pub fn calculate_platform_dims(iPlatform: i16, ix: &mut i16, iy: &mut i16, iw: &mut i16, ih: &mut i16) {
    unsafe {
        let mut iTop: i16 = MAPHEIGHT as i16;
        let mut iRight: i16 = -1;
        let mut iBottom: i16 = -1;
        let mut iLeft: i16 = MAPWIDTH as i16;
        //Calculate the height and width of the platform
        for iCol in 0..MAPWIDTH as i16 {
            for iRow in 0..MAPHEIGHT as i16 {
                if g_Platforms[iPlatform as usize].tiles[iCol as usize * MH + iRow as usize].iID as i32 != TILESETNONE {
                    if iTop > iRow {
                        iTop = iRow;
                    }

                    if iLeft > iCol {
                        iLeft = iCol;
                    }

                    if iBottom < iRow {
                        iBottom = iRow;
                    }

                    if iRight < iCol {
                        iRight = iCol;
                    }
                }
            }
        }

        let mut iWidth: i16 = iRight - iLeft + 1;
        let mut iHeight: i16 = iBottom - iTop + 1;

        if iRight == -1 || iBottom == -1 {
            iWidth = 1;
            iHeight = 1;
            iLeft = 0;
            iTop = 0;
        }

        *ix = iLeft;
        *iy = iTop;

        *iw = iWidth;
        *ih = iHeight;
    }
}

pub fn findcurrentstring() -> i32 {
    unsafe {
        if !findstring.is_empty() {
            let fs = findstring.clone();
            if maplist.find(&fs) {
                loadcurrentmap();
            }
        }
    }

    0
}

pub fn newmap() -> i32 {
    let mut fileName = String::new();
    let mut mapLocation = String::from("maps/");

    if dialog("New Map", "Enter name:", &mut fileName, 64) {
        unsafe {
            g_map.clear_map();
            g_map.clear_platforms();
            mapLocation.push_str(&fileName);
            mapLocation.push_str(".map");
            g_map.save_map(&convert_path(&mapLocation));
            fileName.push_str(".map");
            maplist.add(&fileName);
            maplist.find(&fileName);
        }
        loadcurrentmap();
    }

    0
}

pub fn resetselectedtiles() {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                selectedtiles[j][kk] = false;
            }
        }
    }
}

pub fn copymoveselection() {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                moveselectedtiles[j][kk] = selectedtiles[j][kk];
            }
        }

        copiedlayer = selected_layer;
    }
}

pub fn pastemoveselection(movex: i32, movey: i32) {
    unsafe {
        for kk in 0..MAPHEIGHT {
            for j in 0..MAPWIDTH {
                if moveselectedtiles[j as usize][kk as usize] {
                    if j + movex >= 0 && j + movex < MAPWIDTH && kk + movey >= 0 && kk + movey < MAPHEIGHT {
                        selectedtiles[(j + movex) as usize][(kk + movey) as usize] = moveselectedtiles[j as usize][kk as usize];
                    }
                }
            }
        }
    }
}

pub fn copyselectedtiles() -> bool {
    unsafe {
        //Copy the selected tiles and remove tiles from map
        let mut ret = false;
        for kk in 0..MH {
            for j in 0..MW {
                if selectedtiles[j][kk] {
                    ret = true;
                    let c = &mut copiedtiles[j][kk];
                    for iLayer in 0..MAPLAYERS as usize {
                        c.tile[iLayer] = g_map.mapdata[j][kk][iLayer];
                    }

                    c.block.iType = g_map.objectdata[j][kk].iType;
                    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                        c.block.iSettings[iSetting] = g_map.objectdata[j][kk].iSettings[iSetting];
                    }
                    c.block.fHidden = g_map.objectdata[j][kk].fHidden;

                    c.warp.connection = g_map.warpdata[j][kk].connection;
                    c.warp.direction = g_map.warpdata[j][kk].direction;
                    c.warp.id = g_map.warpdata[j][kk].id;

                    c.tiletype = g_map.mapdatatop[j][kk];

                    for iType in 0..NUMSPAWNAREATYPES as usize {
                        c.nospawn[iType] = g_map.nospawn[iType][j][kk];
                    }

                    c.item = -1;
                    for item in g_map.mapitems.iter() {
                        if item.ix as usize == j && item.iy as usize == kk {
                            c.item = item.itype as i32;
                            break;
                        }
                    }
                }
            }
        }

        copiedlayer = selected_layer;

        ret
    }
}

pub fn clearselectedmaptiles() {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                if selectedtiles[j][kk] {
                    if view_only_layer {
                        g_map.mapdata[j][kk][selected_layer as usize].iID = TILESETNONE as i16;
                    } else {
                        for iLayer in 0..MAPLAYERS as usize {
                            g_map.mapdata[j][kk][iLayer].iID = TILESETNONE as i16;
                        }

                        g_map.objectdata[j][kk].iType = -1;

                        g_map.warpdata[j][kk].connection = -1;
                        g_map.warpdata[j][kk].direction = WARP_UNDEFINED;
                        g_map.warpdata[j][kk].id = -1;

                        for iType in 0..NUMSPAWNAREATYPES as usize {
                            g_map.nospawn[iType][j][kk] = false;
                        }

                        remove_map_item_at(j as i16, kk as i16);
                    }

                    update_tile_type(j as i16, kk as i16);
                }
            }
        }
    }
}

/// `replacetile(short*, short, bool)`
pub fn replacetile_short(iDstTile: &mut i16, iSrcTile: i16, fAllowReplace: bool) {
    unsafe {
        if move_replace || fAllowReplace {
            *iDstTile = iSrcTile;
        }
    }
}

pub fn copytilesettile(dst: &mut TilesetTile, src: &TilesetTile) {
    dst.iID = src.iID;
    dst.iCol = src.iCol;
    dst.iRow = src.iRow;
}

/// `replacetile(TilesetTile*, TilesetTile*)`
pub fn replacetile_tile(dstTile: &mut TilesetTile, srcTile: &TilesetTile) {
    unsafe {
        if move_replace {
            copytilesettile(dstTile, srcTile);
        } else if srcTile.iID as i32 != TILESETNONE {
            copytilesettile(dstTile, srcTile);
        }
    }
}

pub fn copymapblock(dst: &mut MapBlock, src: &MapBlock) {
    dst.iType = src.iType;
    dst.fHidden = src.fHidden;

    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
        dst.iSettings[iSetting] = src.iSettings[iSetting];
    }
}

/// `replacetile(MapBlock*, MapBlock*)`
pub fn replacetile_block(dstTile: &mut MapBlock, srcTile: &MapBlock) {
    unsafe {
        if move_replace {
            copymapblock(dstTile, srcTile);
        } else if srcTile.iType != -1 {
            copymapblock(dstTile, srcTile);
        }
    }
}

pub fn pasteselectedtiles(movex: i32, movey: i32) {
    unsafe {
        //Paste the tiles into their new location
        //Removing tiles that hang over the edges of the map
        for kk in 0..MAPHEIGHT {
            for j in 0..MAPWIDTH {
                if selectedtiles[j as usize][kk as usize] {
                    if j + movex >= 0 && j + movex < MAPWIDTH && kk + movey >= 0 && kk + movey < MAPHEIGHT {
                        let iNewX = (j + movex) as usize;
                        let iNewY = (kk + movey) as usize;
                        let c = copiedtiles[j as usize][kk as usize];

                        if view_only_layer {
                            replacetile_tile(&mut g_map.mapdata[iNewX][iNewY][selected_layer as usize], &c.tile[copiedlayer as usize]);
                        } else {
                            for iLayer in 0..MAPLAYERS as usize {
                                replacetile_tile(&mut g_map.mapdata[iNewX][iNewY][iLayer], &c.tile[iLayer]);
                            }

                            replacetile_block(&mut g_map.objectdata[iNewX][iNewY], &c.block);

                            replacetile_short(&mut g_map.warpdata[iNewX][iNewY].connection, c.warp.connection, c.warp.connection != -1);
                            // The C++ writes the direction through a `short*`; WarpEnterDirection is an int-sized enum.
                            let mut dir = g_map.warpdata[iNewX][iNewY].direction as i16;
                            replacetile_short(&mut dir, c.warp.direction as i16, c.warp.connection != -1);
                            g_map.warpdata[iNewX][iNewY].direction = (g_map.warpdata[iNewX][iNewY].direction & !0xFFFF) | (dir as u16 as i32);
                            replacetile_short(&mut g_map.warpdata[iNewX][iNewY].id, c.warp.id, c.warp.connection != -1);

                            if move_replace {
                                g_map.mapdatatop[iNewX][iNewY] = c.tiletype;
                            } else {
                                update_tile_type(j as i16, kk as i16);
                            }

                            for iType in 0..NUMSPAWNAREATYPES as usize {
                                g_map.nospawn[iType][iNewX][iNewY] = c.nospawn[iType];
                            }

                            if (g_map.mapitems.len() as i32) < MAXMAPITEMS && c.item >= 0 {
                                let item = MapItem { itype: c.item as MapItemType, ix: iNewX as i16, iy: iNewY as i16 };
                                g_map.mapitems.push(item);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn getcenterselection(x: &mut i32, y: &mut i32) {
    unsafe {
        let mut left = -1;
        let mut top = -1;
        let mut right = -1;
        let mut bottom = -1;

        for kk in 0..MAPHEIGHT {
            for j in 0..MAPWIDTH {
                if selectedtiles[j as usize][kk as usize] {
                    if left == -1 {
                        left = j;
                    }

                    if top == -1 {
                        top = kk;
                    }

                    right = j;
                    bottom = kk;
                }
            }
        }

        if right == -1 {
            *x = 0;
            *y = 0;
            return;
        }

        *x = ((right - left) >> 1) + left;
        *y = ((bottom - top) >> 1) + top;
    }
}

//take screenshots in full and thumbnail sizes
pub fn takescreenshot() {
    unsafe {
        let iTileSizes: [i16; 3] = [TILESIZE as i16, PREVIEWTILESIZE as i16, THUMBTILESIZE as i16];
        let old_screen = screen;

        for iScreenshotSize in 0..3i16 {
            let iTileSize = iTileSizes[iScreenshotSize as usize];

            //Allow wrapping of path dots
            rm.spr_platformpath.set_wrap((640 >> iScreenshotSize) as i16);

            //Create new screenshot surface
            let screenshot = SDL_CreateRGBSurface((*old_screen).flags, iTileSize as i32 * 20, iTileSize as i32 * 15, (*(*old_screen).format).BitsPerPixel as i32, 0, 0, 0, 0);
            blitdest = screenshot;
            screen = screenshot;

            //Draw map to screenshot
            drawmap(true, iTileSize, false);

            //Draw platforms to screenshot
            for iPlatform in 0..g_iNumPlatforms as usize {
                let platform = &g_Platforms[iPlatform];
                draw_platform(
                    platform.iPathType,
                    &platform.tiles,
                    platform.iStartX,
                    platform.iStartY,
                    platform.iEndX,
                    platform.iEndY,
                    platform.fAngle,
                    platform.fRadiusX,
                    platform.fRadiusY,
                    iScreenshotSize,
                    g_map.platforms[iPlatform].iTileWidth,
                    g_map.platforms[iPlatform].iTileHeight,
                    true,
                    true,
                );
            }

            //Draw map hazards
            for hazard in g_map.maphazards.clone().iter() {
                draw_map_hazard(hazard, iScreenshotSize, false);
            }

            //Save the screenshot with the same name as the map file
            let mut szSaveFile = String::from("maps/screenshots/");
            szSaveFile.push_str(&get_name_from_file_name(maplist.current_filename(), false));

            if iTileSize as i32 == PREVIEWTILESIZE {
                szSaveFile.push_str("_preview");
            } else if iTileSize as i32 == THUMBTILESIZE {
                szSaveFile.push_str("_thumb");
            }

            szSaveFile.push_str(".png");
            let c = CString::new(convert_path(&szSaveFile)).unwrap();
            IMG_SavePNG(screenshot, c.as_ptr());

            SDL_FreeSurface(screenshot);

            println!("Screenshot taken: {}", szSaveFile);

            screen = old_screen;
            blitdest = screen;
        }
    }
}

pub fn read_animated_tile_type_file(szFile: &str) -> bool {
    unsafe {
        //Detect if the tiletype file already exists, if not create it
        if file_exists(szFile) {
            let mut tsf = BinaryFile::new(szFile, "rb");
            if !tsf.is_open() {
                println!("ERROR: couldn't open tileset file: {}", szFile);
                return false;
            }

            animatedtiletypes = vec![TileType::NonSolid; 256];

            for i in 0..256usize {
                animatedtiletypes[i] = TileType::from_i32(tsf.read_i32());
            }
        } else {
            animatedtiletypes = vec![TileType::NonSolid; 256];
        }

        true
    }
}

pub fn write_animated_tile_type_file(szFile: &str) -> bool {
    unsafe {
        let mut tsf = BinaryFile::new(szFile, "wb");
        if !tsf.is_open() {
            println!("ERROR: couldn't open tileset file to save tile types: {}", szFile);
            return false;
        }

        for i in 0..256usize {
            tsf.write_i32(animatedtiletypes[i].0 as i32);
        }

        true
    }
}

/// FNV-1a over 32-bit little-endian values, for the replay dump.
struct Fnv(u32);

impl Fnv {
    fn new() -> Self {
        Fnv(0x811c9dc5)
    }
    fn add(&mut self, v: i32) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u32;
            self.0 = self.0.wrapping_mul(0x01000193);
        }
    }
    fn addf(&mut self, v: f32) {
        self.add(v.to_bits() as i32);
    }
}

/// Replay-harness `E`/`T`/`X`/`P`/`H` records (EDITOR_REPLAY.md).
pub fn dump_editor_state(out: &mut dyn Write) {
    unsafe {
        let _ = writeln!(
            out,
            "E state={} edit_mode={} layer={} mouse={},{} ignore={} only={} blocks={}",
            state, edit_mode, selected_layer, mouse_x, mouse_y, ignoreclick as i32, view_only_layer as i32, viewblocks as i32
        );
        let _ = writeln!(
            out,
            "T tileset={} start={},{} end={},{} size={},{} drag={} view={},{},{} block={},{} tiletype={} settype={} item={} warp={},{} nospawn={}",
            set_tile_tileset,
            set_tile_start_x,
            set_tile_start_y,
            set_tile_end_x,
            set_tile_end_y,
            set_tile_cols,
            set_tile_rows,
            set_tile_drag as i32,
            view_tileset_x,
            view_tileset_y,
            view_animated_tileset_x,
            set_block,
            set_block_switch_on,
            set_tiletype.0 as i32,
            set_type.0 as i32,
            set_mapitem,
            set_direction,
            set_connection,
            nospawn_mode
        );
        let _ = writeln!(
            out,
            "X move={} start={},{} offset={},{} drag={},{},{},{} replace={} nodrag={} copied={}",
            move_mode,
            move_start_x,
            move_start_y,
            move_offset_x,
            move_offset_y,
            move_drag_start_x,
            move_drag_start_y,
            move_drag_offset_x,
            move_drag_offset_y,
            move_replace as i32,
            move_nodrag as i32,
            copiedlayer
        );
        let _ = writeln!(
            out,
            "P count={} edit={} state={} preview={} switch={},{} hazards={} hzstate={} hzedit={} modeitem={},{} msg={} music={}",
            g_iNumPlatforms,
            iEditPlatform,
            iPlatformEditState,
            iPlatformPreview,
            iPlatformSwitchState,
            iPlatformSwitchIndex,
            g_map.maphazards.len(),
            iEditState,
            iEditMapHazard,
            modeitemmode,
            dragmodeitem,
            g_messagedisplaytimer,
            g_musiccategorydisplaytimer
        );

        let mut hm = Fnv::new();
        for x in 0..MW {
            for y in 0..MH {
                for l in 0..MAPLAYERS as usize {
                    let t = g_map.mapdata[x][y][l];
                    hm.add(t.iID as i32);
                    hm.add(t.iCol as i32);
                    hm.add(t.iRow as i32);
                }
                hm.add(g_map.mapdatatop[x][y].0 as i32);
                let b = &g_map.objectdata[x][y];
                hm.add(b.iType as i32);
                // Only the settings the map format stores: the rest keep stale heap bytes in the C++.
                let stored = match b.iType {
                    1 | 15 => b.iSettings.len(),
                    11..=14 => 1,
                    _ => 0,
                };
                for s in &b.iSettings[..stored] {
                    hm.add(*s as i32);
                }
                hm.add(b.fHidden as i32);
                let w = g_map.warpdata[x][y];
                hm.add(w.direction as i32);
                hm.add(w.connection as i32);
                hm.add(w.id as i32);
                for t in 0..NUMSPAWNAREATYPES as usize {
                    hm.add(g_map.nospawn[t][x][y] as i32);
                }
            }
        }
        hm.add(g_map.mapitems.len() as i32);
        for item in g_map.mapitems.iter() {
            hm.add(item.itype as i32);
            hm.add(item.ix as i32);
            hm.add(item.iy as i32);
        }
        hm.add(g_map.maphazards.len() as i32);
        for hz in g_map.maphazards.iter() {
            hm.add(hz.itype as i32);
            hm.add(hz.ix as i32);
            hm.add(hz.iy as i32);
            for p in 0..NUMMAPHAZARDPARAMS as usize {
                hm.add(hz.iparam[p] as i32);
                hm.addf(hz.dparam[p]);
            }
        }
        for l in 0..3usize {
            hm.add(g_map.eyecandy[l] as i32);
        }
        hm.add(g_map.musicCategoryID as i32);
        hm.add(g_map.iNumRaceGoals as i32);
        for g in g_map.racegoallocations.iter() {
            hm.add(g.x as i32);
            hm.add(g.y as i32);
        }
        hm.add(g_map.iNumFlagBases as i32);
        for b in g_map.flagbaselocations.iter() {
            hm.add(b.x as i32);
            hm.add(b.y as i32);
        }
        for s in g_map.iSwitches.iter() {
            hm.add(*s as i32);
        }
        for c in g_map.szBackgroundFile.bytes() {
            hm.add(c as i32);
        }
        hm.add(g_map.platforms.len() as i32);

        let mut hp = Fnv::new();
        for i in 0..g_iNumPlatforms.max(0) as usize {
            let p = &g_Platforms[i];
            for t in p.tiles.iter() {
                hp.add(t.iID as i32);
                hp.add(t.iCol as i32);
                hp.add(t.iRow as i32);
            }
            for t in p.types.iter() {
                hp.add(t.0 as i32);
            }
            hp.add(p.iVelocity as i32);
            hp.add(p.iStartX as i32);
            hp.add(p.iStartY as i32);
            hp.add(p.iEndX as i32);
            hp.add(p.iEndY as i32);
            hp.add(p.iPathType as i32);
            hp.addf(p.fAngle);
            hp.addf(p.fRadiusX);
            hp.addf(p.fRadiusY);
            hp.add(p.iDrawLayer as i32);
        }

        let mut ha = Fnv::new();
        for t in animatedtiletypes.iter() {
            ha.add(t.0 as i32);
        }

        let _ = writeln!(out, "H map={:08x} plat={:08x} anim={:08x}", hm.0, hp.0, ha.0);
    }
}
