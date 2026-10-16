//! Port of src/leveleditor/leveleditor.cpp

use crate::game::Game;
use crate::common::cmd_args as cmd;
use crate::common::editor_harness;
use crate::common::file_io::BinaryFile;
use crate::common::file_list::BackgroundList;
use crate::common::game::ensure_settings_dir;
use crate::common::game_values::{default_powerup_setting, TITLESTRING};
use crate::common::gfx::color::colors;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
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
use sdl2::sys::image::IMG_SavePNG;
use crate::common::editor_event::EditorEvent;
use crate::common::util::sdl_helpers::poll_event;
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::{
    SDL_EventType, SDL_FillRect, SDL_GetError, SDL_GetScancodeFromKey, SDL_KeyCode,
    SDL_Keycode, SDL_MapRGB, SDL_Rect, SDL_SetColorKey, SDL_Surface, SDL_UpperBlit, SDL_bool, SDL_Keymod,
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

/// The editor's state, a part of `Game`.
pub struct LevelEditorState {
    pub event: EditorEvent,
    pub set_type: TileType,
    pub set_tile_rows: i32,
    pub set_tile_cols: i32,
    pub set_tile_tileset: i32,
    pub set_tile_start_x: i32,
    pub set_tile_start_y: i32,
    pub set_tile_end_x: i32,
    pub set_tile_end_y: i32,
    pub set_tile_drag: bool,
    pub view_tileset_x: i32,
    pub view_tileset_y: i32,
    pub view_animated_tileset_x: i32,
    pub set_block: i32,
    pub set_block_switch_on: i32,
    pub set_tiletype: TileType,
    pub set_mapitem: i32,
    pub set_direction: i32,
    pub set_connection: i32,
    pub edit_mode: i32,
    pub selected_layer: i32,
    pub nospawn_mode: i32,
    pub move_mode: i32,
    pub move_start_x: i32,
    pub move_start_y: i32,
    pub move_offset_x: i32,
    pub move_offset_y: i32,
    pub move_nodrag: bool,
    pub move_replace: bool,
    pub move_drag_start_x: i32,
    pub move_drag_start_y: i32,
    pub move_drag_offset_x: i32,
    pub move_drag_offset_y: i32,
    pub state: i32,
    pub selectedtiles: [[bool; MH]; MW],
    pub moveselectedtiles: [[bool; MH]; MW],
    pub copiedtiles: Global<[[EditorMapTile; MH]; MW]>,
    pub copiedlayer: i32,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub animatedtiletypes: Vec<TileType>,
    pub s_platform: Global<gfxSprite>,
    pub s_platformpathbuttons: Global<gfxSprite>,
    pub s_maphazardbuttons: Global<gfxSprite>,
    pub viewblocks: bool,
    pub view_only_layer: bool,
    pub viewwarps: bool,
    pub ignoreclick: bool,
    pub findstring: String,
    pub g_iNumPlatforms: i16,
    pub g_Platforms: Global<Vec<MapPlatform>>,
    pub backgroundlist: Ptr<BackgroundList>,
    pub g_musiccategorydisplaytimer: i16,
    pub g_messagedisplaytimer: i16,
    pub g_szMessageTitle: String,
    pub g_szMessageLine: [String; 3],
    pub g_fFullScreen: bool,
    pub fExiting: bool,
    pub fSelectedYes: bool,
    pub editor_edit_initialized: bool,
    pub r: SDL_Rect,
    pub iBlockType: i16,
    pub editor_properties_initialized: bool,
    pub rNewButton: [SDL_Rect; 2],
    pub rTypeButton: [[SDL_Rect; 4]; 8],
    pub iPlatformEditState: i16,
    pub iPlatformSwitchState: i16,
    pub iPlatformSwitchIndex: i16,
    pub iEditPlatform: i16,
    pub iPlatformTop: i16,
    pub iPlatformLeft: i16,
    pub iPlatformWidth: i16,
    pub iPlatformHeight: i16,
    pub iPlatformPreview: i16,
    pub editor_platforms_initialized: bool,
    pub iEditState: i16,
    pub iEditMapHazard: i16,
    pub rBackground: [SDL_Rect; 2],
    pub rIconRects: [[SDL_Rect; 2]; MAXMAPHAZARDS as usize],
    pub editor_maphazards_initialized: bool,
    pub tileset: Ptr<CTileset>,
    pub view_tileset_repeat_direction: i16,
    pub view_tileset_repeat_timer: i16,
    pub editor_tiles_initialized: bool,
    pub modeitemmode: i16,
    pub dragmodeitem: i16,
    pub dragoffsetx: i16,
    pub dragoffsety: i16,
    pub editor_modeitems_initialized: bool,
    pub iPage: i16,
    pub sBackgrounds: Global<[gfxSprite; 16]>,
    pub rSrc: SDL_Rect,
    pub rDst: [SDL_Rect; 16],
    pub editor_backgrounds_initialized: bool,
    pub editor_animation_initialized: bool,
}

impl LevelEditorState {
    pub const fn new() -> Self {
        LevelEditorState {
            event: EditorEvent::new(),
            set_type: TileType::Solid,
            set_tile_rows: 0,
            set_tile_cols: 0,
            set_tile_tileset: 0,
            set_tile_start_x: 0,
            set_tile_start_y: 0,
            set_tile_end_x: 0,
            set_tile_end_y: 0,
            set_tile_drag: false,
            view_tileset_x: 0,
            view_tileset_y: 0,
            view_animated_tileset_x: 0,
            set_block: 0,
            set_block_switch_on: 0,
            set_tiletype: TileType::NonSolid,
            set_mapitem: 0,
            set_direction: 0,
            set_connection: 0,
            edit_mode: 1,
            selected_layer: 0,
            nospawn_mode: 0,
            move_mode: 0,
            move_start_x: 0,
            move_start_y: 0,
            move_offset_x: 0,
            move_offset_y: 0,
            move_nodrag: false,
            move_replace: true,
            move_drag_start_x: 0,
            move_drag_start_y: 0,
            move_drag_offset_x: 0,
            move_drag_offset_y: 0,
            state: 0,
            selectedtiles: [[false; MH]; MW],
            moveselectedtiles: [[false; MH]; MW],
            copiedtiles: Global::uninit(),
            copiedlayer: 0,
            mouse_x: 0,
            mouse_y: 0,
            animatedtiletypes: Vec::new(),
            s_platform: Global::uninit(),
            s_platformpathbuttons: Global::uninit(),
            s_maphazardbuttons: Global::uninit(),
            viewblocks: true,
            view_only_layer: false,
            viewwarps: true,
            ignoreclick: false,
            findstring: String::new(),
            g_iNumPlatforms: 0,
            g_Platforms: Global::uninit(),
            backgroundlist: Ptr::null(),
            g_musiccategorydisplaytimer: 0,
            g_messagedisplaytimer: 0,
            g_szMessageTitle: String::new(),
            g_szMessageLine: [String::new(), String::new(), String::new()],
            g_fFullScreen: false,
            fExiting: false,
            fSelectedYes: false,
            editor_edit_initialized: false,
            r: SDL_Rect { x: 0, y: 0, w: 0, h: 0 },
            iBlockType: 0,
            editor_properties_initialized: false,
            rNewButton: [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2],
            rTypeButton: [[SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 4]; 8],
            iPlatformEditState: PLATFORM_EDIT_STATE_SELECT,
            iPlatformSwitchState: 0,
            iPlatformSwitchIndex: 0,
            iEditPlatform: 0,
            iPlatformTop: 0,
            iPlatformLeft: 0,
            iPlatformWidth: 0,
            iPlatformHeight: 0,
            iPlatformPreview: -1,
            editor_platforms_initialized: false,
            iEditState: MAPHAZARD_EDIT_STATE_SELECT,
            iEditMapHazard: 0,
            rBackground: [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2],
            rIconRects: [[SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 2]; MAXMAPHAZARDS as usize],
            editor_maphazards_initialized: false,
            tileset: Ptr::null(),
            view_tileset_repeat_direction: -1,
            view_tileset_repeat_timer: 0,
            editor_tiles_initialized: false,
            modeitemmode: 0,
            dragmodeitem: -1,
            dragoffsetx: 0,
            dragoffsety: 0,
            editor_modeitems_initialized: false,
            iPage: 0,
            sBackgrounds: Global::uninit(),
            rSrc: SDL_Rect { x: 0, y: 0, w: 160, h: 120 },
            rDst: [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; 16],
            editor_backgrounds_initialized: false,
            editor_animation_initialized: false,
        }
    }
}

fn ed() -> &'static mut LevelEditorState {
    &mut game().get().level_editor
}

pub fn bound_to_window_w(x: i32) -> i32 {
    0.max(x.min(640 - 1))
}

pub fn bound_to_window_h(y: i32) -> i32 {
    0.max(y.min(480 - 1))
}

pub fn bound_mouse_motion_coords() {
    unsafe {
        ed().mouse_x = bound_to_window_w(ed().event.motion_x());
        ed().mouse_y = bound_to_window_h(ed().event.motion_y());
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
    pub preview: gfxSprite,
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
            preview: gfxSprite::new(),
        }
    }

    pub fn update_preview(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        unsafe {
            if self.preview.get_surface().is_null() {
                self.preview = gfxSprite::blank(g, 160, 120);
                SDL_SetColorKey(self.preview.get_surface(), SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*self.preview.get_surface()).format, 255, 0, 255));
            }

            crate::common::gfx::blit::fill_rect(self.preview.get_surface(), null(), SDL_MapRGB((*self.preview.get_surface()).format, 255, 0, 255));

            for iPlatformX in 0..MAPWIDTH as i16 {
                for iPlatformY in 0..MAPHEIGHT as i16 {
                    let tile = self.tiles[iPlatformX as usize * MH + iPlatformY as usize];

                    let mut bltrect = SDL_Rect { x: (iPlatformX as i32) << 3, y: (iPlatformY as i32) << 3, w: THUMBTILESIZE, h: THUMBTILESIZE };
                    if tile.iID >= 0 {
                        let src = g.g_tilesetmanager.rect(2, tile.iCol, tile.iRow);
                        crate::common::gfx::blit::upper_blit(gp.g_tilesetmanager.tileset(g, tile.iID as usize).surface(2), src, self.preview.get_surface(), &mut bltrect);
                    } else if tile.iID as i32 == TILESETANIMATED {
                        let src = g.g_tilesetmanager.rect(2, (tile.iCol as i32 * 4) as i16, tile.iRow);
                        g.rm.spr_tileanimation[2].draw_src_to(&*src, self.preview.get_surface(), &bltrect);
                    } else if tile.iID as i32 == TILESETUNKNOWN {
                        //Draw unknown tile
                        let src = g.g_tilesetmanager.rect(2, 0, 0);
                        g.rm.spr_unknowntile[2].draw_src_to(&*src, self.preview.get_surface(), &bltrect);
                    }
                }
            }
        }
    }
}

/// `SDL_GetKeyboardState(NULL)`, which the replay harness replaces while it is active.
fn get_keyboard_state(g: &mut Game) -> *const u8 {
    editor_harness::keyboard_state(g)
}

/// `SDL_GetMouseState(NULL, NULL)`, which the replay harness replaces while it is active.
fn get_mouse_state(g: &mut Game) -> u32 {
    editor_harness::mouse_state(g)
}

pub fn check_key(keystate: *const u8, key: SDL_KeyCode) -> bool {
    unsafe { *keystate.add(SDL_GetScancodeFromKey(key as SDL_Keycode) as usize) != 0 }
}

pub fn set_tileset_tile(tile: &mut TilesetTile, iTileset: i16, iCol: i16, iRow: i16) {
    tile.iID = iTileset;
    tile.iCol = iCol;
    tile.iRow = iRow;
}

pub fn clear_tileset_tile(tile: &mut TilesetTile) {
    set_tileset_tile(tile, TILESETNONE as i16, 0, 0);
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

//main main main
pub fn main() {
    let g = game().get();
    cmd::attach_parent_console();
    crate::globals::init_globals(g);
    unsafe {
        ed().copiedtiles.init([[EditorMapTile::zeroed(); MH]; MW]);
        ed().g_Platforms.init((0..MAX_PLATFORMS).map(|_| MapPlatform::new()).collect());
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
        unsafe { g.RootDataDirectory = cmd.data_root.clone() };
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around inner_main().
    let result = std::panic::catch_unwind(inner_main);
    if let Err(payload) = result {
        let what = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        gfx_show_catched_error(g, &what);
        std::process::exit(1);
    }
}

pub fn inner_main() {
    let g = game().get();
    let mut gp = g.ptr();
    unsafe {
        ensure_settings_dir(g);

        /* This must occur before any data files are loaded */
        initialize_paths(g);

        let mut done: bool;

        println!("-------------------------------------------------------------------------------");
        println!(" {} {}", TITLESTRING, MAPTITLESTRING);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        {
            let options_path = get_home_directory(g) + "leveleditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "rb");
            if editor_settings.is_open() {
                ed().g_fFullScreen = editor_settings.read_bool();
                ed().findstring = editor_settings.read_string_long(FILEBUFSIZE as usize);
            }
        }

        gfx_init(g, 640, 480, ed().g_fFullScreen);
        g.blitdest = g.screen;

        g.rm = Ptr::new_box(CResourceManager::new());
        gp.g_tilesetmanager = Ptr::new_box(CTilesetManager::new(g, &convert_path(gp.get(), "gfx/Classic/tilesets")));
        g.g_map = Ptr::from_box(CMap::new());
        g.filterslist = Ptr::new_box(crate::common::file_list::FiltersList::new(g));
        g.maplist = Ptr::new_box(MapList::new(g, false));
        ed().backgroundlist = Ptr::new_box(BackgroundList::new(g));

        //Add all of the maps that are world only so we can edit them
        gp.maplist.add_world_maps(g);
        let title = format!("{} {}", TITLESTRING, MAPTITLESTRING);
        gfx_settitle(g, &title);

        println!("\n---------------- loading graphics ----------------");

        g.rm.spr_tiletypes = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_tile_types.png")).without_color_key().create(g);
        g.rm.spr_transparenttiles = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_transparent_tiles.png")).with_alpha(160).create(g);

        g.rm.spr_backgroundlevel = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_background_levels.png")).create(g);
        g.rm.spr_tilesetlevel = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_tileset_levels.png")).create(g);

        g.rm.spr_eyecandy = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_eyecandy.png")).create(g);

        ed().s_platform.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_platform.png")).create(g));
        ed().s_platformpathbuttons.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_pathtype_buttons.png")).create(g));
        ed().s_maphazardbuttons.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_maphazard_buttons.png")).create(g));

        g.rm.spr_warps[0] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp.png")).create(g);
        g.rm.spr_warps[1] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp_preview.png")).create(g);
        g.rm.spr_warps[2] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp_thumbnail.png")).create(g);

        g.rm.spr_platformpath = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_platform_path.png")).with_alpha(128).create(g);

        g.rm.spr_selectedtile = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_selectedtile.png")).with_color_key(colors::BLACK).with_alpha(128).create(g);
        g.rm.spr_nospawntile = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_nospawntile.png")).with_color_key(colors::BLACK).with_alpha(128).create(g);
        g.rm.spr_noitemspawntile = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_noitemspawntile.png")).with_color_key(colors::BLACK).with_alpha(128).create(g);
        g.rm.spr_platformstarttile = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_platformstarttile.png")).with_color_key(colors::BLACK).with_alpha(64).with_wrapping(640).create(g);
        g.rm.spr_platformendtile = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_selectedtile.png")).with_color_key(colors::BLACK).with_alpha(64).with_wrapping(640).create(g);

        g.rm.spr_mapitems[0] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_mapitems.png")).create(g);
        g.rm.spr_mapitems[1] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_mapitems_preview.png")).create(g);
        g.rm.spr_mapitems[2] = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_mapitems_thumbnail.png")).create(g);

        g.rm.spr_dialog = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_dialog.png")).with_alpha(255).create(g);
        g.rm.menu_shade = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_shade.png")).with_alpha(128).create(g);

        g.rm.spr_tileanimation[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/tile_animation.png")).create(g);
        g.rm.spr_tileanimation[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_preview.png")).create(g);
        g.rm.spr_tileanimation[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_thumbnail.png")).create(g);

        g.rm.spr_blocks[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/blocks.png")).create(g);
        g.rm.spr_blocks[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/blocks_preview.png")).create(g);
        g.rm.spr_blocks[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/blocks_thumbnail.png")).create(g);

        g.rm.spr_unknowntile[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/unknown_tile.png")).create(g);
        g.rm.spr_unknowntile[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/unknown_tile_preview.png")).create(g);
        g.rm.spr_unknowntile[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/unknown_tile_thumbnail.png")).create(g);

        g.rm.spr_powerups = ImageLoader::new(convert_path(g, "gfx/packs/Classic/powerups/large.png")).create(g);
        g.rm.spr_powerupselector = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_powerup_selector.png")).with_alpha(128).create(g);
        g.rm.spr_hidden_marker = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_hidden_marker.png")).create(g);

        g.rm.spr_flagbases = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/flagbases.png")).create(g);
        g.rm.spr_racegoals = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/racegoal.png")).create(g);

        g.rm.spr_hazard_fireball[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/fireball.png")).create(g);
        g.rm.spr_hazard_fireball[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/fireball_preview.png")).create(g);
        g.rm.spr_hazard_fireball[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/fireball_thumbnail.png")).create(g);

        g.rm.spr_hazard_rotodisc[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/rotodisc.png")).create(g);
        g.rm.spr_hazard_rotodisc[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/rotodisc_preview.png")).create(g);
        g.rm.spr_hazard_rotodisc[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/rotodisc_thumbnail.png")).create(g);

        g.rm.spr_hazard_bulletbill[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/bulletbill.png")).create(g);
        g.rm.spr_hazard_bulletbill[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/bulletbill_preview.png")).create(g);
        g.rm.spr_hazard_bulletbill[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/bulletbill_thumbnail.png")).create(g);

        g.rm.spr_hazard_flame[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/flame.png")).create(g);
        g.rm.spr_hazard_flame[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/flame_preview.png")).create(g);
        g.rm.spr_hazard_flame[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/flame_thumbnail.png")).create(g);

        g.rm.spr_hazard_pirhanaplant[0] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/pirhanaplant.png")).create(g);
        g.rm.spr_hazard_pirhanaplant[1] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/pirhanaplant_preview.png")).create(g);
        g.rm.spr_hazard_pirhanaplant[2] = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/pirhanaplant_thumbnail.png")).create(g);

        g.rm.spr_number_icons = ImageLoader::new(convert_path(g, "gfx/packs/Classic/awards/killsinrownumbers.png")).create(g);

        for i in 0..3usize {
            g.rm.spr_hazard_fireball[i].set_wrap((640 >> i) as i16);
            g.rm.spr_hazard_rotodisc[i].set_wrap((640 >> i) as i16);
            g.rm.spr_hazard_flame[i].set_wrap((640 >> i) as i16);
            g.rm.spr_hazard_pirhanaplant[i].set_wrap((640 >> i) as i16);
        }


        gp.rm.menu_font_small = gfxFont::from_path(g, &convert_path(gp.get(), "gfx/packs/Classic/fonts/font_small.png"));
        gp.rm.menu_font_large = gfxFont::from_path(g, &convert_path(gp.get(), "gfx/packs/Classic/fonts/font_large.png"));

        println!("\n---------------- load map ----------------");

        //Setup Platforms
        const UI_PLATFORM_START_X: i32 = (640 - UI_PLATFORM_COLS * 32 - (UI_PLATFORM_COLS - 1) * 10/* spacing */) / 2;
        const UI_PLATFORM_START_Y: i32 = (480 - UI_PLATFORM_ROWS * 32 - (UI_PLATFORM_ROWS - 1) * 10/* spacing */) / 2;
        for iPlatform in 0..MAX_PLATFORMS as usize {
            let p = &mut ed().g_Platforms[iPlatform];
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

        read_animated_tile_type_file(&convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_tileset.tls"));

        let fs = ed().findstring.clone();
        g.maplist.find(&fs);
        loadcurrentmap(g);
        ed().findstring.clear(); //clear out the find string so that pressing "f" will give you the find dialog

        println!("\n---------------- ready, steady, go! ----------------");

        resetselectedtiles();

        crate::common::game_values::CGameValues::init(&mut gp.game_values, g); // Needed for FPSLimiter

        editor_harness::init(g);
        editor_harness::set_dumper(g, dump_editor_state);

        println!("entering level editor loop...");
        done = false;
        while !done {
            FPSLimiter::instance(g).frame_start(g);

            match ed().state {
                EDITOR_EDIT => {
                    ed().move_nodrag = false;
                    ed().state = editor_edit(g);
                }
                EDITOR_TILES => ed().state = editor_tiles(g),
                EDITOR_BLOCKS => ed().state = editor_blocks(g),
                EDITOR_MAPITEMS => ed().state = editor_mapitems(g),
                EDITOR_MODEITEMS => ed().state = editor_modeitems(g),
                EDITOR_WARP => ed().state = editor_warp(g),
                EDITOR_EYECANDY => ed().state = editor_eyecandy(g),
                EDITOR_PLATFORM => ed().state = editor_platforms(g),
                EDITOR_MAPHAZARDS => ed().state = editor_maphazards(g),
                EDITOR_TILETYPE => ed().state = editor_tiletype(g),
                EDITOR_BACKGROUNDS => ed().state = editor_backgrounds(g),
                EDITOR_ANIMATION => ed().state = editor_animation(g),
                EDITOR_QUIT => done = true,
                DISPLAY_HELP => ed().state = display_help(g),
                SAVE_AS => ed().state = save_as(g),
                FIND => ed().state = find(g),
                CLEAR_MAP => ed().state = clear_map(g),
                NEW_MAP => ed().state = newmap(g),
                SAVE => ed().state = savecurrentmap(g),
                _ => println!(" PANIC: WEIRD STATE: {}", ed().state),
            }

            editor_harness::frame_delay(g, 0);
            FPSLimiter::instance(g).before_flip(g);
            gfx_flipscreen(g);
            FPSLimiter::instance(g).after_flip(g);
        }

        println!("\n---------------- save map ----------------");

        save_map(g, &convert_path(gp.get(), "maps/ZZleveleditor.map"));

        {
            let options_path = get_home_directory(g) + "leveleditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "wb");
            if editor_settings.is_open() {
                editor_settings.write_bool(ed().g_fFullScreen);
                let current = g.maplist.current_filename().to_string();
                editor_settings.write_string_long(&current);
            }
        }

        write_animated_tile_type_file(&convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_tileset.tls"));
        g.g_tilesetmanager.save_tilesets();

        println!("\n---------------- shutdown ----------------");
        crate::common::gfx::blit::report_fallbacks();
    }
}

pub fn calculate_tile_type(g: &mut Game, x: i16, y: i16) -> TileType {
    let mut gp = g.ptr();
    unsafe {
        let mut r#type = TileType::NonSolid;
        let mut k: i16 = MAPLAYERS as i16 - 1;
        while k >= 0 {
            let tile = g.g_map.mapdata[x as usize][y as usize][k as usize];

            let mut iTileType = TileType::NonSolid;
            if tile.iID >= 0 {
                iTileType = gp.g_tilesetmanager.tileset(g, tile.iID as usize).tile_type(tile.iCol as usize, tile.iRow as usize);
            } else if tile.iID as i32 == TILESETANIMATED {
                iTileType = ed().animatedtiletypes[(tile.iRow as i32 + ((tile.iCol as i32) << 5)) as usize];
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

pub fn update_tile_type(g: &mut Game, x: i16, y: i16) {
    unsafe {
        g.g_map.mapdatatop[x as usize][y as usize] = calculate_tile_type(g, x, y);
    }
}

pub fn tile_type_is_modified(g: &mut Game, x: i16, y: i16) -> bool {
        let mut gp = g.ptr();
    unsafe { gp.g_map.mapdatatop[x as usize][y as usize] != calculate_tile_type(g, x, y) }
}

pub fn adjust_map_items(g: &mut Game, x: i16, y: i16) {
    const IGNORED_TILES: [TileType; 3] = [TileType::NonSolid, TileType::SolidOnTop, TileType::IceOnTop];

    unsafe {
        if let Some(it) = g.g_map.mapitems.iter().position(|item: &MapItem| item.ix == x && item.iy == y) {
            let tiletype = g.g_map.mapdatatop[x as usize][y as usize];
            let objtype = g.g_map.objectdata[x as usize][y as usize].iType;

            let tiletypeValid = !IGNORED_TILES.contains(&tiletype);
            let objtypeValid = objtype != -1;
            if tiletypeValid || objtypeValid {
                g.g_map.mapitems.remove(it);
            }
        }
    }
}

/// Removes map items located at the XY position.
pub fn remove_map_item_at(g: &mut Game, x: i16, y: i16) {
    unsafe {
        g.g_map.mapitems.retain(|item: &MapItem| !(item.ix == x && item.iy == y));
    }
}

pub fn init_editor_edit() {
    unsafe {
        if ed().editor_edit_initialized {
            return;
        }

        ed().fExiting = false;
        ed().fSelectedYes = false;
        ed().g_musiccategorydisplaytimer = 0;

        ed().editor_edit_initialized = true;
    }
}

/// The block-painting branch shared by the button-down and motion handlers of `editor_edit`.
unsafe fn paint_block(g: &mut Game, iClickX: i16, iClickY: i16) {
    let b = &mut g.g_map.objectdata[iClickX as usize][iClickY as usize];
    b.iType = ed().set_block as i16;
    b.fHidden = false;

    if ed().set_block == 1 || ed().set_block == 15 {
        for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
            g.g_map.objectdata[iClickX as usize][iClickY as usize].iSettings[iSetting] = default_powerup_setting(0, iSetting);
        }
    } else if (11..=14).contains(&ed().set_block) {
        g.g_map.objectdata[iClickX as usize][iClickY as usize].iSettings[0] = ed().set_block_switch_on as i16;
    }
}

pub fn editor_edit(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    init_editor_edit();

    unsafe {
        if ed().fExiting {
            //handle messages
            while poll_event(&mut ed().event) != 0 {
                if ed().event.type_() == SDL_KEYDOWN_EV {
                    let key = ed().event.key_sym();

                    if key == k(SDLK_LEFT) {
                        ed().fSelectedYes = true;
                    } else if key == k(SDLK_RIGHT) {
                        ed().fSelectedYes = false;
                    } else if ed().event.key_sym() == k(SDLK_KP_ENTER) || ed().event.key_sym() == k(SDLK_RETURN) {
                        if ed().fSelectedYes {
                            return EDITOR_QUIT;
                        }

                        ed().fExiting = false;
                    }
                }
            }
        } else {
            //handle messages
            while poll_event(&mut ed().event) != 0 {
                let keystate = get_keyboard_state(g);

                match ed().event.type_() {
                    SDL_QUIT_EV => return EDITOR_QUIT,

                    SDL_KEYDOWN_EV => {
                        let key = ed().event.key_sym();

                        if key == k(SDLK_ESCAPE) {
                            if ed().g_musiccategorydisplaytimer > 0 {
                                ed().g_musiccategorydisplaytimer = 0;
                            } else if ed().edit_mode != 1 {
                                ed().edit_mode = 1;
                            } else {
                                ed().fSelectedYes = false;
                                ed().fExiting = true;
                            }
                        }

                        if ed().event.key_mod() & (SDL_Keymod::KMOD_LALT as u16 | SDL_Keymod::KMOD_RALT as u16) != 0 {
                            if ed().event.key_sym() == k(SDLK_RETURN) {
                                ed().g_fFullScreen = !ed().g_fFullScreen;
                                gfx_changefullscreen(g, ed().g_fFullScreen);
                                g.blitdest = g.screen;
                            }
                        }

                        if key >= k(SDLK_1) && key <= k(SDLK_4) {
                            if ed().edit_mode == 4 {
                                //no spawn zones
                                ed().nospawn_mode = key - k(SDLK_1) + 1;
                            }
                        }

                        if key == k(SDLK_INSERT) {
                            takescreenshot(g);
                        }

                        if key == k(SDLK_t) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_TILES;
                        }

                        if key == k(SDLK_i) {
                            if ed().edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        let v = !g.g_map.nospawn[ed().nospawn_mode as usize][iCol as usize][iRow as usize];
                                        set_no_spawn(g, ed().nospawn_mode as i16, iCol, iRow, v);
                                    }
                                }
                            } else {
                                ed().editor_edit_initialized = false;
                                return EDITOR_BLOCKS;
                            }
                        }

                        if key == k(SDLK_a) {
                            if ed().edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        set_no_spawn(g, ed().nospawn_mode as i16, iCol, iRow, true);
                                    }
                                }
                            } else {
                                ed().editor_edit_initialized = false;
                                return EDITOR_ANIMATION;
                            }
                        }

                        if key == k(SDLK_o) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_MAPITEMS;
                        }

                        if key == k(SDLK_j) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_MODEITEMS;
                        }

                        if key == k(SDLK_h) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_MAPHAZARDS;
                        }

                        if key == k(SDLK_k) {
                            let iMouseX = ed().mouse_x / TILESIZE;
                            let iMouseY = ed().mouse_y / TILESIZE;

                            let iType = g.g_map.objectdata[iMouseX as usize][iMouseY as usize].iType;
                            if iType == 1 || iType == 15 || iType == 4 || iType == 5 || iType == 17 || iType == 18 || iType == 3 {
                                editor_properties(g, iMouseX as i16, iMouseY as i16);
                            }
                        }

                        //if 'B' is pressed, rotate backgrounds
                        if key == k(SDLK_b) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_BACKGROUNDS;
                        }

                        if key == k(SDLK_g) {
                            ed().backgroundlist.next();

                            g.rm.spr_background = ImageLoader::new(ed().backgroundlist.current_path()).without_color_key().create(g);
                            g.g_map.szBackgroundFile = get_filename_from_path(&ed().backgroundlist.current_path().to_string_lossy());

                            if !check_key(keystate, SDLK_LSHIFT) && !check_key(keystate, SDLK_RSHIFT) {
                                //Set music to background default
                                set_music_category_from_background(g);
                            }
                        }

                        if key == k(SDLK_r) {
                            if ed().g_musiccategorydisplaytimer > 0 && {
                                g.g_map.musicCategoryID += 1;
                                g.g_map.musicCategoryID as i32 >= MAXMUSICCATEGORY
                            } {
                                g.g_map.musicCategoryID = 0;
                            }

                            ed().g_musiccategorydisplaytimer = 90;
                        }

                        if key == k(SDLK_s) {
                            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                                return SAVE_AS;
                            }

                            return SAVE;
                        }

                        if key == k(SDLK_f) {
                            if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) || ed().findstring.is_empty() {
                                return FIND;
                            }

                            findcurrentstring(g);
                        }

                        if key == k(SDLK_DELETE) && (check_key(keystate, SDLK_LCTRL) || check_key(keystate, SDLK_RCTRL)) {
                            return CLEAR_MAP;
                        }

                        if key == k(SDLK_DELETE) || key == k(SDLK_BACKSPACE) {
                            if ed().edit_mode == 3 {
                                clearselectedmaptiles(g);
                                resetselectedtiles();
                            } else if ed().edit_mode == 6 {
                                for iRow in 0..MH {
                                    for iCol in 0..MW {
                                        g.g_map.mapdatatop[iCol][iRow] = TileType::NonSolid;
                                    }
                                }
                            }
                        }

                        if key == k(SDLK_n) {
                            if ed().edit_mode == 4 {
                                for iRow in 0..MAPHEIGHT as i16 {
                                    for iCol in 0..MAPWIDTH as i16 {
                                        set_no_spawn(g, ed().nospawn_mode as i16, iCol, iRow, false);
                                    }
                                }
                            } else {
                                return NEW_MAP;
                            }
                        }

                        if key == k(SDLK_v) {
                            ed().viewblocks = !ed().viewblocks;
                        }

                        if key == k(SDLK_e) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_EYECANDY;
                        }

                        if key == k(SDLK_w) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_WARP;
                        }

                        if key == k(SDLK_l) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_TILETYPE;
                        }

                        if key == k(SDLK_F1) {
                            return DISPLAY_HELP;
                        }

                        if key == k(SDLK_PAGEUP) {
                            loop {
                                g.maplist.prev(false);
                                if g.maplist.is_valid() {
                                    break;
                                }
                            }

                            loadcurrentmap(g);
                        }

                        if key == k(SDLK_PAGEDOWN) {
                            loop {
                                g.maplist.next(false);
                                if g.maplist.is_valid() {
                                    break;
                                }
                            }

                            loadcurrentmap(g);
                        }

                        if key == k(SDLK_y) {
                            ed().selected_layer += 1;
                            if ed().selected_layer >= MAPLAYERS {
                                ed().selected_layer = 0;
                            }
                        }

                        if key == k(SDLK_u) {
                            ed().view_only_layer = !ed().view_only_layer;
                        }

                        if key == k(SDLK_m) {
                            if ed().edit_mode == 3 {
                                ed().move_replace = !ed().move_replace;
                            }

                            ed().edit_mode = 3;
                        }

                        if key == k(SDLK_x) {
                            ed().edit_mode = 4;
                            ed().nospawn_mode = 0;
                        }

                        if key == k(SDLK_z) {
                            ed().edit_mode = 5;
                        }

                        if key == k(SDLK_LCTRL) {
                            ed().move_nodrag = true;
                        }

                        if key == k(SDLK_p) {
                            ed().editor_edit_initialized = false;
                            return EDITOR_PLATFORM;
                        }

                        if key == k(SDLK_c) {
                            if ed().edit_mode == 3 {
                                if copyselectedtiles(g) {
                                    ed().move_mode = 3;
                                    getcenterselection(&mut ed().move_start_x, &mut ed().move_start_y);
                                }
                            }
                        }
                    }

                    SDL_KEYUP_EV => {
                        if ed().event.key_sym() == k(SDLK_LCTRL) {
                            ed().move_nodrag = false;
                        }
                    }

                    SDL_MOUSEBUTTONDOWN_EV => {
                        let iClickX = (ed().event.button_x() / TILESIZE) as i16;
                        let iClickY = (ed().event.button_y() / TILESIZE) as i16;

                        if ed().event.button_button() == BUTTON_LEFT && !ed().ignoreclick {
                            if ed().edit_mode == 0 {
                                //paint selected blocks
                                paint_block(g, iClickX, iClickY);

                                adjust_map_items(g, iClickX, iClickY);
                            } else if ed().edit_mode == 1 {
                                //paint selected tile(s)
                                for i in 0..ed().set_tile_cols as i16 {
                                    let iLocalX = (iClickX as i32 + i as i32) as i16;

                                    for j in 0..ed().set_tile_rows as i16 {
                                        let iLocalY = (iClickY as i32 + j as i32) as i16;

                                        if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                                            let fNeedUpdate = !tile_type_is_modified(g, iLocalX, iLocalY);
                                            set_tileset_tile(
                                                &mut g.g_map.mapdata[iLocalX as usize][iLocalY as usize][ed().selected_layer as usize],
                                                ed().set_tile_tileset as i16,
                                                (ed().set_tile_start_x + i as i32) as i16,
                                                (ed().set_tile_start_y + j as i32) as i16,
                                            );

                                            if fNeedUpdate {
                                                update_tile_type(g, iLocalX, iLocalY);
                                            }

                                            adjust_map_items(g, iLocalX, iLocalY);
                                        }
                                    }
                                }
                            } else if ed().edit_mode == 2 {
                                //warps
                                g.g_map.warpdata[iClickX as usize][iClickY as usize].direction = ed().set_direction as WarpEnterDirection;
                                g.g_map.warpdata[iClickX as usize][iClickY as usize].connection = ed().set_connection as i16;
                            } else if ed().edit_mode == 3 {
                                //move tiles
                                if ed().move_mode == 3 {
                                    ed().move_mode = 0;

                                    pasteselectedtiles(g, ed().move_offset_x, ed().move_offset_y);

                                    copymoveselection();
                                    resetselectedtiles();
                                    pastemoveselection(ed().move_offset_x, ed().move_offset_y);

                                    ed().move_offset_x = 0;
                                    ed().move_offset_y = 0;
                                } else {
                                    if ed().selectedtiles[iClickX as usize][iClickY as usize] {
                                        if copyselectedtiles(g) {
                                            ed().move_mode = 1;
                                            clearselectedmaptiles(g);
                                        }
                                    } else {
                                        if !check_key(keystate, SDLK_LSHIFT) && !check_key(keystate, SDLK_RSHIFT) && !check_key(keystate, SDLK_LCTRL) {
                                            resetselectedtiles();
                                        }

                                        if ed().move_nodrag {
                                            ed().selectedtiles[iClickX as usize][iClickY as usize] = true;
                                        } else {
                                            ed().move_mode = 2;
                                            ed().move_drag_start_x = iClickX as i32;
                                            ed().move_drag_start_y = iClickY as i32;
                                        }
                                    }

                                    ed().move_start_x = iClickX as i32;
                                    ed().move_start_y = iClickY as i32;
                                }
                            } else if ed().edit_mode == 4 {
                                //no player spawn areas
                                set_no_spawn(g, ed().nospawn_mode as i16, iClickX, iClickY, true);
                            } else if ed().edit_mode == 5 {
                                // no item spawn areas
                                g.g_map.nospawn[5][iClickX as usize][iClickY as usize] = true;
                            } else if ed().edit_mode == 6 {
                                //tile types
                                g.g_map.mapdatatop[iClickX as usize][iClickY as usize] = ed().set_tiletype;
                                adjust_map_items(g, iClickX, iClickY);
                            } else if ed().edit_mode == 7 {
                                //map items
                                if (g.g_map.mapitems.len() as i32) < MAXMAPITEMS {
                                    let mut fTileNotAvailable = false;
                                    for item in g.g_map.mapitems.iter() {
                                        if item.ix == iClickX && item.iy == iClickY {
                                            fTileNotAvailable = true;
                                            break;
                                        }
                                    }
                                    let top = g.g_map.mapdatatop[iClickX as usize][iClickY as usize];
                                    if top != TileType::NonSolid && top != TileType::SolidOnTop && top != TileType::IceOnTop {
                                        fTileNotAvailable = true;
                                    }
                                    if !fTileNotAvailable {
                                        let mapitem = MapItem { itype: ed().set_mapitem as MapItemType, ix: iClickX, iy: iClickY };
                                        g.g_map.mapitems.push(mapitem);
                                    }
                                }
                            } else if ed().edit_mode == 8 {
                                //animated tiles
                                paint_animated_tiles(g, iClickX, iClickY);
                            }
                        } else if ed().event.button_button() == BUTTON_RIGHT {
                            erase_at(g, iClickX, iClickY, true);
                        }
                    }

                    SDL_MOUSEMOTION_EV => {
                        bound_mouse_motion_coords();
                        let iClickX = (bound_to_window_w(ed().event.motion_x()) / TILESIZE) as i16;
                        let iClickY = (bound_to_window_h(ed().event.motion_y()) / TILESIZE) as i16;

                        if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) && !ed().ignoreclick {
                            if ed().edit_mode == 0 {
                                paint_block(g, iClickX, iClickY);

                                adjust_map_items(g, iClickX, iClickY);
                            } else if ed().edit_mode == 1 {
                                for i in 0..ed().set_tile_cols as i16 {
                                    let iLocalX = (iClickX as i32 + i as i32) as i16;

                                    for j in 0..ed().set_tile_rows as i16 {
                                        let iLocalY = (iClickY as i32 + j as i32) as i16;

                                        if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                                            let fNeedUpdate = !tile_type_is_modified(g, iLocalX, iLocalY);

                                            set_tileset_tile(
                                                &mut g.g_map.mapdata[iLocalX as usize][iLocalY as usize][ed().selected_layer as usize],
                                                ed().set_tile_tileset as i16,
                                                (ed().set_tile_start_x + i as i32) as i16,
                                                (ed().set_tile_start_y + j as i32) as i16,
                                            );

                                            if fNeedUpdate {
                                                update_tile_type(g, iLocalX, iLocalY);
                                            }

                                            // (sic) the C++ passes iLocalY twice here.
                                            adjust_map_items(g, iLocalY, iLocalY);
                                        }
                                    }
                                }
                            } else if ed().edit_mode == 2 {
                                g.g_map.warpdata[iClickX as usize][iClickY as usize].direction = ed().set_direction as WarpEnterDirection;
                                g.g_map.warpdata[iClickX as usize][iClickY as usize].connection = ed().set_connection as i16;
                            } else if ed().edit_mode == 3 {
                                if ed().move_mode == 0 {
                                    ed().selectedtiles[iClickX as usize][iClickY as usize] = true;
                                }
                            } else if ed().edit_mode == 4 {
                                set_no_spawn(g, ed().nospawn_mode as i16, iClickX, iClickY, true);
                            } else if ed().edit_mode == 5 {
                                g.g_map.nospawn[5][iClickX as usize][iClickY as usize] = true;
                            } else if ed().edit_mode == 6 {
                                g.g_map.mapdatatop[iClickX as usize][iClickY as usize] = ed().set_tiletype;
                                adjust_map_items(g, iClickX, iClickY);
                            } else if ed().edit_mode == 8 {
                                paint_animated_tiles(g, iClickX, iClickY);
                            }
                        } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) {
                            erase_at(g, iClickX, iClickY, false);
                        }
                    }

                    SDL_MOUSEBUTTONUP_EV => {
                        let iClickX = ed().event.button_x() / TILESIZE;
                        let iClickY = ed().event.button_y() / TILESIZE;

                        if ed().event.button_button() == BUTTON_LEFT {
                            ed().ignoreclick = false;

                            if ed().move_mode == 1 {
                                pasteselectedtiles(g, ed().move_offset_x, ed().move_offset_y);

                                copymoveselection();
                                resetselectedtiles();
                                pastemoveselection(ed().move_offset_x, ed().move_offset_y);

                                ed().move_offset_x = 0;
                                ed().move_offset_y = 0;
                            } else if ed().move_mode == 2 {
                                let left = if ed().move_drag_start_x < iClickX { ed().move_drag_start_x } else { iClickX };
                                let top = if ed().move_drag_start_y < iClickY { ed().move_drag_start_y } else { iClickY };
                                let right = if ed().move_drag_start_x < iClickX { iClickX } else { ed().move_drag_start_x };
                                let bottom = if ed().move_drag_start_y < iClickY { iClickY } else { ed().move_drag_start_y };

                                for kk in top..=bottom {
                                    for j in left..=right {
                                        ed().selectedtiles[j as usize][kk as usize] = true;
                                    }
                                }
                            }

                            ed().move_mode = 0;
                        }
                    }

                    _ => {}
                }
            }

            if ed().move_mode == 1 || ed().move_mode == 3 {
                ed().move_offset_x = (ed().mouse_x / TILESIZE) - ed().move_start_x;
                ed().move_offset_y = (ed().mouse_y / TILESIZE) - ed().move_start_y;
            } else if ed().move_mode == 2 {
                ed().move_drag_offset_x = ed().mouse_x / TILESIZE;
                ed().move_drag_offset_y = ed().mouse_y / TILESIZE;
            }
        }

        if g.maplist.is_valid() {
            drawmap(g, false, TILESIZE as i16, false);
        } else {
            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);
            gp.rm.menu_font_large.draw_centered(g, 320, 200, "Map has been deleted.");
        }

        //Ask if you are sure you want to exit
        if ed().fExiting {
            gp.rm.spr_dialog.draw_src(g, 224, 176, &rect(0, 0, 192, 128));
            gp.rm.menu_font_large.draw_centered(g, 320, 195, "Exit");
            gp.rm.menu_font_large.draw_centered(g, 320, 220, "Are You Sure?");
            gp.rm.menu_font_large.draw_centered(g, 282, 254, "Yes");
            gp.rm.menu_font_large.draw_centered(g, 356, 254, "No");

            gp.rm.spr_dialog.draw_src(g, if ed().fSelectedYes { 250 } else { 326 }, 250, &rect(192, 0, 64, 32));
        } else {
            if ed().edit_mode == 0 {
                gp.rm.menu_font_small.draw(g, 0, 0, "Block Mode");
            } else if ed().edit_mode == 1 || ed().edit_mode == 8 {
                let mut modestring = String::new();

                if ed().edit_mode == 1 {
                    modestring.push_str("Tile Mode - ");
                } else {
                    modestring.push_str("Animated Tile Mode - ");
                }

                if ed().selected_layer == 0 {
                    modestring.push_str("Bottom Background");
                } else if ed().selected_layer == 1 {
                    modestring.push_str("Top Background");
                } else if ed().selected_layer == 2 {
                    modestring.push_str("Bottom Foreground");
                } else if ed().selected_layer == 3 {
                    modestring.push_str("Top Foreground");
                }

                if ed().view_only_layer {
                    modestring.push_str(" Only");
                }

                gp.rm.menu_font_small.draw(g, 0, 0, &modestring);

                draw_layer_indicator(g);
            } else if ed().edit_mode == 2 {
                gp.rm.menu_font_small.draw(g, 0, 0, "Warp Mode");
            } else if ed().edit_mode == 3 {
                for kk in 0..MAPHEIGHT {
                    for j in 0..MAPWIDTH {
                        if ed().selectedtiles[j as usize][kk as usize] {
                            gp.rm.spr_selectedtile.draw(g, (j + ed().move_offset_x) * TILESIZE, (kk + ed().move_offset_y) * TILESIZE);
                        }
                    }
                }

                //Draw dragging selection
                if ed().move_mode == 2 {
                    let left = if ed().move_drag_start_x < ed().move_drag_offset_x { ed().move_drag_start_x } else { ed().move_drag_offset_x };
                    let top = if ed().move_drag_start_y < ed().move_drag_offset_y { ed().move_drag_start_y } else { ed().move_drag_offset_y };
                    let right = if ed().move_drag_start_x < ed().move_drag_offset_x { ed().move_drag_offset_x } else { ed().move_drag_start_x };
                    let bottom = if ed().move_drag_start_y < ed().move_drag_offset_y { ed().move_drag_offset_y } else { ed().move_drag_start_y };

                    for kk in top..=bottom {
                        for j in left..=right {
                            if !ed().selectedtiles[j as usize][kk as usize] {
                                gp.rm.spr_selectedtile.draw(g, j * TILESIZE, kk * TILESIZE);
                            }
                        }
                    }
                }

                if ed().move_replace {
                    gp.rm.menu_font_small.draw(g, 0, 0, "Move Mode - Replace");
                } else {
                    gp.rm.menu_font_small.draw(g, 0, 0, "Move Mode - Merge");
                }

                draw_layer_indicator(g);
            } else if ed().edit_mode == 4 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g.g_map.nospawn[ed().nospawn_mode as usize][j][kk] {
                            gp.rm.spr_nospawntile.draw_src(g, j as i32 * TILESIZE, kk as i32 * TILESIZE, &rect(ed().nospawn_mode * 32, 0, 32, 32));
                        }

                        if ed().nospawn_mode > 0 {
                            if g.g_map.nospawn[0][j][kk] {
                                gp.rm.spr_nospawntile.draw_src(g, j as i32 * TILESIZE, kk as i32 * TILESIZE, &rect(0, 0, 32, 32));
                            }
                        }
                    }
                }

                gp.rm.menu_font_small.draw(g, 0, 480 - (gp.rm.menu_font_small.get_height() << 1), "No Player Spawn: [x] Global, [1-4] Team Spawn Zone");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[a] All, [n] None, [i] Invert");
            } else if ed().edit_mode == 5 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g.g_map.nospawn[5][j][kk] {
                            gp.rm.spr_noitemspawntile.draw(g, j as i32 * TILESIZE, kk as i32 * TILESIZE);
                        }
                    }
                }

                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "No Item Spawn");
            } else if ed().edit_mode == 6 {
                for kk in 0..MH {
                    for j in 0..MW {
                        if g.g_map.mapdatatop[j][kk] != TileType::NonSolid {
                            gp.rm.spr_transparenttiles.draw_src(
                                g,
                                j as i32 * TILESIZE,
                                kk as i32 * TILESIZE,
                                &rect(prev_tile_type(gp.g_map.mapdatatop[j][kk]).0 as i32 * TILESIZE, 0, TILESIZE, TILESIZE),
                            );
                        }
                    }
                }

                gp.rm.menu_font_small.draw(g, 0, 0, "Tile Type Mode");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Press [Delete] To Clear All Tile Types");
            } else if ed().edit_mode == 7 {
                gp.rm.menu_font_small.draw(g, 0, 0, "Map Item Mode");
            }

            gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

            if ed().g_musiccategorydisplaytimer > 0 {
                ed().g_musiccategorydisplaytimer -= 1;

                gp.rm.spr_dialog.draw_src(g, 224, 176, &rect(0, 0, 192, 128));
                gp.rm.menu_font_small.draw_centered(g, 320, 195, "Music Category");
                gp.rm.menu_font_large.draw_centered(g, 320, 220, g_szMusicCategoryNames[g.g_map.musicCategoryID as usize]);

                gp.rm.menu_font_small.draw_centered(g, 320, 255, "Press 'R' Again");
                gp.rm.menu_font_small.draw_centered(g, 320, 270, "To Change");
            }

            draw_message(g);
        }

        EDITOR_EDIT
    }
}

unsafe fn draw_layer_indicator(g: &mut Game) {
        let mut gp = g.ptr();
    if ed().view_only_layer {
        gp.rm.spr_backgroundlevel.draw_src(g, 2, 18 + (3 - ed().selected_layer) * 18, &rect(ed().selected_layer * 16, (3 - ed().selected_layer) * 18, 16, 16));
    } else {
        gp.rm.spr_backgroundlevel.draw_src(g, 2, 18, &rect(ed().selected_layer * 16, 0, 16, 70));
    }
}

/// `strncmp(name, szBackgroundFile, strlen(name))` over the music category names.
unsafe fn set_music_category_from_background(g: &mut Game) {
    for iCategory in 0..MAXMUSICCATEGORY as i16 {
        if g.g_map.szBackgroundFile.as_bytes().starts_with(g_szMusicCategoryNames[iCategory as usize].as_bytes()) {
            g.g_map.musicCategoryID = iCategory;
            break;
        }
    }
}

unsafe fn paint_animated_tiles(g: &mut Game, iClickX: i16, iClickY: i16) {
    for i in 0..ed().set_tile_cols as i16 {
        let iLocalX = (iClickX as i32 + i as i32) as i16;

        for j in 0..ed().set_tile_rows as i16 {
            let iLocalY = (iClickY as i32 + j as i32) as i16;

            if iLocalX >= 0 && (iLocalX as i32) < MAPWIDTH && iLocalY >= 0 && (iLocalY as i32) < MAPHEIGHT {
                let fNeedUpdate = !tile_type_is_modified(g, iLocalX, iLocalY);
                set_tileset_tile(
                    &mut g.g_map.mapdata[iLocalX as usize][iLocalY as usize][ed().selected_layer as usize],
                    TILESETANIMATED as i16,
                    (ed().set_tile_start_y + j as i32) as i16,
                    (ed().set_tile_start_x + i as i32) as i16,
                );

                if fNeedUpdate {
                    update_tile_type(g, iLocalX, iLocalY);
                }
            }
        }
    }
}

/// The right-button handlers of `editor_edit`; only the button-down one removes map items.
unsafe fn erase_at(g: &mut Game, iClickX: i16, iClickY: i16, fButtonDown: bool) {
    let (x, y) = (iClickX as usize, iClickY as usize);
    if ed().edit_mode == 0 {
        g.g_map.objectdata[x][y].iType = -1;
    } else if ed().edit_mode == 1 || ed().edit_mode == 8 {
        let fNeedUpdate = !tile_type_is_modified(g, iClickX, iClickY);
        g.g_map.mapdata[x][y][ed().selected_layer as usize].iID = TILESETNONE as i16;

        if fNeedUpdate {
            update_tile_type(g, iClickX, iClickY);
        }
    } else if ed().edit_mode == 2 {
        g.g_map.warpdata[x][y].direction = WARP_UNDEFINED;
        g.g_map.warpdata[x][y].connection = -1;
    } else if ed().edit_mode == 3 {
        if fButtonDown {
            if ed().move_mode == 3 {
                ed().move_mode = 0;
                resetselectedtiles();
                ed().move_offset_x = 0;
                ed().move_offset_y = 0;
            } else if ed().selectedtiles[x][y] {
                ed().selectedtiles[x][y] = false;
            } else {
                resetselectedtiles();
            }
        } else if ed().move_mode == 0 {
            ed().selectedtiles[x][y] = false;
        }
    } else if ed().edit_mode == 4 {
        set_no_spawn(g, ed().nospawn_mode as i16, iClickX, iClickY, false);
    } else if ed().edit_mode == 5 {
        g.g_map.nospawn[5][x][y] = false;
    } else if ed().edit_mode == 6 {
        g.g_map.mapdatatop[x][y] = TileType::NonSolid;
    } else if ed().edit_mode == 7 && fButtonDown {
        remove_map_item_at(g, iClickX, iClickY);
    }
}

pub fn draw_message(g: &mut Game) {
        let mut gp = g.ptr();
    unsafe {
        if ed().g_messagedisplaytimer > 0 {
            ed().g_messagedisplaytimer -= 1;

            gp.rm.spr_dialog.draw_src(g, 224, 176, &rect(0, 0, 192, 128));
            gp.rm.menu_font_large.draw_centered(g, 320, 195, &ed().g_szMessageTitle);
            gp.rm.menu_font_large.draw_centered(g, 320, 220, &ed().g_szMessageLine[0]);
            gp.rm.menu_font_large.draw_centered(g, 320, 240, &ed().g_szMessageLine[1]);
            gp.rm.menu_font_large.draw_centered(g, 320, 260, &ed().g_szMessageLine[2]);
        }
    }
}

pub fn set_no_spawn(g: &mut Game, nospawnmode: i16, col: i16, row: i16, value: bool) {
    unsafe {
        if !value && nospawnmode > 0 && g.g_map.nospawn[0][col as usize][row as usize] {
            return;
        }

        g.g_map.nospawn[nospawnmode as usize][col as usize][row as usize] = value;

        if nospawnmode == 0 {
            for imode in 1..5usize {
                g.g_map.nospawn[imode][col as usize][row as usize] = value;
            }
        }
    }
}

fn in_moved_selection(i: i32, j: i32) -> bool {
    unsafe {
        (ed().move_mode == 1 || ed().move_mode == 3)
            && i - ed().move_offset_x >= 0
            && i - ed().move_offset_x < MAPWIDTH
            && j - ed().move_offset_y >= 0
            && j - ed().move_offset_y < MAPHEIGHT
            && ed().selectedtiles[(i - ed().move_offset_x) as usize][(j - ed().move_offset_y) as usize]
    }
}

pub fn drawlayer(g: &mut Game, layer: i32, fUseCopied: bool, iBlockSize: i16) {
    let mut gp = g.ptr();
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
                        tile = Some(ed().copiedtiles[(i as i32 - ed().move_offset_x) as usize][(j as i32 - ed().move_offset_y) as usize].tile[layer as usize]);
                    }
                } else if !fUseCopied {
                    tile = Some(g.g_map.mapdata[i as usize][j as usize][layer as usize]);
                }

                let tile = match tile {
                    Some(t) if t.iID as i32 != TILESETNONE => t,
                    _ => continue,
                };

                if tile.iID >= 0 {
                    gp.g_tilesetmanager.draw(g, g.screen, tile.iID, iTilesetIndex, tile.iCol, tile.iRow, i, j);
                } else if tile.iID as i32 == TILESETANIMATED {
                    let mut iSrcCol: i16 = ((tile.iCol as i32) << 2) as i16;
                    let mut iSrcRow: i16 = tile.iRow;

                    if iSrcCol > 31 || iSrcCol < 0 || iSrcRow > 31 || iSrcRow < 0 {
                        iSrcCol = 0;
                        iSrcRow = 0;
                    }

                    let src = g.g_tilesetmanager.rect(iTilesetIndex, iSrcCol, iSrcRow);
                    let dst = g.g_tilesetmanager.rect(iTilesetIndex, i, j);
                    crate::common::gfx::blit::upper_blit(g.rm.spr_tileanimation[iTilesetIndex as usize].get_surface(), src, g.screen, dst);
                } else if tile.iID as i32 == TILESETUNKNOWN {
                    let src = g.g_tilesetmanager.rect(iTilesetIndex, 0, 0);
                    let dst = g.g_tilesetmanager.rect(iTilesetIndex, i, j);
                    crate::common::gfx::blit::upper_blit(g.rm.spr_unknowntile[iTilesetIndex as usize].get_surface(), src, g.screen, dst);
                }
            }
        }
    }
}

pub fn drawmap(g: &mut Game, fScreenshot: bool, iBlockSize: i16, fWithPlatforms: bool) {
        let mut gp = g.ptr();
    unsafe {
        let bs = iBlockSize as i32;
        if bs != TILESIZE {
            let srcrect = rect(0, 0, 640, 480);
            let dstrect = rect(0, 0, bs * 20, bs * 15);

            g.rm.spr_background.draw_stretch(&srcrect, g.blitdest, &dstrect);
        } else {
            gp.rm.spr_background.draw(g, 0, 0);
        }

        if (ed().view_only_layer && ed().selected_layer == 0) || !ed().view_only_layer {
            drawlayer(g, 0, false, iBlockSize);
        }

        if !fScreenshot {
            if (ed().view_only_layer && ed().copiedlayer == 0) || !ed().view_only_layer {
                drawlayer(g, 0, true, iBlockSize);
            }
        }

        if (ed().view_only_layer && ed().selected_layer == 1) || !ed().view_only_layer {
            drawlayer(g, 1, false, iBlockSize);
        }

        if !fScreenshot {
            if (ed().view_only_layer && ed().copiedlayer == 1) || !ed().view_only_layer {
                drawlayer(g, 1, true, iBlockSize);
            }
        }

        if fWithPlatforms {
            gp.g_map.draw_platforms(g, 0);
        }

        if (ed().viewblocks && !ed().view_only_layer) || fScreenshot {
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
                        ed().copiedtiles[(i - ed().move_offset_x) as usize][(j - ed().move_offset_y) as usize].block
                    } else {
                        g.g_map.objectdata[i as usize][j as usize]
                    };

                    let blocktype = block.iType as i32;

                    if blocktype > -1 {
                        //Don't screenshot hidden blocks
                        if fScreenshot && g.g_map.objectdata[i as usize][j as usize].fHidden {
                            continue;
                        }

                        if blocktype < 7 {
                            rSrcBlock.x = blocktype * bs;
                            rSrcBlock.y = 0;
                        } else if (7..=10).contains(&blocktype) {
                            //On/Off Blocks
                            rSrcBlock.x = blocktype * bs;
                            rSrcBlock.y = bs * (1 - g.g_map.iSwitches[((blocktype - 7) % 4) as usize] as i32);
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

                        let dst = g.g_tilesetmanager.rect(iTilesizeIndex, i as i16, j as i16);
                        crate::common::gfx::blit::upper_blit(g.rm.spr_blocks[iTilesizeIndex as usize].get_surface(), &rSrcBlock, g.screen, dst);
                    }
                }
            }
        }

        if fWithPlatforms {
            gp.g_map.draw_platforms(g, 1);
        }

        if !ed().view_only_layer || fScreenshot {
            let idx = if bs == TILESIZE {
                0
            } else if bs == PREVIEWTILESIZE {
                1
            } else {
                2
            };
            for item in g.g_map.mapitems.clone().iter() {
                gp.rm.spr_mapitems[idx].draw_src(g, item.ix as i32 * bs, item.iy as i32 * bs, &rect(item.itype as i32 * bs, 0, bs, bs));
            }

            if !fScreenshot {
                for j in 0..MAPHEIGHT {
                    for i in 0..MAPWIDTH {
                        let iNewX = (i - ed().move_offset_x) as i16;
                        let iNewY = (j - ed().move_offset_y) as i16;

                        if (ed().move_mode == 1 || ed().move_mode == 3)
                            && iNewX >= 0
                            && (iNewX as i32) < MAPWIDTH
                            && iNewY >= 0
                            && (iNewY as i32) < MAPHEIGHT
                            && ed().selectedtiles[iNewX as usize][iNewY as usize]
                        {
                            let item = ed().copiedtiles[iNewX as usize][iNewY as usize].item;
                            if item >= 0 {
                                gp.rm.spr_mapitems[0].draw_src(g, i << 5, j << 5, &rect(item << 5, 0, TILESIZE, TILESIZE));
                            }
                        }
                    }
                }
            }
        }

        if fWithPlatforms {
            gp.g_map.draw_platforms(g, 2);
        }

        if (ed().view_only_layer && ed().selected_layer == 2) || !ed().view_only_layer {
            drawlayer(g, 2, false, iBlockSize);
        }

        if !fScreenshot {
            if (ed().view_only_layer && ed().copiedlayer == 2) || !ed().view_only_layer {
                drawlayer(g, 2, true, iBlockSize);
            }
        }

        if (ed().view_only_layer && ed().selected_layer == 3) || !ed().view_only_layer {
            drawlayer(g, 3, false, iBlockSize);
        }

        if !fScreenshot {
            if (ed().view_only_layer && ed().copiedlayer == 3) || !ed().view_only_layer {
                drawlayer(g, 3, true, iBlockSize);
            }
        }

        if fWithPlatforms {
            gp.g_map.draw_platforms(g, 3);
        }

        if (ed().viewwarps && !ed().view_only_layer) || fScreenshot {
            for j in 0..MAPHEIGHT {
                for i in 0..MAPWIDTH {
                    let warp: Warp = if in_moved_selection(i, j) {
                        ed().copiedtiles[(i - ed().move_offset_x) as usize][(j - ed().move_offset_y) as usize].warp
                    } else {
                        g.g_map.warpdata[i as usize][j as usize]
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
                        g.rm.spr_warps[idx].draw_src_to(&rSrcWarp, g.screen, &rDstWarp);
                    }
                }
            }
        }

        if fWithPlatforms {
            gp.g_map.draw_platforms(g, 4);
        }
    }
}

pub fn editor_warp(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    ed().edit_mode = 2; //change to edit mode using warps
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if ed().event.button_button() == BUTTON_LEFT {
                        if ed().event.button_x() / TILESIZE < 10 && ed().event.button_y() / TILESIZE < 4 {
                            ed().set_direction = ed().event.button_y() / TILESIZE;
                            ed().set_connection = ed().event.button_x() / TILESIZE;
                        }

                        ed().edit_mode = 2; //change to edit mode using warps

                        //The user must release the mouse button before trying to add a tile
                        ed().ignoreclick = true;

                        return EDITOR_EDIT;
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        ed().r.x = 0;
        ed().r.y = 0;
        ed().r.w = 640;
        ed().r.h = 480;

        g.rm.spr_warps[0].draw_to(g.screen, &ed().r);
        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        draw_message(g);
        EDITOR_WARP
    }
}

const NUM_EYECANDY: i16 = 7;
const szEyecandyNames: [&str; NUM_EYECANDY as usize] = ["Clouds", "Ghosts", "Leaves", "Snow", "Fish", "Rain", "Bubbles"];
const szLayerNames: [&str; 3] = ["Back Layer", "Mid Layer", "Top Layer"];

pub fn editor_eyecandy(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            let ty = ed().event.type_();
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                if ed().event.key_sym() == k(SDLK_ESCAPE) || ed().event.key_sym() == k(SDLK_e) {
                    return EDITOR_EDIT;
                }
                fall = true;
            }

            if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                if ed().event.button_button() == BUTTON_LEFT {
                    let mut ix: i16 = 165;
                    for iLayer in 0..3usize {
                        for kk in 0..NUM_EYECANDY {
                            if ed().event.button_x() >= ix as i32
                                && ed().event.button_x() < ix as i32 + 90
                                && ed().event.button_y() >= kk as i32 * 65 + 20
                                && ed().event.button_y() < kk as i32 * 65 + 72
                            {
                                let mask: i16 = 1 << kk;
                                if g.g_map.eyecandy[iLayer] & mask != 0 {
                                    g.g_map.eyecandy[iLayer] &= !mask;
                                } else {
                                    g.g_map.eyecandy[iLayer] |= mask;
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

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        let mut ix: i16 = 165;
        for iLayer in 0..3usize {
            for kk in 0..NUM_EYECANDY {
                let iy: i16 = kk * 65 + 20;

                if ed().mouse_x >= ix as i32 && ed().mouse_x < ix as i32 + 90 && ed().mouse_y >= iy as i32 && ed().mouse_y < iy as i32 + 52 {
                    gp.rm.spr_powerupselector.draw_src(g, ix as i32, iy as i32, &rect(0, 0, 90, 52));
                } else {
                    gp.rm.spr_powerupselector.draw_src(g, ix as i32, iy as i32, &rect(0, 52, 90, 52));
                }

                gp.rm.spr_eyecandy.draw_src(g, ix as i32 + 10, iy as i32 + 10, &rect((kk as i32) << 5, 0, 32, 32));

                let mask: i16 = 1 << kk;
                if g.g_map.eyecandy[iLayer] & mask != 0 {
                    gp.rm.spr_hidden_marker.draw(g, ix as i32 + 57, iy as i32 + 16);
                }
            }

            gp.rm.menu_font_small.draw_centered(g, ix as i32 + 45, 10, szLayerNames[iLayer]);
            ix += 110;
        }

        for kk in 0..NUM_EYECANDY {
            gp.rm.menu_font_small.draw_centered(g, 320, kk as i32 * 65 + 62, szEyecandyNames[kk as usize]);
        }

        gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Eyecandy: [e] Exit, [LMB] Choose Eyecandy");

        draw_message(g);
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

pub fn init_editor_properties(g: &mut Game, iBlockCol: i16, iBlockRow: i16) {
    unsafe {
        if ed().editor_properties_initialized {
            return;
        }

        ed().iBlockType = g.g_map.objectdata[iBlockCol as usize][iBlockRow as usize].iType;

        ed().editor_properties_initialized = true;
    }
}

pub fn editor_properties(g: &mut Game, iBlockCol: i16, iBlockRow: i16) -> i32 {
        let mut gp = g.ptr();
    init_editor_properties(g, iBlockCol, iBlockRow);
    unsafe {
        let (bc, br) = (iBlockCol as usize, iBlockRow as usize);
        loop {
            FPSLimiter::instance(g).frame_start(g);
            //handle messages
            while poll_event(&mut ed().event) != 0 {
                let ty = ed().event.type_();
                let mut fall = false;

                if ty == SDL_QUIT_EV {
                    return EDITOR_QUIT;
                }

                if ty == SDL_KEYDOWN_EV {
                    let sym = ed().event.key_sym();
                    if sym == k(SDLK_ESCAPE) || sym == k(SDLK_k) {
                        ed().editor_properties_initialized = false;
                        return EDITOR_EDIT;
                    } else if (ed().iBlockType == 1 || ed().iBlockType == 15) && ((sym >= k(SDLK_0) && sym <= k(SDLK_9)) || sym == k(SDLK_BACKQUOTE) || sym == k(SDLK_d)) {
                        let mut iSettingIndex: i16 = 0;
                        let piSetting = get_block_property(ed().mouse_x, ed().mouse_y, Some(&mut iSettingIndex));

                        //If shift is held, set all powerups to this setting
                        let mut iValue: i16 = (sym - k(SDLK_0)) as i16;

                        if sym == k(SDLK_0) {
                            iValue = 10;
                        } else if sym == k(SDLK_BACKQUOTE) {
                            iValue = 0;
                        } else if sym == k(SDLK_d) {
                            iValue = default_powerup_setting(0, iSettingIndex as usize);
                        }

                        let keystate = get_keyboard_state(g);
                        if check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT) {
                            for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                                if ed().event.key_sym() == k(SDLK_d) {
                                    iValue = default_powerup_setting(0, iSetting);
                                }

                                g.g_map.objectdata[bc][br].iSettings[iSetting] = iValue;
                            }
                        } else if let Some(s) = piSetting {
                            g.g_map.objectdata[bc][br].iSettings[s] = iValue;
                        }
                    }
                    fall = true;
                }

                if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                    let mut iHiddenCheckboxY: i16 = 0;
                    if ed().iBlockType == 1 || ed().iBlockType == 15 {
                        let mut iSettingIndex: i16 = 0;
                        let piSetting = get_block_property(ed().event.button_x(), ed().event.button_y(), Some(&mut iSettingIndex));

                        if let Some(s) = piSetting {
                            let iMouseState = get_mouse_state(g) as u8 as u32;
                            let setting = &mut g.g_map.objectdata[bc][br].iSettings[s];

                            if (ed().event.button_button() == BUTTON_RIGHT && (iMouseState & SDL_BUTTON_LMASK) != 0)
                                || (ed().event.button_button() == BUTTON_LEFT && (iMouseState & SDL_BUTTON_RMASK) != 0)
                            {
                                *setting = default_powerup_setting(0, iSettingIndex as usize);
                            } else if ed().event.button_button() == BUTTON_LEFT {
                                if *setting < 10 {
                                    *setting += 1;
                                }
                            } else if ed().event.button_button() == BUTTON_RIGHT {
                                if *setting > 0 {
                                    *setting -= 1;
                                }
                            }
                        }

                        iHiddenCheckboxY = 365;
                    } else if ed().iBlockType == 4 || ed().iBlockType == 5 || ed().iBlockType == 17 || ed().iBlockType == 18 || ed().iBlockType == 3 {
                        iHiddenCheckboxY = 214;
                    }

                    if ed().event.button_x() >= 270 && ed().event.button_x() < 370 && ed().event.button_y() >= iHiddenCheckboxY as i32 && ed().event.button_y() < iHiddenCheckboxY as i32 + 52 {
                        if ed().event.button_button() == BUTTON_LEFT {
                            g.g_map.objectdata[bc][br].fHidden = !g.g_map.objectdata[bc][br].fHidden;
                        }
                    }

                    if ed().iBlockType == 1 || ed().iBlockType == 15 {
                        if ed().event.button_x() >= 390 && ed().event.button_x() < 490 && ed().event.button_y() >= iHiddenCheckboxY as i32 && ed().event.button_y() < iHiddenCheckboxY as i32 + 52 {
                            if ed().event.button_button() == BUTTON_LEFT {
                                if g.g_map.objectdata[bc][br].iSettings[0] >= 0 {
                                    g.g_map.objectdata[bc][br].iSettings[0] = -1;
                                } else {
                                    g.g_map.objectdata[bc][br].iSettings[0] = default_powerup_setting(0, 0);
                                }
                            }
                        }
                    }
                } else if ty == SDL_MOUSEMOTION_EV {
                    bound_mouse_motion_coords();
                }
            }

            drawmap(g, false, TILESIZE as i16, false);
            gp.rm.menu_shade.draw(g, 0, 0);

            let mut iHiddenCheckboxY: i16 = 0;

            if ed().iBlockType == 1 || ed().iBlockType == 15 {
                let fUseGame = g.g_map.objectdata[bc][br].iSettings[0] == -1;

                for iSetting in 0..NUM_BLOCK_SETTINGS as i16 {
                    let ix: i16 = (iSetting % 6) * 100 + 35;
                    let iy: i16 = (iSetting / 6) * 62 + 65;
                    let (ix, iy) = (ix as i32, iy as i32);

                    if ed().mouse_x >= ix - 10 && ed().mouse_x < ix + 80 && ed().mouse_y >= iy - 10 && ed().mouse_y < iy + 42 && !fUseGame {
                        gp.rm.spr_powerupselector.draw_src(g, ix - 10, iy - 10, &rect(0, 0, 90, 52));
                    } else {
                        gp.rm.spr_powerupselector.draw_src(g, ix - 10, iy - 10, &rect(0, 52, 90, 52));
                    }

                    gp.rm.spr_powerups.draw_src(g, ix, iy, &rect((iSetting as i32) << 5, 0, 32, 32));

                    let szNum = if fUseGame { "X".to_string() } else { format!("{}", g.g_map.objectdata[bc][br].iSettings[iSetting as usize]) };

                    gp.rm.menu_font_large.draw_centered(g, ix + 55, iy + 5, &szNum);
                }

                iHiddenCheckboxY = 365;

                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 3, "Block Property Mode");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 2, "[~, 0-9] Set Value [LMB] Increase [RMB] Decrease [D] Default");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[Shift] + [0-9 or D] Set All To Value");
            } else if ed().iBlockType == 4 || ed().iBlockType == 5 || ed().iBlockType == 17 || ed().iBlockType == 18 || ed().iBlockType == 3 {
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Block Property Mode");
                iHiddenCheckboxY = 214;
            }

            let hy = iHiddenCheckboxY as i32;
            if ed().mouse_x >= 270 && ed().mouse_x < 370 && ed().mouse_y >= hy && ed().mouse_y < hy + 52 {
                gp.rm.spr_powerupselector.draw_src(g, 270, hy, &rect(90, 0, 100, 52));
            } else {
                gp.rm.spr_powerupselector.draw_src(g, 270, hy, &rect(90, 52, 100, 52));
            }

            gp.rm.menu_font_large.draw_centered(g, 320, hy + 3, "Hidden");

            if g.g_map.objectdata[bc][br].fHidden {
                gp.rm.spr_hidden_marker.draw(g, 310, hy + 27);
            }

            //Display "Use Game" option
            if ed().iBlockType == 1 || ed().iBlockType == 15 {
                if ed().mouse_x >= 390 && ed().mouse_x < 490 && ed().mouse_y >= hy && ed().mouse_y < hy + 52 {
                    gp.rm.spr_powerupselector.draw_src(g, 390, hy, &rect(90, 0, 100, 52));
                } else {
                    gp.rm.spr_powerupselector.draw_src(g, 390, hy, &rect(90, 52, 100, 52));
                }

                gp.rm.menu_font_large.draw_centered(g, 440, hy + 3, "Use Game");

                if g.g_map.objectdata[bc][br].iSettings[0] == -1 {
                    gp.rm.spr_hidden_marker.draw(g, 430, hy + 27);
                }
            }

            gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

            draw_message(g);
            editor_harness::frame_delay(g, 0);
            FPSLimiter::instance(g).before_flip(g);
            gfx_flipscreen(g);
            FPSLimiter::instance(g).after_flip(g);
        }
    }
}

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

pub fn editor_platforms_update_layout(g: &mut Game) {
    const padding: i32 = 32;
    const btn_size: i32 = 32;
    const btn_spacing: i32 = 10;

    const grid_area_w: i32 = UI_PLATFORM_COLS * btn_size + (UI_PLATFORM_COLS - 1) * btn_spacing;
    const grid_area_max_h: i32 = UI_PLATFORM_ROWS * btn_size + (UI_PLATFORM_ROWS - 1) * btn_spacing;

    unsafe {
        let rows = ((ed().g_iNumPlatforms as i32 + 1).min(MAX_PLATFORMS) + UI_PLATFORM_COLS - 1) / UI_PLATFORM_COLS;
        let grid_area_h = rows * btn_size + (rows - 1) * btn_spacing;
        let title_area_h = padding + g.rm.menu_font_small.get_height();
        let newbtn_area_h = padding + if (ed().g_iNumPlatforms as i32) < MAX_PLATFORMS { 24 + ed().rNewButton[0].h } else { 0 };

        ed().r.w = 2 * padding + grid_area_w;
        ed().r.h = title_area_h + grid_area_h + newbtn_area_h;
        ed().r.x = (640 - ed().r.w) / 2;
        ed().r.y = (480 - grid_area_max_h) / 2 - title_area_h;

        ed().rNewButton[1].x = ed().r.x + (ed().r.w >> 1) - (ed().rNewButton[0].w >> 1);
        ed().rNewButton[1].y = ed().r.y + ed().r.h - padding - ed().rNewButton[0].h;
    }
}

pub fn init_editor_platforms(g: &mut Game) {
    unsafe {
        if ed().editor_platforms_initialized {
            return;
        }

        ed().rNewButton[0] = rect(0, 352, 76, 32);

        ed().rNewButton[1].w = ed().rNewButton[0].w;
        ed().rNewButton[1].h = ed().rNewButton[0].h;

        editor_platforms_update_layout(g);

        for iType in 0..3usize {
            ed().rTypeButton[iType][0] = rect(0, 0, 192, 32);
            ed().rTypeButton[iType][1] = rect(320 - (ed().rTypeButton[iType][0].w >> 1), 180 + iType as i32 * 40, 192, 32);
            ed().rTypeButton[iType][2] = rect(24 * iType as i32, 32, 24, 24);
            ed().rTypeButton[iType][3] = rect(ed().rTypeButton[iType][1].x + 8, ed().rTypeButton[iType][1].y + 4, 24, 24);
        }

        ed().editor_platforms_initialized = true;
    }
}

pub fn editor_platforms_draw_background_section(g: &mut Game, src_area: &SDL_Rect, dst_area: &SDL_Rect) {
    let mut offset_y = 0;
    while offset_y < dst_area.h {
        let h = (dst_area.h - offset_y).min(src_area.h);

        let mut offset_x = 0;
        while offset_x < dst_area.w {
            let w = (dst_area.w - offset_x).min(src_area.w);

            let src = rect(src_area.x, src_area.y, w, h);
            let dst = rect(dst_area.x + offset_x, dst_area.y + offset_y, w, h);
            unsafe { ed().s_platform.draw_src_to(&src, g.screen, &dst) };

            offset_x += w;
        }

        offset_y += h;
    }
}

pub fn editor_platforms_draw_background(g: &mut Game) {
    const corner: i32 = 24;

    unsafe {
        let mut src = rect(0, 0, corner, corner);
        let mut dst = rect(ed().r.x, ed().r.y, corner, corner);

        const src_w: [i32; 3] = [corner, 256 - 2 * corner, corner];
        const src_h: [i32; 3] = [corner, 224 - 2 * corner, corner];

        let dst_w: [i32; 3] = [corner, ed().r.w - 2 * corner, corner];
        let dst_h: [i32; 3] = [corner, ed().r.h - 2 * corner, corner];

        for row in 0..3 {
            src.x = 0;
            src.h = src_h[row];

            dst.x = ed().r.x;
            dst.h = dst_h[row];

            for col in 0..3 {
                src.w = src_w[col];
                dst.w = dst_w[col];
                editor_platforms_draw_background_section(g, &src, &dst);
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
        PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == ed().iPlatformEditState || PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState
    }
}

/// The nested `while (editor_x() == EDITOR_X)` loops the platform editor runs for tiles, animation and tile types.
unsafe fn run_nested_editor(g: &mut Game, f: fn(&mut Game) -> i32, stay: i32) {
    FPSLimiter::instance(g).frame_start(g);
    while f(g) == stay {
        editor_harness::frame_delay(g, 0);
        FPSLimiter::instance(g).before_flip(g);
        gfx_flipscreen(g);
        FPSLimiter::instance(g).after_flip(g);
        FPSLimiter::instance(g).frame_start(g);
    }
}

pub fn editor_platforms(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    init_editor_platforms(g);

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            let ty = ed().event.type_();
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                let sym = ed().event.key_sym();
                let ep = ed().iEditPlatform as usize;
                if sym == k(SDLK_s) {
                    savecurrentmap(g);
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_9) {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState && sym - k(SDLK_1) < ed().g_iNumPlatforms as i32 {
                        ed().iEditPlatform = (sym - k(SDLK_1)) as i16;
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    } else if (PLATFORM_EDIT_STATE_PATH_TYPE == ed().iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == ed().iPlatformEditState)
                        && sym >= k(SDLK_1)
                        && sym <= k(SDLK_3)
                    {
                        ed().g_Platforms[ep].iPathType = path_type_from((sym - k(SDLK_1)) as u8);

                        if PLATFORM_EDIT_STATE_PATH_TYPE == ed().iPlatformEditState {
                            ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                            set_platform_to_defaults(ed().iEditPlatform);
                        } else {
                            ed().iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                        }
                    } else if PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState && sym - k(SDLK_1) < ed().g_iNumPlatforms as i32 {
                        if ed().iPlatformSwitchState == 1 {
                            switch_platforms(g, ed().iPlatformSwitchIndex, (sym - k(SDLK_1)) as i16);

                            ed().iPlatformPreview = -1;

                            ed().iPlatformSwitchState = 0;
                            ed().iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                        } else {
                            ed().iPlatformSwitchState = 1;
                            ed().iPlatformSwitchIndex = (sym - k(SDLK_1)) as i16;
                        }
                    }
                } else if sym == k(SDLK_n) {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState && (ed().g_iNumPlatforms as i32) < MAX_PLATFORMS {
                        ed().iEditPlatform = ed().g_iNumPlatforms;
                        ed().g_iNumPlatforms += 1;
                        editor_platforms_update_layout(g);
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_PATH_TYPE;
                    }
                } else if sym == k(SDLK_m) {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState {
                        ed().iPlatformPreview = -1;
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_MOVE;
                        ed().iPlatformSwitchState = 0;
                    }
                } else if sym == k(SDLK_t) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(g, editor_tiles, EDITOR_TILES);
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState {
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE;
                    }
                } else if sym == k(SDLK_a) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(g, editor_animation, EDITOR_ANIMATION);
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_ANIMATED;
                    }
                } else if sym == k(SDLK_l) {
                    if platform_edit_or_animated_or_tiletype() {
                        run_nested_editor(g, editor_tiletype, EDITOR_TILETYPE);
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_TILETYPE;
                    }
                } else if sym == k(SDLK_y) {
                    //Change the draw layer
                    if platform_edit_or_animated_or_tiletype() {
                        ed().g_Platforms[ep].iDrawLayer += 1;
                        if ed().g_Platforms[ep].iDrawLayer > 4 {
                            ed().g_Platforms[ep].iDrawLayer = 0;
                        }
                    }
                } else if sym == k(SDLK_c) {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState {
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_TEST;
                        insert_platforms_into_map(g);
                        gp.g_map.reset_platforms(g);
                    }
                } else if sym == k(SDLK_DELETE) {
                    if platform_edit_or_animated_or_tiletype() {
                        //Copy platforms into empty spot
                        let mut iPlatform = ed().iEditPlatform as usize;
                        while (iPlatform as i32) < ed().g_iNumPlatforms as i32 - 1 {
                            for iCol in 0..MW {
                                for iRow in 0..MH {
                                    ed().g_Platforms[iPlatform].tiles[iCol * MH + iRow] = ed().g_Platforms[iPlatform + 1].tiles[iCol * MH + iRow];
                                    ed().g_Platforms[iPlatform].types[iCol * MH + iRow] = ed().g_Platforms[iPlatform + 1].types[iCol * MH + iRow];
                                }
                            }

                            ed().g_Platforms[iPlatform].iPathType = ed().g_Platforms[iPlatform + 1].iPathType;

                            ed().g_Platforms[iPlatform].iVelocity = ed().g_Platforms[iPlatform + 1].iVelocity;
                            ed().g_Platforms[iPlatform].iStartX = ed().g_Platforms[iPlatform + 1].iStartX;
                            ed().g_Platforms[iPlatform].iStartY = ed().g_Platforms[iPlatform + 1].iStartY;
                            ed().g_Platforms[iPlatform].iEndX = ed().g_Platforms[iPlatform + 1].iEndX;
                            ed().g_Platforms[iPlatform].iEndY = ed().g_Platforms[iPlatform + 1].iEndY;

                            ed().g_Platforms[iPlatform].fAngle = ed().g_Platforms[iPlatform + 1].fAngle;
                            ed().g_Platforms[iPlatform].fRadiusX = ed().g_Platforms[iPlatform + 1].fRadiusX;
                            ed().g_Platforms[iPlatform].fRadiusY = ed().g_Platforms[iPlatform + 1].fRadiusY;

                            ed().g_Platforms[iPlatform].iDrawLayer = ed().g_Platforms[iPlatform + 1].iDrawLayer;
                            iPlatform += 1;
                        }

                        ed().g_iNumPlatforms -= 1;
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_ESCAPE) {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState {
                        ed().editor_platforms_initialized = false;
                        return EDITOR_EDIT;
                    } else if PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState
                        || PLATFORM_EDIT_STATE_ANIMATED == ed().iPlatformEditState
                        || PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState
                        || PLATFORM_EDIT_STATE_TEST == ed().iPlatformEditState
                    {
                        ed().iPlatformPreview = -1;
                        ed().g_Platforms[ep].update_preview(g);
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                        //Fix menu offset
                        editor_platforms_update_layout(g);
                    } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState || PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState {
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                    }
                } else if sym == k(SDLK_KP_MINUS) || sym == k(SDLK_MINUS) {
                    if platform_edit_or_animated_or_tiletype() {
                        let p = &mut ed().g_Platforms[ep];
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
                        let p = &mut ed().g_Platforms[ep];
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
                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                        calculate_platform_dims(ed().iEditPlatform, &mut ed().iPlatformLeft, &mut ed().iPlatformTop, &mut ed().iPlatformWidth, &mut ed().iPlatformHeight);
                        insert_platforms_into_map(g);
                    }
                }
            } else if ty == SDL_MOUSEBUTTONDOWN_EV {
                let ep = ed().iEditPlatform as usize;
                if ed().event.button_button() == BUTTON_LEFT && !ed().ignoreclick {
                    if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState {
                        //check clicks on existing platforms
                        for iPlatform in 0..ed().g_iNumPlatforms as i32 {
                            let ic = ed().g_Platforms[iPlatform as usize].rIcon[1];
                            if ed().event.button_x() >= ic.x && ed().event.button_x() < ic.x + ic.w && ed().event.button_y() >= ic.y && ed().event.button_y() < ic.y + ic.h {
                                if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState {
                                    ed().iEditPlatform = iPlatform as i16;
                                    ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                                    ed().ignoreclick = true;
                                } else if PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState {
                                    if ed().iPlatformSwitchState == 1 {
                                        switch_platforms(g, ed().iPlatformSwitchIndex, iPlatform as i16);

                                        ed().iPlatformPreview = -1;

                                        ed().iPlatformSwitchState = 0;
                                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_SELECT;
                                    } else {
                                        ed().iPlatformSwitchState = 1;
                                        ed().iPlatformSwitchIndex = iPlatform as i16;
                                    }
                                }
                            }
                        }

                        if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState
                            && (ed().g_iNumPlatforms as i32) < MAX_PLATFORMS
                            && ed().event.button_x() >= ed().rNewButton[1].x
                            && ed().event.button_x() < ed().rNewButton[1].x + ed().rNewButton[1].w
                            && ed().event.button_y() >= ed().rNewButton[1].y
                            && ed().event.button_y() < ed().rNewButton[1].y + ed().rNewButton[1].h
                        {
                            //Create a new platform then edit it

                            ed().iEditPlatform = ed().g_iNumPlatforms;
                            ed().g_iNumPlatforms += 1;
                            editor_platforms_update_layout(g);
                            ed().iPlatformEditState = PLATFORM_EDIT_STATE_PATH_TYPE;
                            ed().ignoreclick = true;
                        }
                    } else if PLATFORM_EDIT_STATE_PATH_TYPE == ed().iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == ed().iPlatformEditState {
                        if !ed().ignoreclick {
                            let iClickX = ed().event.button_x() as i16 as i32;
                            let iClickY = ed().event.button_y() as i16 as i32;

                            for iType in 0..3usize {
                                let tb = ed().rTypeButton[iType][1];
                                if iClickX >= tb.x && iClickX < tb.x + tb.w && iClickY >= tb.y && iClickY < tb.y + tb.h {
                                    ed().ignoreclick = true;

                                    ed().g_Platforms[ep].iPathType = path_type_from(iType as u8);

                                    if PLATFORM_EDIT_STATE_PATH_TYPE == ed().iPlatformEditState {
                                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_EDIT;
                                        set_platform_to_defaults(ed().iEditPlatform);
                                    } else {
                                        ed().iPlatformEditState = PLATFORM_EDIT_STATE_PATH;
                                    }

                                    break;
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState {
                        if !ed().ignoreclick {
                            let ix = (ed().event.button_x() / TILESIZE) as i16;
                            let iy = (ed().event.button_y() / TILESIZE) as i16;

                            for i in 0..ed().set_tile_cols as i16 {
                                for j in 0..ed().set_tile_rows as i16 {
                                    let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                    if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                        let idx = (cx * MAPHEIGHT + cy) as usize;
                                        let tile = &mut ed().g_Platforms[ep].tiles[idx];
                                        set_tileset_tile(tile, ed().set_tile_tileset as i16, (ed().set_tile_start_x + i as i32) as i16, (ed().set_tile_start_y + j as i32) as i16);
                                        let t = *tile;
                                        ed().g_Platforms[ep].types[idx] = gp.g_tilesetmanager.tileset(g, t.iID as usize).tile_type(t.iCol as usize, t.iRow as usize);
                                    }
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_ANIMATED == ed().iPlatformEditState {
                        //animated tiles
                        if !ed().ignoreclick {
                            let ix = (ed().event.button_x() / TILESIZE) as i16;
                            let iy = (ed().event.button_y() / TILESIZE) as i16;

                            for i in 0..ed().set_tile_cols as i16 {
                                for j in 0..ed().set_tile_rows as i16 {
                                    let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                    if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                        let idx = (cx * MAPHEIGHT + cy) as usize;
                                        let tile = &mut ed().g_Platforms[ep].tiles[idx];
                                        set_tileset_tile(tile, TILESETANIMATED as i16, (ed().set_tile_start_y + j as i32) as i16, (ed().set_tile_start_x + i as i32) as i16);
                                        let t = *tile;
                                        ed().g_Platforms[ep].types[idx] = ed().animatedtiletypes[(t.iRow as i32 + ((t.iCol as i32) << 5)) as usize];
                                    }
                                }
                            }
                        }
                    } else if PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState {
                        if !ed().ignoreclick {
                            let ix = ed().event.button_x() / TILESIZE;
                            let iy = ed().event.button_y() / TILESIZE;

                            ed().g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = ed().set_tiletype;
                        }
                    } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState {
                        let keystate = get_keyboard_state(g);
                        if ed().g_Platforms[ep].iPathType == PlatformPathType::Ellipse && (check_key(keystate, SDLK_z) || check_key(keystate, SDLK_x) || check_key(keystate, SDLK_c)) {
                            update_platform_path_radius(
                                ed().iEditPlatform,
                                ed().event.button_x() as i16,
                                ed().event.button_y() as i16,
                                check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT),
                                check_key(keystate, SDLK_z),
                                check_key(keystate, SDLK_c),
                            );
                        } else {
                            update_platform_path_start(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    }
                } else {
                    if ed().event.button_button() == BUTTON_RIGHT {
                        let ix = ed().event.button_x() / TILESIZE;
                        let iy = ed().event.button_y() / TILESIZE;

                        if PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == ed().iPlatformEditState {
                            clear_tileset_tile(&mut ed().g_Platforms[ep].tiles[(ix * MAPHEIGHT + iy) as usize]);
                            ed().g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = TileType::NonSolid;
                        } else if PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState {
                            ed().g_Platforms[ep].types[(ix * MAPHEIGHT + iy) as usize] = TileType::NonSolid;
                        } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState {
                            let keystate = get_keyboard_state(g);
                            if ed().g_Platforms[ep].iPathType == PlatformPathType::Straight {
                                update_platform_path_end(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                            } else if ed().g_Platforms[ep].iPathType == PlatformPathType::StraightContinuous || ed().g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                                update_platform_path_angle(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                            }
                        }
                    }
                    fall = true;
                }
            }

            if fall || ty == SDL_MOUSEMOTION_EV {
                let ep = ed().iEditPlatform as usize;
                bound_mouse_motion_coords();
                let ix = (bound_to_window_w(ed().event.motion_x()) / TILESIZE) as i16;
                let iy = (bound_to_window_h(ed().event.motion_y()) / TILESIZE) as i16;

                if PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_ANIMATED == ed().iPlatformEditState {
                    if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) && !ed().ignoreclick {
                        for i in 0..ed().set_tile_cols as i16 {
                            for j in 0..ed().set_tile_rows as i16 {
                                let (cx, cy) = (ix as i32 + i as i32, iy as i32 + j as i32);
                                if cx >= 0 && cx < MAPWIDTH && cy >= 0 && cy < MAPHEIGHT {
                                    let idx = (cx * MAPHEIGHT + cy) as usize;
                                    let tile = &mut ed().g_Platforms[ep].tiles[idx];

                                    if PLATFORM_EDIT_STATE_EDIT == ed().iPlatformEditState {
                                        set_tileset_tile(tile, ed().set_tile_tileset as i16, (ed().set_tile_start_x + i as i32) as i16, (ed().set_tile_start_y + j as i32) as i16);
                                        let t = *tile;
                                        ed().g_Platforms[ep].types[idx] = gp.g_tilesetmanager.tileset(g, t.iID as usize).tile_type(t.iCol as usize, t.iRow as usize);
                                    } else {
                                        set_tileset_tile(tile, TILESETANIMATED as i16, (ed().set_tile_start_y + j as i32) as i16, (ed().set_tile_start_x + i as i32) as i16);
                                        let t = *tile;
                                        ed().g_Platforms[ep].types[idx] = ed().animatedtiletypes[(t.iRow as i32 + ((t.iCol as i32) << 5)) as usize];
                                    }
                                }
                            }
                        }
                    } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) {
                        let idx = (ix as i32 * MAPHEIGHT + iy as i32) as usize;
                        clear_tileset_tile(&mut ed().g_Platforms[ep].tiles[idx]);
                        ed().g_Platforms[ep].types[idx] = TileType::NonSolid;
                    }
                } else if PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState {
                    let idx = (ix as i32 * MAPHEIGHT + iy as i32) as usize;
                    if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) && !ed().ignoreclick {
                        ed().g_Platforms[ep].types[idx] = ed().set_tiletype;
                    } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) {
                        ed().g_Platforms[ep].types[idx] = TileType::NonSolid;
                    }
                } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState {
                    if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) {
                        let keystate = get_keyboard_state(g);
                        if ed().g_Platforms[ep].iPathType == PlatformPathType::Ellipse && (check_key(keystate, SDLK_z) || check_key(keystate, SDLK_x) || check_key(keystate, SDLK_c)) {
                            update_platform_path_radius(
                                ed().iEditPlatform,
                                ed().event.button_x() as i16,
                                ed().event.button_y() as i16,
                                check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT),
                                check_key(keystate, SDLK_z),
                                check_key(keystate, SDLK_c),
                            );
                        } else {
                            update_platform_path_start(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) {
                        let keystate = get_keyboard_state(g);
                        if ed().g_Platforms[ep].iPathType == PlatformPathType::Straight {
                            update_platform_path_end(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        } else if ed().g_Platforms[ep].iPathType == PlatformPathType::StraightContinuous || ed().g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                            update_platform_path_angle(ed().iEditPlatform, ed().event.button_x() as i16, ed().event.button_y() as i16, check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT));
                        }
                    }
                }
                //Display platform preview
                else if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState {
                    ed().iPlatformPreview = -1;
                    for iPlatform in 0..ed().g_iNumPlatforms as i32 {
                        let ic = ed().g_Platforms[iPlatform as usize].rIcon[1];
                        if ed().event.button_x() >= ic.x && ed().event.button_x() < ic.x + ic.w && ed().event.button_y() >= ic.y && ed().event.button_y() < ic.y + ic.h {
                            ed().iPlatformPreview = iPlatform as i16;
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if ed().event.button_button() == BUTTON_LEFT {
                    ed().ignoreclick = false;
                }
            }
        }

        if PLATFORM_EDIT_STATE_TEST != ed().iPlatformEditState {
            drawmap(g, false, TILESIZE as i16, false);
            gp.rm.menu_shade.draw(g, 0, 0);
        }

        let ep = ed().iEditPlatform as usize;
        if PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState || PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState {
            editor_platforms_draw_background(g);

            gp.rm.menu_font_small.draw_centered(g, 320, ed().r.y + 18, "Platforms");

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Platform Mode: [esc] Exit  [c] Check Paths, [1-8] Select, [n] New");
            gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

            for iPlatform in 0..ed().g_iNumPlatforms as usize {
                let p = &mut ed().g_Platforms[iPlatform];
                let src = p.rIcon[0];
                ed().s_platform.draw_src_to(&src, g.screen, &p.rIcon[1]);
            }

            if (ed().g_iNumPlatforms as i32) < MAX_PLATFORMS && PLATFORM_EDIT_STATE_SELECT == ed().iPlatformEditState {
                let src = ed().rNewButton[0];
                ed().s_platform.draw_src_to(&src, g.screen, &ed().rNewButton[1]);
            }

            if PLATFORM_EDIT_STATE_MOVE == ed().iPlatformEditState {
                if ed().iPlatformSwitchState == 0 {
                    gp.rm.menu_font_small.draw_centered(g, 320, 280, "Select First");
                    gp.rm.menu_font_small.draw_centered(g, 320, 300, "Platform To Switch");
                } else {
                    gp.rm.menu_font_small.draw_centered(g, 320, 280, "Select Second");
                    gp.rm.menu_font_small.draw_centered(g, 320, 300, "Platform To Switch");
                }
            }

            if ed().iPlatformPreview >= 0 {
                display_platform_preview(g, ed().iPlatformPreview, ed().event.button_x() as i16, ed().event.button_y() as i16);
            }
        } else if PLATFORM_EDIT_STATE_PATH_TYPE == ed().iPlatformEditState || PLATFORM_EDIT_STATE_CHANGE_PATH_TYPE == ed().iPlatformEditState {
            //Draw path options
            for iType in 0..3usize {
                let s0 = ed().rTypeButton[iType][0];
                ed().s_platformpathbuttons.draw_src_to(&s0, g.screen, &ed().rTypeButton[iType][1]);
                let s2 = ed().rTypeButton[iType][2];
                ed().s_platformpathbuttons.draw_src_to(&s2, g.screen, &ed().rTypeButton[iType][3]);

                gp.rm.menu_font_large.draw(g, ed().rTypeButton[iType][1].x + 36, ed().rTypeButton[iType][1].y + 6, szPathNames[iType]);
            }

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Path Type");
        } else if platform_edit_or_animated_or_tiletype() {
            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 3, "Edit Platform");
            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 2, "[esc] Exit  [t] Tiles  [a] Animation [l] Types [del] Delete  [p] Path");
            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[+/-] Velocity  [y] Draw Layer");
            draw_platform_unfinished(g, ed().iEditPlatform, PLATFORM_EDIT_STATE_TILETYPE == ed().iPlatformEditState);

            if ed().g_Platforms[ep].iPathType == PlatformPathType::Ellipse {
                let iVelMarkerX: i16 = (198 + (ed().g_Platforms[ep].iVelocity as i32 + 10) * 12) as i16;

                let rVel = [rect(0, 400, 244, 17), rect(198, 10, 244, 17)];
                let mut d = rVel[1];
                ed().s_platform.draw_src_to(&rVel[0], g.screen, &d);

                let rMarker = [rect(244, 400, 8, 18), rect(iVelMarkerX as i32, 10, 8, 18)];
                let mut d = rMarker[1];
                ed().s_platform.draw_src_to(&rMarker[0], g.screen, &d);

                gp.rm.menu_font_small.draw_right_justified(g, 198, 10, "Counter");
                gp.rm.menu_font_small.draw(g, 442, 10, "Clockwise");
            } else {
                let iVelMarkerX: i16 = (220 + (ed().g_Platforms[ep].iVelocity as i32 - 1) * 12) as i16;

                let rVel = [rect(12, 384, 172, 13), rect(234, 10, 172, 13)];
                let mut d = rVel[1];
                ed().s_platform.draw_src_to(&rVel[0], g.screen, &d);

                let rMarker = [rect(184, 384, 8, 16), rect(iVelMarkerX as i32, 8, 8, 16)];
                let mut d = rMarker[1];
                ed().s_platform.draw_src_to(&rMarker[0], g.screen, &d);

                gp.rm.menu_font_small.draw_right_justified(g, 234, 10, "Slow");
                gp.rm.menu_font_small.draw(g, 406, 10, "Fast");
            }

            gp.rm.spr_number_icons.draw_src(g, 619, 5, &rect((ed().g_Platforms[ep].iDrawLayer as i32) << 4, 48, 16, 16));
        } else if PLATFORM_EDIT_STATE_PATH == ed().iPlatformEditState {
            let platform = &ed().g_Platforms[ep];
            let tiledata = g.g_map.platforms[ep].iTileData.clone();
            draw_platform(
                g,
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
                ed().iPlatformWidth,
                ed().iPlatformHeight,
                false,
                true,
                g.blitdest,
            );

            if platform.iPathType == PlatformPathType::Straight {
                gp.rm.menu_font_small.draw(g, 0, 480 - (gp.rm.menu_font_small.get_height() << 1), "Edit Path");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[esc] Exit  [LMB] Set Start Point  [RMB] Set End Point [t] Path Type");
            } else if platform.iPathType == PlatformPathType::StraightContinuous {
                gp.rm.menu_font_small.draw(g, 0, 480 - (gp.rm.menu_font_small.get_height() << 1), "Edit Path: [esc] Exit  [LMB] Set Start Point  [RMB] Set Angle");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[SHIFT + LMB] Location Snap [SHIFT + RMB] Angle Snap [t] Path Type");
            } else if platform.iPathType == PlatformPathType::Ellipse {
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 4, "Edit Path: [esc] Exit  [LMB] Set Center [SHIFT + LMB] Center Snap");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 3, "[X + LMB] Set X Radius [SHIFT + X + LMB] X Radius Snap");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 2, "[Z + LMB] Set Y Radius [SHIFT + Z + LMB] Y Radius Snap");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[C + LMB] Set Circular Radius [SHIFT + C + LMB] Circular Radius Snap [t] Path Type");
            }
        } else if PLATFORM_EDIT_STATE_TEST == ed().iPlatformEditState {
            gp.g_map.update_platforms(g);

            //Platforms are drawn inside drawmap(...)
            drawmap(g, false, TILESIZE as i16, true);

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Check Paths: [esc] Exit");
        }

        draw_message(g);
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

pub fn display_platform_preview(g: &mut Game, iPlatformId: i16, iMouseX: i16, iMouseY: i16) {
    unsafe {
        let srcRect = rect(0, 0, 160, 120);
        let dstRect = rect(iMouseX as i32, iMouseY as i32, 160, 120);
        ed().g_Platforms[iPlatformId as usize].preview.draw_src_to(&srcRect, g.screen, &dstRect);
    }
}

pub fn switch_platforms(g: &mut Game, iPlatformId1: i16, iPlatformId2: i16) {
    unsafe {
        let mut tempPlatform = MapPlatform::new();
        let (a, b) = (iPlatformId1 as usize, iPlatformId2 as usize);
        copy_platform(&mut tempPlatform, &ed().g_Platforms[a]);
        let from = Ptr::from_mut(&mut ed().g_Platforms[b]);
        copy_platform(&mut ed().g_Platforms[a], &from);
        copy_platform(&mut ed().g_Platforms[b], &tempPlatform);

        ed().g_Platforms[a].update_preview(g);
        ed().g_Platforms[b].update_preview(g);
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
        if !ed().ignoreclick {
            if fSnapToTile {
                //Round off coord to nearest tile
                iClickX &= !15;
                iClickY &= !15;
            }

            ed().g_Platforms[iEditPlatform_ as usize].iStartX = iClickX;
            ed().g_Platforms[iEditPlatform_ as usize].iStartY = iClickY;
        }
    }
}

pub fn update_platform_path_end(iEditPlatform_: i16, mut iClickX: i16, mut iClickY: i16, fSnapToTile: bool) {
    unsafe {
        if !ed().ignoreclick {
            if fSnapToTile {
                //Round off coord to nearest tile
                iClickX &= !15;
                iClickY &= !15;
            }

            ed().g_Platforms[iEditPlatform_ as usize].iEndX = iClickX;
            ed().g_Platforms[iEditPlatform_ as usize].iEndY = iClickY;
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
        if !ed().ignoreclick {
            let p = &mut ed().g_Platforms[iEditPlatform_ as usize];
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
        if !ed().ignoreclick {
            let p = &mut ed().g_Platforms[iEditPlatform_ as usize];
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
pub fn draw_platform_unfinished(g: &mut Game, iPlatform: i16, fDrawTileTypes: bool) {
        let mut gp = g.ptr();
    unsafe {
        let p = iPlatform as usize;
        for iCol in 0..MAPWIDTH as i16 {
            for iRow in 0..MAPHEIGHT as i16 {
                let idx = iCol as usize * MH + iRow as usize;
                let tile = ed().g_Platforms[p].tiles[idx];

                if tile.iID >= 0 {
                    gp.g_tilesetmanager.draw(g, g.screen, tile.iID, 0, tile.iCol, tile.iRow, iCol, iRow);
                } else if tile.iID as i32 == TILESETANIMATED {
                    let mut iSrcCol: i16 = ((tile.iCol as i32) << 2) as i16;
                    let mut iSrcRow: i16 = tile.iRow;

                    if iSrcCol > 31 || iSrcCol < 0 || iSrcRow > 31 || iSrcRow < 0 {
                        iSrcCol = 0;
                        iSrcRow = 0;
                    }

                    let src = g.g_tilesetmanager.rect(0, iSrcCol, iSrcRow);
                    let dst = g.g_tilesetmanager.rect(0, iCol, iRow);
                    crate::common::gfx::blit::upper_blit(g.rm.spr_tileanimation[0].get_surface(), src, g.screen, dst);
                } else if tile.iID as i32 == TILESETUNKNOWN {
                    let src = g.g_tilesetmanager.rect(0, 0, 0);
                    let dst = g.g_tilesetmanager.rect(0, iCol, iRow);
                    crate::common::gfx::blit::upper_blit(g.rm.spr_unknowntile[0].get_surface(), src, g.screen, dst);
                }

                if fDrawTileTypes {
                    let t = ed().g_Platforms[p].types[idx];
                    gp.rm.spr_transparenttiles.draw_src(g, iCol as i32 * TILESIZE, iRow as i32 * TILESIZE, &rect(prev_tile_type(t).0 as i32 * TILESIZE, 0, TILESIZE, TILESIZE));
                }
            }
        }
    }
}

const MAPHAZARD_EDIT_STATE_SELECT: i16 = 0;
const MAPHAZARD_EDIT_STATE_TYPE: i16 = 1;
const MAPHAZARD_EDIT_STATE_LOCATION: i16 = 2;
const MAPHAZARD_EDIT_STATE_PROPERTIES: i16 = 3;

const szHazardNames: [&str; 8] = ["Fireballs", "Rotodisc", "Bullet Bill", "Flame Thrower", "Green Pirhana", "Red Pirhana", "Tall Pirhana", "Short Pirhana"];

pub fn init_editor_maphazards() {
    unsafe {
        if ed().editor_maphazards_initialized {
            return;
        }

        ed().rBackground[0] = rect(0, 0, 256, 224);
        ed().rBackground[1] = rect(320 - (ed().rBackground[0].w >> 1), 240 - (ed().rBackground[0].h >> 1), 256, 224);

        ed().rNewButton[0] = rect(0, 352, 76, 32);
        ed().rNewButton[1] = rect(ed().rBackground[1].x + (ed().rBackground[1].w >> 1) - (ed().rNewButton[0].w >> 1), ed().rBackground[1].y + ed().rBackground[1].h + 8, 76, 32);

        for iType in 0..8usize {
            ed().rTypeButton[iType][0] = rect(0, 0, 192, 32);
            ed().rTypeButton[iType][1] = rect(320 - (ed().rTypeButton[iType][0].w >> 1), 84 + iType as i32 * 40, 192, 32);
            ed().rTypeButton[iType][2] = rect(24 * iType as i32, 32, 24, 24);
            ed().rTypeButton[iType][3] = rect(ed().rTypeButton[iType][1].x + 8, ed().rTypeButton[iType][1].y + 4, 24, 24);
        }

        //Setup Map Hazard Icons
        for iMapHazard in 0..MAXMAPHAZARDS {
            let i = iMapHazard as usize;
            ed().rIconRects[i][0] = rect((iMapHazard % 8) * 32, (iMapHazard / 8) * 32 + 224, 32, 32);
            ed().rIconRects[i][1] = rect((iMapHazard % 6) * 40 + 204, (iMapHazard / 6) * 40 + ed().rBackground[1].y + 16, 32, 32);
        }

        ed().editor_maphazards_initialized = true;
    }
}

pub fn editor_maphazards(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    init_editor_maphazards();

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            let ty = ed().event.type_();
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                let sym = ed().event.key_sym();
                let eh = ed().iEditMapHazard as usize;
                if sym == k(SDLK_s) {
                    savecurrentmap(g);
                } else if sym == k(SDLK_l) {
                    if MAPHAZARD_EDIT_STATE_SELECT != ed().iEditState {
                        ed().iEditState = MAPHAZARD_EDIT_STATE_LOCATION;
                    }
                } else if sym == k(SDLK_p) {
                    if MAPHAZARD_EDIT_STATE_SELECT != ed().iEditState {
                        ed().iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                    }
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_9) {
                    if MAPHAZARD_EDIT_STATE_SELECT == ed().iEditState {
                        let iHazard = (sym - k(SDLK_1)) as i16;
                        if iHazard < g.g_map.maphazards.len() as i16 {
                            ed().iEditMapHazard = iHazard;
                            ed().iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                        }
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

                        if hazard.itype == 0 || hazard.itype == 1 {
                            hazard.iparam[0] = (sym - k(SDLK_1) + 1) as i16;
                        }
                    }
                } else if sym == k(SDLK_DELETE) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState || MAPHAZARD_EDIT_STATE_LOCATION == ed().iEditState {
                        //Copy platforms into empty spot
                        let mut iMapHazard = ed().iEditMapHazard as usize;
                        while iMapHazard < g.g_map.maphazards.len().wrapping_sub(1) {
                            g.g_map.maphazards[iMapHazard].itype = g.g_map.maphazards[iMapHazard + 1].itype;
                            g.g_map.maphazards[iMapHazard].ix = g.g_map.maphazards[iMapHazard + 1].ix;
                            g.g_map.maphazards[iMapHazard].iy = g.g_map.maphazards[iMapHazard + 1].iy;

                            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                                g.g_map.maphazards[iMapHazard].iparam[iParam] = g.g_map.maphazards[iMapHazard + 1].iparam[iParam];
                                g.g_map.maphazards[iMapHazard].dparam[iParam] = g.g_map.maphazards[iMapHazard + 1].dparam[iParam];
                            }
                            iMapHazard += 1;
                        }

                        g.g_map.maphazards.pop();
                        ed().iEditState = MAPHAZARD_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_ESCAPE) {
                    if MAPHAZARD_EDIT_STATE_SELECT == ed().iEditState {
                        ed().editor_maphazards_initialized = false;
                        return EDITOR_EDIT;
                    } else if MAPHAZARD_EDIT_STATE_LOCATION == ed().iEditState || MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        ed().iEditState = MAPHAZARD_EDIT_STATE_SELECT;
                    }
                } else if sym == k(SDLK_KP_MINUS) || sym == k(SDLK_MINUS) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

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
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

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
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

                        if hazard.itype >= 2 && hazard.itype <= 7 {
                            if hazard.iparam[0] > 30 {
                                hazard.iparam[0] -= 30;
                            }
                        }
                    }
                } else if sym == k(SDLK_PERIOD) || sym == k(SDLK_RIGHTBRACKET) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

                        if hazard.itype >= 2 && hazard.itype <= 7 {
                            if hazard.iparam[0] < 480 {
                                hazard.iparam[0] += 30;
                            }
                        }
                    }
                } else if sym == k(SDLK_d) {
                    if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = &mut g.g_map.maphazards[eh];

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
                    if MAPHAZARD_EDIT_STATE_SELECT == ed().iEditState {
                        ed().iEditMapHazard = new_map_hazard(g);
                        ed().iEditState = MAPHAZARD_EDIT_STATE_TYPE;
                    }
                }
            } else if ty == SDL_MOUSEBUTTONDOWN_EV {
                if ed().event.button_button() == BUTTON_LEFT && !ed().ignoreclick {
                    let iClickX = ed().event.button_x() as i16;
                    let iClickY = ed().event.button_y() as i16;
                    let (cx, cy) = (iClickX as i32, iClickY as i32);

                    if MAPHAZARD_EDIT_STATE_SELECT == ed().iEditState {
                        //check clicks on existing platforms
                        for iMapHazard in 0..g.g_map.maphazards.len() {
                            let ic = ed().rIconRects[iMapHazard][1];
                            if cx >= ic.x && cx < ic.x + ic.w && cy >= ic.y && cy < ic.y + ic.h {
                                ed().iEditMapHazard = iMapHazard as i16;
                                ed().iEditState = MAPHAZARD_EDIT_STATE_PROPERTIES;
                                ed().ignoreclick = true;

                                break;
                            }
                        }

                        //check click on the new button
                        if (g.g_map.maphazards.len() as i32) < MAXMAPHAZARDS
                            && cx >= ed().rNewButton[1].x
                            && cx < ed().rNewButton[1].x + ed().rNewButton[1].w
                            && cy >= ed().rNewButton[1].y
                            && cy < ed().rNewButton[1].y + ed().rNewButton[1].h
                        {
                            ed().iEditMapHazard = new_map_hazard(g);
                            ed().iEditState = MAPHAZARD_EDIT_STATE_TYPE;
                            ed().ignoreclick = true;
                        }
                    } else if MAPHAZARD_EDIT_STATE_TYPE == ed().iEditState && !ed().ignoreclick {
                        for iType in 0..8usize {
                            let tb = ed().rTypeButton[iType][1];
                            if cx >= tb.x && cx < tb.x + tb.w && cy >= tb.y && cy < tb.y + tb.h {
                                let hazard = &mut g.g_map.maphazards[ed().iEditMapHazard as usize];
                                hazard.itype = iType as i16;
                                ed().iEditState = MAPHAZARD_EDIT_STATE_LOCATION;
                                ed().ignoreclick = true;

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
                    } else if MAPHAZARD_EDIT_STATE_LOCATION == ed().iEditState && !ed().ignoreclick {
                        let hazard = &mut g.g_map.maphazards[ed().iEditMapHazard as usize];
                        hazard.ix = iClickX / 16;
                        hazard.iy = iClickY / 16;
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState && !ed().ignoreclick {
                        let hazard = Ptr::from_mut(&mut g.g_map.maphazards[ed().iEditMapHazard as usize]);
                        let iType = hazard.itype;
                        if iType == 0 {
                            //Edit fireball string
                            adjust_map_hazard_radius(g, hazard.get(), iClickX, iClickY);
                        } else if iType == 1 {
                            //rotodisc
                            adjust_map_hazard_radius(g, hazard.get(), iClickX, iClickY);
                        }
                    }
                } else {
                    fall = true;
                }
            }

            if fall || ty == SDL_MOUSEMOTION_EV {
                bound_mouse_motion_coords();
                let iClickX = bound_to_window_w(ed().event.motion_x()) as i16;
                let iClickY = bound_to_window_h(ed().event.motion_y()) as i16;

                if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) && !ed().ignoreclick {
                    if MAPHAZARD_EDIT_STATE_LOCATION == ed().iEditState && !ed().ignoreclick {
                        let hazard = &mut g.g_map.maphazards[ed().iEditMapHazard as usize];
                        hazard.ix = iClickX / 16;
                        hazard.iy = iClickY / 16;
                    } else if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
                        let hazard = Ptr::from_mut(&mut g.g_map.maphazards[ed().iEditMapHazard as usize]);
                        let iType = hazard.itype;
                        if iType == 0 || iType == 1 {
                            adjust_map_hazard_radius(g, hazard.get(), iClickX, iClickY);
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if ed().event.button_button() == BUTTON_LEFT {
                    ed().ignoreclick = false;
                }
            }
        }

        //Draw platform editing
        drawmap(g, false, TILESIZE as i16, false);

        gp.rm.menu_shade.draw(g, 0, 0);

        if MAPHAZARD_EDIT_STATE_SELECT == ed().iEditState {
            let s = ed().rBackground[0];
            ed().s_platform.draw_src_to(&s, g.screen, &ed().rBackground[1]);

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Map Hazard Mode: [esc] Exit");

            for iMapHazard in 0..g.g_map.maphazards.len() {
                let s = ed().rIconRects[iMapHazard][0];
                ed().s_platform.draw_src_to(&s, g.screen, &ed().rIconRects[iMapHazard][1]);
            }

            if (g.g_map.maphazards.len() as i32) < MAXMAPHAZARDS {
                let s = ed().rNewButton[0];
                ed().s_platform.draw_src_to(&s, g.screen, &ed().rNewButton[1]);
            }

            gp.rm.menu_font_small.draw_centered(g, 320, ed().rBackground[1].y - 18, "Hazards");
        } else if MAPHAZARD_EDIT_STATE_TYPE == ed().iEditState {
            //Draw map hazard options
            for iType in 0..8usize {
                let s0 = ed().rTypeButton[iType][0];
                ed().s_maphazardbuttons.draw_src_to(&s0, g.screen, &ed().rTypeButton[iType][1]);
                let s2 = ed().rTypeButton[iType][2];
                ed().s_maphazardbuttons.draw_src_to(&s2, g.screen, &ed().rTypeButton[iType][3]);

                gp.rm.menu_font_large.draw(g, ed().rTypeButton[iType][1].x + 36, ed().rTypeButton[iType][1].y + 6, szHazardNames[iType]);
            }

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Choose Hazard Type");
        } else if MAPHAZARD_EDIT_STATE_LOCATION == ed().iEditState {
            let hazard = g.g_map.maphazards[ed().iEditMapHazard as usize];
            draw_map_hazard(g, &hazard, 0, true, g.blitdest);
            draw_map_hazard_controls(g, &hazard);

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Location: [esc] Exit, [p] Properties, [LMB] Set Location");
        } else if MAPHAZARD_EDIT_STATE_PROPERTIES == ed().iEditState {
            let hazard = g.g_map.maphazards[ed().iEditMapHazard as usize];
            draw_map_hazard(g, &hazard, 0, true, g.blitdest);
            draw_map_hazard_controls(g, &hazard);

            if hazard.itype == 0 || hazard.itype == 1 {
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 3, "Properties");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 2, "[esc] Exit, [l] Location, [+/-] Velocity, [LMB] Angle and Radius");

                if hazard.itype == 1 {
                    gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[Shift + LMB] Snap To Angle, [1-9] Number of Rotodiscs");
                } else {
                    gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[Shift + LMB] Snap To Angle");
                }
            } else if hazard.itype == 2 {
                gp.rm.menu_font_small.draw(g, 0, 480 - (gp.rm.menu_font_small.get_height() << 1), "Properties: [esc] Exit, [l] Location, [d] Direction");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[-/+] Velocity, [[/]] or [</>] Frequency");
            } else if hazard.itype >= 3 && hazard.itype <= 7 {
                gp.rm.menu_font_small.draw(g, 0, 480 - (gp.rm.menu_font_small.get_height() << 1), "Properties: [esc] Exit, [l] Location");
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[[/]] or [</>] Frequency, [d] direction");
            } else {
                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Properties: [esc] Exit, [l] Location");
            }
        }

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        draw_message(g);
        EDITOR_MAPHAZARDS
    }
}

pub fn new_map_hazard(g: &mut Game) -> i16 {
    unsafe {
        let mut hazard = MapHazard::default();
        hazard.itype = 0;
        hazard.ix = 10;
        hazard.iy = 7;

        for iMapHazard in 0..NUMMAPHAZARDPARAMS as usize {
            hazard.iparam[iMapHazard] = 0;
            hazard.dparam[iMapHazard] = 0.0;
        }

        g.g_map.maphazards.push(hazard);
        (g.g_map.maphazards.len() - 1) as i16
    }
}

pub fn draw_map_hazard_controls(g: &mut Game, hazard: &MapHazard) {
        let mut gp = g.ptr();
    unsafe {
        if hazard.itype == 0 || hazard.itype == 1 || hazard.itype == 2 {
            let iVelMarkerX: i16 = if hazard.itype == 2 {
                ((hazard.dparam[0] + 10.0f32) as i16 as i32 * 12 + 196) as i16
            } else {
                (((hazard.dparam[0] + 0.05f32) / 0.005f32) as i16 as i32 * 12 + 196) as i16
            };

            let rVel = [rect(0, 400, 244, 17), rect(198, 420, 244, 17)];
            let mut d = rVel[1];
            ed().s_platform.draw_src_to(&rVel[0], g.screen, &d);

            let rMarker = [rect(244, 400, 8, 18), rect(iVelMarkerX as i32, 418, 8, 18)];
            let mut d = rMarker[1];
            ed().s_platform.draw_src_to(&rMarker[0], g.screen, &d);

            if hazard.itype == 2 {
                gp.rm.menu_font_small.draw_right_justified(g, 190, 420, "Left");
                gp.rm.menu_font_small.draw(g, 450, 420, "Right");
            } else {
                gp.rm.menu_font_small.draw_right_justified(g, 190, 420, "Counter Clockwise");
                gp.rm.menu_font_small.draw(g, 450, 420, "Clockwise");
            }
        }

        if hazard.itype >= 2 && hazard.itype <= 7 {
            // Draw frequency for bullet bill and flame cannon
            let iFreqMarkerX: i16 = (((hazard.iparam[0] as i32 / 30) - 1) * 12 + 196) as i16;

            let rVel = [rect(0, 384, 184, 13), rect(198, 390, 184, 13)];
            let mut d = rVel[1];
            ed().s_platform.draw_src_to(&rVel[0], g.screen, &d);

            let rMarker = [rect(244, 400, 8, 18), rect(iFreqMarkerX as i32, 388, 8, 18)];
            let mut d = rMarker[1];
            ed().s_platform.draw_src_to(&rMarker[0], g.screen, &d);

            gp.rm.menu_font_small.draw_right_justified(g, 190, 390, "More Frequent");
            gp.rm.menu_font_small.draw(g, 388, 390, "Less Frequent");
        }
    }
}

pub fn adjust_map_hazard_radius(g: &mut Game, hazard: &mut MapHazard, iClickX: i16, iClickY: i16) {
    let iDiffX: i16 = (iClickX as i32 - (((hazard.ix as i32) << 4) + 16)) as i16;
    let iDiffY: i16 = (iClickY as i32 - (((hazard.iy as i32) << 4) + 16)) as i16;

    let radius: f32 = ((iDiffX as i32 * iDiffX as i32 + iDiffY as i32 * iDiffY as i32) as f64).sqrt() as f32;
    let mut angle: f32 = (iDiffY as f64).atan2(iDiffX as f64) as f32;

    if angle < 0.0f32 {
        angle += TWO_PI;
    }

    let keystate = get_keyboard_state(g);
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

pub fn init_editor_tiles(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        if ed().editor_tiles_initialized {
            return;
        }

        ed().set_tile_drag = false;
        ed().view_tileset_repeat_direction = -1;
        ed().view_tileset_repeat_timer = 0;
        ed().tileset = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);

        ed().editor_tiles_initialized = true;
    }
}

pub fn editor_tiles(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    init_editor_tiles(g);

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    if !ed().set_tile_drag {
                        let sym = ed().event.key_sym();
                        if sym >= k(SDLK_1) && sym <= k(SDLK_9) && (sym as i64) < k(SDLK_1) as i64 + g.g_tilesetmanager.count() as i64 {
                            ed().set_tile_tileset = sym - k(SDLK_1);
                            ed().tileset = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);
                            ed().view_tileset_x = 0;
                            ed().view_tileset_y = 0;
                        } else if sym == k(SDLK_PAGEUP) {
                            if ed().set_tile_tileset > 0 {
                                ed().set_tile_tileset -= 1;
                                ed().tileset = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);
                                ed().view_tileset_x = 0;
                                ed().view_tileset_y = 0;
                            }
                        } else if sym == k(SDLK_PAGEDOWN) {
                            if (ed().set_tile_tileset as i64) < g.g_tilesetmanager.count() as i64 - 1 {
                                ed().set_tile_tileset += 1;
                                ed().tileset = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);
                                ed().view_tileset_x = 0;
                                ed().view_tileset_y = 0;
                            }
                        } else if sym == k(SDLK_UP) {
                            if ed().view_tileset_y > 0 {
                                ed().view_tileset_y -= 1;
                                ed().view_tileset_repeat_direction = 0;
                                ed().view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_DOWN) {
                            if ed().view_tileset_y < gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize).height() as i32 - 15 {
                                ed().view_tileset_y += 1;
                                ed().view_tileset_repeat_direction = 1;
                                ed().view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_LEFT) {
                            if ed().view_tileset_x > 0 {
                                ed().view_tileset_x -= 1;
                                ed().view_tileset_repeat_direction = 2;
                                ed().view_tileset_repeat_timer = 30;
                            }
                        } else if sym == k(SDLK_RIGHT) {
                            if ed().view_tileset_x < gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize).width() as i32 - 20 {
                                ed().view_tileset_x += 1;
                                ed().view_tileset_repeat_direction = 3;
                                ed().view_tileset_repeat_timer = 30;
                            }
                        } else {
                            ed().edit_mode = 1; //change to edit mode using tiles
                            ed().set_tile_drag = false;
                            ed().editor_tiles_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_KEYUP_EV => {
                    let sym = ed().event.key_sym();
                    if sym == k(SDLK_UP) || sym == k(SDLK_DOWN) || sym == k(SDLK_LEFT) || sym == k(SDLK_RIGHT) {
                        ed().view_tileset_repeat_direction = -1;
                        ed().view_tileset_repeat_timer = 0;
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    let iCol = (ed().event.button_x() / TILESIZE + ed().view_tileset_x) as i16;
                    let iRow = (ed().event.button_y() / TILESIZE + ed().view_tileset_y) as i16;

                    if ed().event.button_button() == BUTTON_LEFT {
                        let tileset_ = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);

                        if iCol < tileset_.width() && iRow < tileset_.height() {
                            ed().set_tile_start_x = iCol as i32;
                            ed().set_tile_start_y = iRow as i32;
                            ed().set_tile_end_x = ed().set_tile_start_x;
                            ed().set_tile_end_y = ed().set_tile_start_y;

                            ed().set_tile_drag = true;
                        }
                    } else if ed().event.button_button() == BUTTON_RIGHT {
                        ed().set_type = ed().tileset.get().increment_tile_type(iCol as isize as usize, iRow as isize as usize);
                    } else if ed().event.button_button() == BUTTON_MIDDLE {
                        ed().set_type = ed().tileset.get().decrement_tile_type(iCol as isize as usize, iRow as isize as usize);
                    }
                }

                SDL_MOUSEBUTTONUP_EV => {
                    let iCol = (ed().event.button_x() / TILESIZE + ed().view_tileset_x) as i16;
                    let iRow = (ed().event.button_y() / TILESIZE + ed().view_tileset_y) as i16;

                    if ed().event.button_button() == BUTTON_LEFT {
                        if iCol < ed().tileset.width() && iRow < ed().tileset.height() {
                            ed().set_tile_cols = ed().set_tile_end_x - ed().set_tile_start_x + 1;
                            ed().set_tile_rows = ed().set_tile_end_y - ed().set_tile_start_y + 1;

                            ed().set_tile_drag = false;
                            ed().edit_mode = 1; //change to edit mode using tiles
                            ed().editor_tiles_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_MOUSEMOTION_EV => {
                    bound_mouse_motion_coords();
                    let iCol = (bound_to_window_w(ed().event.motion_x()) / TILESIZE + ed().view_tileset_x) as i16;
                    let iRow = (bound_to_window_h(ed().event.motion_y()) / TILESIZE + ed().view_tileset_y) as i16;

                    if iCol < ed().tileset.width() && iRow < ed().tileset.height() {
                        if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) {
                            if (iCol as i32) < ed().set_tile_start_x {
                                ed().set_tile_start_x = iCol as i32;
                            }

                            if iCol as i32 > ed().set_tile_end_x {
                                ed().set_tile_end_x = iCol as i32;
                            }

                            if (iRow as i32) < ed().set_tile_start_y {
                                ed().set_tile_start_y = iRow as i32;
                            }

                            if iRow as i32 > ed().set_tile_end_y {
                                ed().set_tile_end_y = iRow as i32;
                            }
                        } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) || ed().event.motion_state() == sdl_button(SDL_BUTTON_MIDDLE) {
                            ed().tileset.get().set_tile_type(iCol as isize as usize, iRow as isize as usize, ed().set_type);
                        }
                    }
                }

                _ => {}
            }
        }

        //Allow auto-scrolling of tilesets when the arrow keys are held down
        if ed().view_tileset_repeat_direction >= 0 && ed().view_tileset_repeat_timer > 0 {
            ed().view_tileset_repeat_timer -= 1;
            if ed().view_tileset_repeat_timer <= 0 {
                ed().view_tileset_repeat_timer = 5;

                let ts = gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize);
                if ed().view_tileset_repeat_direction == 0 && ed().view_tileset_y > 0 {
                    ed().view_tileset_y -= 1;
                } else if ed().view_tileset_repeat_direction == 1 && ed().view_tileset_y < ts.height() as i32 - 15 {
                    ed().view_tileset_y += 1;
                } else if ed().view_tileset_repeat_direction == 2 && ed().view_tileset_x > 0 {
                    ed().view_tileset_x -= 1;
                } else if ed().view_tileset_repeat_direction == 3 && ed().view_tileset_x < ts.width() as i32 - 20 {
                    ed().view_tileset_x += 1;
                }
            }
        }

        crate::common::gfx::blit::fill_rect(g.screen, null(), 0xFF888888);

        let rectSrc = rect(
            ed().view_tileset_x << 5,
            ed().view_tileset_y << 5,
            if ed().tileset.width() > 20 { 640 } else { (ed().tileset.width() as i32) << 5 },
            if ed().tileset.height() > 15 { 480 } else { (ed().tileset.height() as i32) << 5 },
        );

        ed().r = rect(0, 0, 640, 480);

        crate::common::gfx::blit::upper_blit(gp.g_tilesetmanager.tileset(g, ed().set_tile_tileset as usize).surface(0), &rectSrc, g.screen, &mut ed().r);
        gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), ed().tileset.name());

        let mut i = ed().view_tileset_x;
        while i < ed().view_tileset_x + 20 && i < ed().tileset.width() as i32 {
            let mut j = ed().view_tileset_y;
            while j < ed().view_tileset_y + 15 && j < ed().tileset.height() as i32 {
                let t = ed().tileset.tile_type(i as usize, j as usize);
                if t != TileType::NonSolid {
                    gp.rm.spr_tiletypes.draw_src(g, (i - ed().view_tileset_x) << 5, (j - ed().view_tileset_y) << 5, &rect((prev_tile_type(t).0 as i32) << 3, 0, 8, 8));
                }
                j += 1;
            }
            i += 1;
        }

        if ed().set_tile_drag {
            for i in ed().set_tile_start_x..=ed().set_tile_end_x {
                for j in ed().set_tile_start_y..=ed().set_tile_end_y {
                    gp.rm.spr_selectedtile.draw(g, (i - ed().view_tileset_x) << 5, (j - ed().view_tileset_y) << 5);
                }
            }
        }

        draw_message(g);
        EDITOR_TILES
    }
}

pub fn editor_blocks(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    ed().edit_mode = 0;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if ed().event.button_button() == BUTTON_LEFT {
                        let set_block_x = (ed().event.button_x() / TILESIZE) as i16;
                        let set_block_y = (ed().event.button_y() / TILESIZE) as i16;

                        //Set the selected block to one of the interaction blocks
                        if set_block_y == 0 && set_block_x >= 0 && set_block_x <= 6 {
                            ed().set_block = set_block_x as i32;
                        } else if set_block_y == 0 && set_block_x >= 7 && set_block_x <= 11 {
                            ed().set_block = set_block_x as i32 + 8;
                        } else if set_block_y >= 1 && set_block_y <= 2 && set_block_x >= 0 && set_block_x <= 3 {
                            //set the selected block to an on/off switch block

                            ed().set_block = set_block_x as i32 + 7;
                            g.g_map.iSwitches[set_block_x as usize] = 2 - set_block_y;
                        } else if set_block_y >= 1 && set_block_y <= 2 && set_block_x >= 4 && set_block_x <= 7 {
                            //set the selected block to a switch block

                            ed().set_block = set_block_x as i32 + 7;
                            ed().set_block_switch_on = (set_block_y == 1) as i32;
                        } else if set_block_y == 3 && set_block_x >= 0 && set_block_x <= 9 {
                            //set the selected block to a weapon breakable block

                            ed().set_block = set_block_x as i32 + 20;
                        }

                        ed().edit_mode = 0;

                        //The user must release the mouse button before trying to add a tile
                        ed().ignoreclick = true;
                        return EDITOR_EDIT;
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        g.rm.spr_blocks[0].draw_src_to(&rect(0, 0, 224, 32), g.screen, &rect(0, 0, 224, 32));
        g.rm.spr_blocks[0].draw_src_to(&rect(224, 0, 128, 64), g.screen, &rect(0, 32, 128, 64));
        g.rm.spr_blocks[0].draw_src_to(&rect(352, 0, 128, 64), g.screen, &rect(128, 32, 128, 64));
        g.rm.spr_blocks[0].draw_src_to(&rect(0, 32, 160, 32), g.screen, &rect(224, 0, 160, 32));
        g.rm.spr_blocks[0].draw_src_to(&rect(0, 64, 320, 32), g.screen, &rect(0, 96, 320, 32));

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        draw_message(g);
        EDITOR_BLOCKS
    }
}

pub fn editor_mapitems(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    ed().edit_mode = 7;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if ed().event.button_button() == BUTTON_LEFT {
                        let set_item_x = (ed().event.button_x() / TILESIZE) as i16;
                        let set_item_y = (ed().event.button_y() / TILESIZE) as i16;

                        //Set the selected block to one of the interaction blocks
                        if set_item_y == 0 && set_item_x >= 0 && set_item_x <= 5 {
                            ed().set_mapitem = set_item_x as i32;

                            ed().edit_mode = 7;

                            //The user must release the mouse button before trying to add a tile
                            ed().ignoreclick = true;
                            return EDITOR_EDIT;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        gp.rm.spr_mapitems[0].draw_src(g, 0, 0, &rect(0, 0, 192, 32));

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());
        gp.rm.menu_font_small.draw_right_justified(g, 0, 480 - g.rm.menu_font_small.get_height(), "Map Items");

        draw_message(g);
        EDITOR_MAPITEMS
    }
}

pub fn init_editor_modeitems() {
    unsafe {
        if ed().editor_modeitems_initialized {
            return;
        }

        ed().modeitemmode = 0;
        ed().dragmodeitem = -1;
        ed().dragoffsetx = 0;
        ed().dragoffsety = 0;

        ed().editor_modeitems_initialized = true;
    }
}

pub fn editor_modeitems(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    init_editor_modeitems();

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            let ty = ed().event.type_();
            let mut fall = false;

            if ty == SDL_QUIT_EV {
                return EDITOR_QUIT;
            }

            if ty == SDL_KEYDOWN_EV {
                ed().dragmodeitem = -1;

                let sym = ed().event.key_sym();
                if sym == k(SDLK_s) {
                    savecurrentmap(g);
                } else if sym == k(SDLK_ESCAPE) || sym == k(SDLK_j) {
                    ed().editor_modeitems_initialized = false;
                    return EDITOR_EDIT;
                } else if sym >= k(SDLK_1) && sym <= k(SDLK_2) {
                    ed().modeitemmode = (sym - k(SDLK_1)) as i16;
                } else if sym == k(SDLK_r) {
                    //Set this mode item set to random
                    if ed().modeitemmode == 0 {
                        if g.g_map.iNumRaceGoals == 0 {
                            g.g_map.iNumRaceGoals = MAXRACEGOALS as i16;
                        } else {
                            g.g_map.iNumRaceGoals = 0;
                        }
                    } else if ed().modeitemmode == 1 {
                        if g.g_map.iNumFlagBases == 0 {
                            g.g_map.iNumFlagBases = 4;
                        } else {
                            g.g_map.iNumFlagBases = 0;
                        }
                    }
                }
                fall = true;
            }

            if fall || ty == SDL_MOUSEBUTTONDOWN_EV {
                if ed().event.button_button() == BUTTON_LEFT {
                    let iMouseX = ed().event.button_x() as i16;
                    let iMouseY = ed().event.button_y() as i16;

                    ed().dragmodeitem = -1;
                    if ed().modeitemmode == 0 {
                        for iGoal in 0..g.g_map.iNumRaceGoals {
                            let g = g.g_map.racegoallocations[iGoal as usize];
                            if iMouseX >= g.x && (iMouseX as i32) < g.x as i32 + 36 && iMouseY >= g.y && (iMouseY as i32) < g.y as i32 + 36 {
                                ed().dragmodeitem = iGoal;
                                ed().dragoffsetx = iMouseX - g.x;
                                ed().dragoffsety = iMouseY - g.y;
                            }
                        }
                    } else if ed().modeitemmode == 1 {
                        for iBase in 0..g.g_map.iNumFlagBases {
                            let b = g.g_map.flagbaselocations[iBase as usize];
                            if iMouseX >= b.x && (iMouseX as i32) < b.x as i32 + 32 && iMouseY >= b.y && (iMouseY as i32) < b.y as i32 + 32 {
                                ed().dragmodeitem = iBase;
                                ed().dragoffsetx = iMouseX - b.x;
                                ed().dragoffsety = iMouseY - b.y;
                            }
                        }
                    }
                }
            } else if ty == SDL_MOUSEBUTTONUP_EV {
                if ed().event.button_button() == BUTTON_LEFT {
                    ed().dragmodeitem = -1;
                }
            } else if ty == SDL_MOUSEMOTION_EV {
                bound_mouse_motion_coords();
                if ed().dragmodeitem >= 0 && ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) {
                    let keystate = get_keyboard_state(g);
                    let fShiftDown = check_key(keystate, SDLK_LSHIFT) || check_key(keystate, SDLK_RSHIFT);

                    let d = ed().dragmodeitem as usize;
                    let loc = if ed().modeitemmode == 0 {
                        Some(&mut g.g_map.racegoallocations[d])
                    } else if ed().modeitemmode == 1 {
                        Some(&mut g.g_map.flagbaselocations[d])
                    } else {
                        None
                    };
                    if let Some(loc) = loc {
                        loc.x = (ed().event.motion_x() - ed().dragoffsetx as i32) as i16;
                        loc.y = (ed().event.motion_y() - ed().dragoffsety as i32) as i16;

                        if fShiftDown {
                            loc.x = ((ed().event.motion_x() >> 5) << 5) as i16;
                            loc.y = ((ed().event.motion_y() >> 5) << 5) as i16;
                        }
                    }
                }
            }
        }

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        //draw race goals
        if ed().modeitemmode == 0 {
            if g.g_map.iNumRaceGoals == 0 {
                gp.rm.menu_font_large.draw_centered(g, 320, 200, "Race goals are set to random.");
                gp.rm.menu_font_large.draw_centered(g, 320, 220, "Press 'R' to manually set them.");
            } else {
                for iGoal in 0..g.g_map.iNumRaceGoals {
                    let g = g.g_map.racegoallocations[iGoal as usize];
                    gp.rm.spr_racegoals.draw_src(gp.get(), g.x as i32 - 16, g.y as i32 - 18, &rect(0, 0, 68, 54));
                    let szNum = format!("{}", iGoal + 1);
                    gp.rm.menu_font_large.draw_centered(gp.get(), g.x as i32 + 18, g.y as i32 + 6, &szNum);
                }
            }

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Set Race Goal Locations - Press [2] for Flag Bases");
        } else if ed().modeitemmode == 1 {
            if g.g_map.iNumFlagBases == 0 {
                gp.rm.menu_font_large.draw_centered(g, 320, 200, "Flag bases are set to random.");
                gp.rm.menu_font_large.draw_centered(g, 320, 220, "Press 'R' to manually set them.");
            } else {
                for iBase in 0..g.g_map.iNumFlagBases {
                    let b = g.g_map.flagbaselocations[iBase as usize];
                    gp.rm.spr_flagbases.draw_src(g, b.x as i32 - 8, b.y as i32 - 8, &rect(iBase as i32 * 48, 0, 48, 48));
                }
            }

            gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Set Flag Base Locations - Press [1] for Race Goals");
        }

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        draw_message(g);
        EDITOR_MODEITEMS
    }
}

pub fn editor_tiletype(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    ed().edit_mode = 6;
                    return EDITOR_EDIT;
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if ed().event.button_button() == BUTTON_LEFT {
                        let iCol = (ed().event.button_x() / TILESIZE) as i16;
                        let iRow = (ed().event.button_y() / TILESIZE) as i16;

                        if (iCol as i32) < NUMTILETYPES - 1 && iRow == 0 {
                            ed().set_tiletype = TileType::from_i32(iCol as i32 + 1);

                            ed().edit_mode = 6;
                            ed().ignoreclick = true;
                            return EDITOR_EDIT;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        gp.rm.spr_transparenttiles.draw(g, 0, 0);

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        draw_message(g);
        EDITOR_TILETYPE
    }
}

pub fn init_editor_backgrounds(g: &mut Game) {
    unsafe {
        if ed().editor_backgrounds_initialized {
            return;
        }

        ed().iPage = (ed().backgroundlist.current_index() / 16) as i16;

        for iRectY in 0..4i32 {
            for iRectX in 0..4i32 {
                ed().rDst[(iRectY * 4 + iRectX) as usize] = rect(iRectX * 160, iRectY * 120, 160, 120);
            }
        }

        ed().sBackgrounds.init(std::array::from_fn(|_| gfxSprite::blank(g, 160, 120)));

        load_background_page(g, ed().iPage);

        ed().editor_backgrounds_initialized = true;
    }
}

unsafe fn free_background_surfaces() {
    for iSurface in 0..16usize {
        ed().sBackgrounds[iSurface] = gfxSprite::new();
    }
}

pub fn editor_backgrounds(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    init_editor_backgrounds(g);

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => {
                    free_background_surfaces();

                    ed().editor_backgrounds_initialized = false;
                    return EDITOR_EDIT;
                }

                SDL_KEYDOWN_EV => {
                    let sym = ed().event.key_sym();
                    if sym == k(SDLK_ESCAPE) {
                        free_background_surfaces();

                        ed().editor_backgrounds_initialized = false;
                        return EDITOR_EDIT;
                    } else if sym == k(SDLK_PAGEDOWN) || sym == k(SDLK_DOWN) {
                        if ((ed().iPage as i32 + 1) * 16) < ed().backgroundlist.count() as i32 {
                            ed().iPage += 1;
                            load_background_page(g, ed().iPage);
                        }
                    } else if sym == k(SDLK_PAGEUP) || sym == k(SDLK_UP) {
                        if (ed().iPage as i32 - 1) * 16 >= 0 {
                            ed().iPage -= 1;
                            load_background_page(g, ed().iPage);
                        }
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    if ed().event.button_button() == BUTTON_LEFT || ed().event.button_button() == BUTTON_RIGHT {
                        for iBackground in 0..16i32 {
                            if ed().iPage as i32 * 16 + iBackground >= ed().backgroundlist.count() as i32 {
                                break;
                            }

                            let d = ed().rDst[iBackground as usize];
                            if ed().event.button_x() >= d.x && ed().event.button_x() < d.x + d.w && ed().event.button_y() >= d.y && ed().event.button_y() < d.y + d.h {
                                ed().backgroundlist.set_current_index((ed().iPage as i32 * 16 + iBackground) as usize);

                                g.rm.spr_background = ImageLoader::new(ed().backgroundlist.current_path()).without_color_key().create(g);
                                g.g_map.szBackgroundFile = get_filename_from_path(&ed().backgroundlist.current_path().to_string_lossy());

                                if ed().event.button_button() == BUTTON_LEFT {
                                    //Set music to background default
                                    set_music_category_from_background(g);
                                }

                                free_background_surfaces();

                                ed().editor_backgrounds_initialized = false;
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
        crate::common::gfx::blit::fill_rect(g.screen, &rc, 0x0);

        for iBackground in 0..16i32 {
            if ed().iPage as i32 * 16 + iBackground >= ed().backgroundlist.count() as i32 {
                break;
            }

            let s = ed().rSrc;
            ed().sBackgrounds[iBackground as usize].draw_src_to(&s, g.screen, &ed().rDst[iBackground as usize]);
        }

        gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height() * 2, "[Page Up] next page, [Page Down] previous page");
        gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[LMB] choose background with music category, [RMB] choose just background");

        let iID = ed().mouse_x / 160 + ed().mouse_y / 120 * 4 + ed().iPage as i32 * 16;

        if (iID as i64) < ed().backgroundlist.count() as i64 {
            let name = ed().backgroundlist.at(iID as usize).to_string_lossy().into_owned();
            gp.rm.menu_font_small.draw(g, 0, 0, &name);
        }

        draw_message(g);
        EDITOR_BACKGROUNDS
    }
}

pub fn init_editor_animation() {
    unsafe {
        if ed().editor_animation_initialized {
            return;
        }

        ed().set_tile_drag = false;
        ed().view_tileset_repeat_direction = -1;
        ed().view_tileset_repeat_timer = 0;

        ed().editor_animation_initialized = true;
    }
}

pub fn editor_animation(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    init_editor_animation();

    unsafe {
        //handle messages
        while poll_event(&mut ed().event) != 0 {
            let iCol = (ed().event.button_x() / TILESIZE + ed().view_animated_tileset_x) as i16;
            let iRow = (ed().event.button_y() / TILESIZE) as i16;

            let fInValidTile = iRow >= 0 && iRow <= 7;

            match ed().event.type_() {
                SDL_QUIT_EV => return EDITOR_QUIT,

                SDL_KEYDOWN_EV => {
                    let sym = ed().event.key_sym();
                    if sym == k(SDLK_ESCAPE) || sym == k(SDLK_a) {
                        if !ed().set_tile_drag {
                            ed().edit_mode = 8;
                            ed().editor_animation_initialized = false;
                            return EDITOR_EDIT;
                        }
                    } else if sym == k(SDLK_LEFT) {
                        if ed().view_animated_tileset_x > 0 {
                            ed().view_animated_tileset_x -= 1;
                            ed().view_tileset_repeat_direction = 2;
                            ed().view_tileset_repeat_timer = 30;
                        }
                    } else if sym == k(SDLK_RIGHT) {
                        if ed().view_animated_tileset_x < 12 {
                            ed().view_animated_tileset_x += 1;
                            ed().view_tileset_repeat_direction = 3;
                            ed().view_tileset_repeat_timer = 30;
                        }
                    }
                }

                SDL_KEYUP_EV => {
                    let sym = ed().event.key_sym();
                    if sym == k(SDLK_LEFT) || sym == k(SDLK_RIGHT) {
                        ed().view_tileset_repeat_direction = -1;
                        ed().view_tileset_repeat_timer = 0;
                    }
                }

                SDL_MOUSEBUTTONDOWN_EV => {
                    let idx = (iCol as i32 + ((iRow as i32) << 5)) as usize;
                    if ed().event.button_button() == BUTTON_LEFT {
                        if fInValidTile {
                            ed().set_tile_start_x = iCol as i32;
                            ed().set_tile_start_y = iRow as i32;
                            ed().set_tile_end_x = ed().set_tile_start_x;
                            ed().set_tile_end_y = ed().set_tile_start_y;

                            ed().set_tile_drag = true;
                        }
                    } else if ed().event.button_button() == BUTTON_RIGHT {
                        ed().animatedtiletypes[idx] = next_tile_type(ed().animatedtiletypes[idx]);
                        ed().set_type = ed().animatedtiletypes[idx];
                    } else if ed().event.button_button() == BUTTON_MIDDLE {
                        ed().animatedtiletypes[idx] = prev_tile_type(ed().animatedtiletypes[idx]);
                        ed().set_type = ed().animatedtiletypes[idx];
                    }
                }

                SDL_MOUSEBUTTONUP_EV => {
                    if ed().event.button_button() == BUTTON_LEFT {
                        if fInValidTile {
                            ed().set_tile_cols = ed().set_tile_end_x - ed().set_tile_start_x + 1;
                            ed().set_tile_rows = ed().set_tile_end_y - ed().set_tile_start_y + 1;

                            ed().set_tile_drag = false;
                            ed().edit_mode = 8; //change to edit mode using tiles
                            ed().editor_animation_initialized = false;
                            return EDITOR_EDIT;
                        }
                    }
                }

                SDL_MOUSEMOTION_EV => {
                    bound_mouse_motion_coords();
                    if fInValidTile {
                        if ed().event.motion_state() == sdl_button(SDL_BUTTON_LEFT) {
                            if (iCol as i32) < ed().set_tile_start_x {
                                ed().set_tile_start_x = iCol as i32;
                            }

                            if iCol as i32 > ed().set_tile_end_x {
                                ed().set_tile_end_x = iCol as i32;
                            }

                            if (iRow as i32) < ed().set_tile_start_y {
                                ed().set_tile_start_y = iRow as i32;
                            }

                            if iRow as i32 > ed().set_tile_end_y {
                                ed().set_tile_end_y = iRow as i32;
                            }
                        } else if ed().event.motion_state() == sdl_button(SDL_BUTTON_RIGHT) || ed().event.motion_state() == sdl_button(SDL_BUTTON_MIDDLE) {
                            ed().animatedtiletypes[(iCol as i32 + ((iRow as i32) << 5)) as usize] = ed().set_type;
                        }
                    }
                }

                _ => {}
            }
        }

        //Allow auto-scrolling of tilesets when the arrow keys are held down
        if ed().view_tileset_repeat_direction >= 0 && ed().view_tileset_repeat_timer > 0 {
            ed().view_tileset_repeat_timer -= 1;
            if ed().view_tileset_repeat_timer <= 0 {
                ed().view_tileset_repeat_timer = 5;

                if ed().view_tileset_repeat_direction == 2 && ed().view_animated_tileset_x > 0 {
                    ed().view_animated_tileset_x -= 1;
                } else if ed().view_tileset_repeat_direction == 3 && ed().view_animated_tileset_x < 12 {
                    ed().view_animated_tileset_x += 1;
                }
            }
        }

        crate::common::gfx::blit::fill_rect(g.screen, null(), 0xFF888888);

        for iCol in ed().view_animated_tileset_x as i16..(ed().view_animated_tileset_x + 20) as i16 {
            for iRow in 0..8i16 {
                let iDestX: i16 = ((iCol as i32 - ed().view_animated_tileset_x) << 5) as i16;
                let iDestY: i16 = iRow << 5;
                let iSrcX: i16 = iRow << 7;
                let iSrcY: i16 = iCol << 5;

                gp.rm.spr_tileanimation[0].draw_src(g, iDestX as i32, iDestY as i32, &rect(iSrcX as i32, iSrcY as i32, TILESIZE, TILESIZE));

                let t = ed().animatedtiletypes[(iCol as i32 + ((iRow as i32) << 5)) as usize];
                if t != TileType::NonSolid {
                    gp.rm.spr_tiletypes.draw_src(g, iDestX as i32, iDestY as i32, &rect((prev_tile_type(t).0 as i32) << 3, 0, 8, 8));
                }
            }
        }

        if ed().set_tile_drag {
            for i in (ed().set_tile_start_x - ed().view_animated_tileset_x) as i16..=(ed().set_tile_end_x - ed().view_animated_tileset_x) as i16 {
                for j in ed().set_tile_start_y as i16..=ed().set_tile_end_y as i16 {
                    gp.rm.spr_selectedtile.draw(g, (i as i32) << 5, (j as i32) << 5);
                }
            }
        }

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());

        gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "Use Arrow Keys To Scroll");

        draw_message(g);
        EDITOR_ANIMATION
    }
}

pub fn load_background_page(g: &mut Game, iPage_: i16) {
    let mut gp = g.ptr();
    unsafe {
        let srcRectBackground = rect(0, 0, 640, 480);
        let dstRectBackground = rect(0, 0, 160, 120);

        for iIndex in 0..16i32 {
            if iPage_ as i32 * 16 + iIndex >= ed().backgroundlist.count() as i32 {
                break;
            }

            let path = ed().backgroundlist.at((iPage_ as i32 * 16 + iIndex) as usize);
            if path.as_os_str().is_empty() {
                return;
            }

            let temp = ImageLoader::new(path).without_color_key().create(gp.get());

            crate::common::gfx::blit::fill_rect(ed().sBackgrounds[iIndex as usize].get_surface(), null(), 0x0);

            if temp.get_width() != 640 || temp.get_height() != 480 {
                println!("WARNING: Background {} is {}x{} but must be 640x480. Skipping.", path.display(), temp.get_width(), temp.get_height());
                continue;
            }

            temp.draw_stretch(&srcRectBackground, ed().sBackgrounds[iIndex as usize].get_surface(), &dstRectBackground);
        }
    }
}

pub fn display_help(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        drawmap(g, false, TILESIZE as i16, false);
        gp.rm.menu_shade.draw(g, 0, 0);

        let fh = g.rm.menu_font_small.get_height();
        let mut offsety = 10;
        let mut offsetx = 20;
        let mut line = |text: &str, gap: i32, offsety: &mut i32, offsetx: i32| {
            gp.rm.menu_font_small.draw(g, offsetx, *offsety, text);
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
        gp.rm.menu_font_small.draw(g, offsetx, offsety, "[alt] + [enter] - Full Screen/Window");

        //handle messages
        while poll_event(&mut ed().event) != 0 {
            match ed().event.type_() {
                SDL_QUIT_EV => return 0,
                SDL_KEYDOWN_EV => return 0,
                _ => {}
            }
        }

        DISPLAY_HELP
    }
}

pub fn save_as(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    let mut fileName = String::new();
    let mut mapLocation = String::from("maps/");

    if dialog(g, "Save As", "Enter name:", &mut fileName, 64) {
        mapLocation.push_str(&fileName);
        mapLocation.push_str(".map");
        save_map(g, &convert_path(gp.get(), &mapLocation));
        fileName.push_str(".map");
        unsafe {
            gp.maplist.add(g, &fileName);
            g.maplist.find(&fileName);
        }
        loadcurrentmap(g);
    }

    0
}

unsafe fn draw_dialog(g: &mut Game, title: &str, instructions: &str, input: Option<&str>) {
        let mut gp = g.ptr();
    drawmap(g, false, TILESIZE as i16, false);
    gp.rm.menu_shade.draw(g, 0, 0);
    gp.rm.spr_dialog.draw_src(g, 224, 176, &rect(0, 0, 192, 128));
    gp.rm.menu_font_large.draw_centered(g, 320, 200, title);
    gp.rm.menu_font_small.draw(g, 240, 235, instructions);
    if let Some(input) = input {
        gp.rm.menu_font_small.draw(g, 240, 255, input);
    }
    gp.rm.menu_font_small.draw_right_justified(g, 640, 0, gp.maplist.current_filename());
    gfx_flipscreen(g);
}

pub fn dialog(g: &mut Game, title: &str, instructions: &str, input: &mut String, inputsize: i32) -> bool {
    let mut gp = g.ptr();
    unsafe {
        let mut currentChar: u32 = 0;

        draw_dialog(g, title, instructions, None);

        loop {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(&mut ed().event) != 0 {
                match ed().event.type_() {
                    SDL_QUIT_EV => return false,

                    SDL_KEYDOWN_EV => {
                        let sym = ed().event.key_sym();
                        if sym == k(SDLK_KP_ENTER) || sym == k(SDLK_RETURN) {
                            return true;
                        } else if sym == k(SDLK_ESCAPE) {
                            return false;
                        } else if sym == k(SDLK_BACKSPACE) {
                            if currentChar > 0 {
                                input.truncate((currentChar - 1) as usize);

                                draw_dialog(g, title, instructions, Some(input));

                                currentChar -= 1;
                            }
                        } else if ((48..=57).contains(&sym) || sym == 45 || sym == 32 || sym == 61 || (95..=122).contains(&sym)) && currentChar < (inputsize as u32) - 1 {
                            //insert character into fileName and onScreenText and increment current char
                            let mut key: u8 = sym as u8;

                            let keystate = get_keyboard_state(g);
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

                            draw_dialog(g, title, instructions, Some(input));
                        }
                    }

                    _ => {}
                }
            }

            let mut delay = WAITTIME - (crate::services::ticks(gp.get()) as i32 - framestart);
            if delay < 0 {
                delay = 0;
            } else if delay > WAITTIME {
                delay = WAITTIME;
            }

            editor_harness::frame_delay(g, delay as u32);
        }
    }
}

pub fn find(g: &mut Game) -> i32 {
    let mut fileName = String::new();

    if dialog(g, "Find Map", "Enter name:", &mut fileName, 64) {
        unsafe {
            ed().findstring = fileName.clone();

            let fs = ed().findstring.clone();
            if g.maplist.find(&fs) {
                loadcurrentmap(g);
            }
        }
    }

    0
}

pub fn clear_map(g: &mut Game) -> i32 {
    unsafe {
        g.g_map.clear_map();
        ed().g_iNumPlatforms = 0;
    }

    println!("Map Cleared");
    0
}

pub fn loadcurrentmap(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        let filename = g.maplist.current_filename().to_string();
        gp.g_map.load_map(g, &filename, read_type_full);

        if g.g_map.iNumRaceGoals == 0 {
            for iGoal in 0..MAXRACEGOALS as usize {
                g.g_map.racegoallocations[iGoal].x = (iGoal as i32 * 80 + 20) as i16;
                g.g_map.racegoallocations[iGoal].y = 18;
            }
        }

        if g.g_map.iNumFlagBases == 0 {
            for iBase in 0..4usize {
                g.g_map.flagbaselocations[iBase].x = (iBase as i32 * 80 + 20) as i16;
                g.g_map.flagbaselocations[iBase].y = 18;
            }
        }

        let filename = concat("gfx/packs/Classic/backgrounds/", &g.g_map.szBackgroundFile);
        let mut path = convert_path(g, &filename);
        ed().backgroundlist.set_current_path(Path::new(&filename));

        if !file_exists(&path) {
            path = convert_path(g, "gfx/packs/Classic/backgrounds/Land_Classic.png");
            ed().backgroundlist.set_current_path(Path::new("gfx/packs/Classic/backgrounds/Land_Classic.png"));
        }

        g.rm.spr_background = ImageLoader::new(&path).without_color_key().create(g);

        ed().g_iNumPlatforms = g.g_map.platforms.len() as i16;

        for iPlatform in 0..ed().g_iNumPlatforms as usize {
            let mp = g.g_map.platforms[iPlatform];
            for iCol in 0..MW {
                for iRow in 0..MH {
                    if (iCol as i16) < mp.iTileWidth && (iRow as i16) < mp.iTileHeight {
                        ed().g_Platforms[iPlatform].tiles[iCol * MH + iRow] = *mp.tile_at(iCol, iRow);
                        ed().g_Platforms[iPlatform].types[iCol * MH + iRow] = mp.tile_type_at(iCol, iRow);
                    } else {
                        clear_tileset_tile(&mut ed().g_Platforms[iPlatform].tiles[iCol * MH + iRow]);
                        ed().g_Platforms[iPlatform].types[iCol * MH + iRow] = TileType::NonSolid;
                    }
                }
            }

            ed().g_Platforms[iPlatform].iDrawLayer = mp.iDrawLayer;
            ed().g_Platforms[iPlatform].iPathType = mp.pPath.path_type_id();

            let mut mpm = mp;
            let any = mpm.pPath.as_any();
            if let Some(path) = any.downcast_ref::<StraightPath>() {
                ed().g_Platforms[iPlatform].iVelocity = (path.path().speed() * 4.0f32) as i32 as i16;
                ed().g_Platforms[iPlatform].iStartX = path.start_pos().x as i32 as i16;
                ed().g_Platforms[iPlatform].iStartY = path.start_pos().y as i32 as i16;
                ed().g_Platforms[iPlatform].iEndX = path.end_pos().x as i32 as i16;
                ed().g_Platforms[iPlatform].iEndY = path.end_pos().y as i32 as i16;
            } else if let Some(path) = any.downcast_ref::<StraightPathContinuous>() {
                ed().g_Platforms[iPlatform].iVelocity = (path.path().speed() * 4.0f32) as i32 as i16;
                ed().g_Platforms[iPlatform].iStartX = path.start_pos().x as i32 as i16;
                ed().g_Platforms[iPlatform].iStartY = path.start_pos().y as i32 as i16;
                ed().g_Platforms[iPlatform].fAngle = path.angle();
            } else if let Some(path) = any.downcast_ref::<EllipsePath>() {
                ed().g_Platforms[iPlatform].iVelocity = (path.path().speed() / 0.0030f32) as i16;
                ed().g_Platforms[iPlatform].fRadiusX = path.radius().x;
                ed().g_Platforms[iPlatform].fRadiusY = path.radius().y;
                ed().g_Platforms[iPlatform].iStartX = path.center_pos().x as i32 as i16;
                ed().g_Platforms[iPlatform].iStartY = path.center_pos().y as i32 as i16;
                ed().g_Platforms[iPlatform].fAngle = path.start_angle();
            }

            ed().g_Platforms[iPlatform].update_preview(g);
        }
    }
}

pub fn set_platform_to_defaults(iPlatform: i16) {
    unsafe {
        let p = &mut ed().g_Platforms[iPlatform as usize];
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

pub fn savecurrentmap(g: &mut Game) -> i32 {
    unsafe {
        ed().g_messagedisplaytimer = 60;
        ed().g_szMessageTitle = "Saved".to_string();
        ed().g_szMessageLine[0] = "Your map has".to_string();
        ed().g_szMessageLine[1] = "been saved.".to_string();
        ed().g_szMessageLine[2] = String::new();

        let file = g.maplist.current_filename().to_string();
        save_map(g, &file);
    }
    0
}

pub fn insert_platforms_into_map(g: &mut Game) {
        let mut gp = g.ptr();
    unsafe {
        //First take the created platforms and move them into the actual map
        g.g_map.clear_platforms();

        g.g_map.platforms.reserve(ed().g_iNumPlatforms as usize);

        for iPlatform in 0..ed().g_iNumPlatforms {
            let (mut iTop, mut iLeft, mut iWidth, mut iHeight) = (0i16, 0i16, 0i16, 0i16);
            calculate_platform_dims(iPlatform, &mut iLeft, &mut iTop, &mut iWidth, &mut iHeight);

            let n = (iWidth as i32 * iHeight as i32) as usize;
            let mut tiles: Vec<TilesetTile> = vec![TilesetTile::default(); n];
            let mut types: Vec<TileType> = vec![TileType::NonSolid; n];

            let p = &ed().g_Platforms[iPlatform as usize];
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

            gp.g_map.add_permanent_platform(MovingPlatform::new(g, tiles, types, iWidth, iHeight, iDrawLayer, path, false));
        }
    }
}

pub fn save_map(g: &mut Game, file: &str) {
        let mut gp = g.ptr();
    insert_platforms_into_map(g);

    //Then save the rest of the map
    unsafe { gp.g_map.save_map(g, file) };
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
                if ed().g_Platforms[iPlatform as usize].tiles[iCol as usize * MH + iRow as usize].iID as i32 != TILESETNONE {
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

pub fn findcurrentstring(g: &mut Game) -> i32 {
    unsafe {
        if !ed().findstring.is_empty() {
            let fs = ed().findstring.clone();
            if g.maplist.find(&fs) {
                loadcurrentmap(g);
            }
        }
    }

    0
}

pub fn newmap(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    let mut fileName = String::new();
    let mut mapLocation = String::from("maps/");

    if dialog(g, "New Map", "Enter name:", &mut fileName, 64) {
        unsafe {
            g.g_map.clear_map();
            g.g_map.clear_platforms();
            mapLocation.push_str(&fileName);
            mapLocation.push_str(".map");
            gp.get().g_map.save_map(g, &convert_path(gp.get(), &mapLocation));
            fileName.push_str(".map");
            gp.maplist.add(g, &fileName);
            g.maplist.find(&fileName);
        }
        loadcurrentmap(g);
    }

    0
}

pub fn resetselectedtiles() {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                ed().selectedtiles[j][kk] = false;
            }
        }
    }
}

pub fn copymoveselection() {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                ed().moveselectedtiles[j][kk] = ed().selectedtiles[j][kk];
            }
        }

        ed().copiedlayer = ed().selected_layer;
    }
}

pub fn pastemoveselection(movex: i32, movey: i32) {
    unsafe {
        for kk in 0..MAPHEIGHT {
            for j in 0..MAPWIDTH {
                if ed().moveselectedtiles[j as usize][kk as usize] {
                    if j + movex >= 0 && j + movex < MAPWIDTH && kk + movey >= 0 && kk + movey < MAPHEIGHT {
                        ed().selectedtiles[(j + movex) as usize][(kk + movey) as usize] = ed().moveselectedtiles[j as usize][kk as usize];
                    }
                }
            }
        }
    }
}

pub fn copyselectedtiles(g: &mut Game) -> bool {
    unsafe {
        //Copy the selected tiles and remove tiles from map
        let mut ret = false;
        for kk in 0..MH {
            for j in 0..MW {
                if ed().selectedtiles[j][kk] {
                    ret = true;
                    let c = &mut ed().copiedtiles[j][kk];
                    for iLayer in 0..MAPLAYERS as usize {
                        c.tile[iLayer] = g.g_map.mapdata[j][kk][iLayer];
                    }

                    c.block.iType = g.g_map.objectdata[j][kk].iType;
                    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                        c.block.iSettings[iSetting] = g.g_map.objectdata[j][kk].iSettings[iSetting];
                    }
                    c.block.fHidden = g.g_map.objectdata[j][kk].fHidden;

                    c.warp.connection = g.g_map.warpdata[j][kk].connection;
                    c.warp.direction = g.g_map.warpdata[j][kk].direction;
                    c.warp.id = g.g_map.warpdata[j][kk].id;

                    c.tiletype = g.g_map.mapdatatop[j][kk];

                    for iType in 0..NUMSPAWNAREATYPES as usize {
                        c.nospawn[iType] = g.g_map.nospawn[iType][j][kk];
                    }

                    c.item = -1;
                    for item in g.g_map.mapitems.iter() {
                        if item.ix as usize == j && item.iy as usize == kk {
                            c.item = item.itype as i32;
                            break;
                        }
                    }
                }
            }
        }

        ed().copiedlayer = ed().selected_layer;

        ret
    }
}

pub fn clearselectedmaptiles(g: &mut Game) {
    unsafe {
        for kk in 0..MH {
            for j in 0..MW {
                if ed().selectedtiles[j][kk] {
                    if ed().view_only_layer {
                        g.g_map.mapdata[j][kk][ed().selected_layer as usize].iID = TILESETNONE as i16;
                    } else {
                        for iLayer in 0..MAPLAYERS as usize {
                            g.g_map.mapdata[j][kk][iLayer].iID = TILESETNONE as i16;
                        }

                        g.g_map.objectdata[j][kk].iType = -1;

                        g.g_map.warpdata[j][kk].connection = -1;
                        g.g_map.warpdata[j][kk].direction = WARP_UNDEFINED;
                        g.g_map.warpdata[j][kk].id = -1;

                        for iType in 0..NUMSPAWNAREATYPES as usize {
                            g.g_map.nospawn[iType][j][kk] = false;
                        }

                        remove_map_item_at(g, j as i16, kk as i16);
                    }

                    update_tile_type(g, j as i16, kk as i16);
                }
            }
        }
    }
}

/// `replacetile(short*, short, bool)`
pub fn replacetile_short(iDstTile: &mut i16, iSrcTile: i16, fAllowReplace: bool) {
    unsafe {
        if ed().move_replace || fAllowReplace {
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
        if ed().move_replace {
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
        if ed().move_replace {
            copymapblock(dstTile, srcTile);
        } else if srcTile.iType != -1 {
            copymapblock(dstTile, srcTile);
        }
    }
}

pub fn pasteselectedtiles(g: &mut Game, movex: i32, movey: i32) {
    unsafe {
        //Paste the tiles into their new location
        //Removing tiles that hang over the edges of the map
        for kk in 0..MAPHEIGHT {
            for j in 0..MAPWIDTH {
                if ed().selectedtiles[j as usize][kk as usize] {
                    if j + movex >= 0 && j + movex < MAPWIDTH && kk + movey >= 0 && kk + movey < MAPHEIGHT {
                        let iNewX = (j + movex) as usize;
                        let iNewY = (kk + movey) as usize;
                        let c = ed().copiedtiles[j as usize][kk as usize];

                        if ed().view_only_layer {
                            replacetile_tile(&mut g.g_map.mapdata[iNewX][iNewY][ed().selected_layer as usize], &c.tile[ed().copiedlayer as usize]);
                        } else {
                            for iLayer in 0..MAPLAYERS as usize {
                                replacetile_tile(&mut g.g_map.mapdata[iNewX][iNewY][iLayer], &c.tile[iLayer]);
                            }

                            replacetile_block(&mut g.g_map.objectdata[iNewX][iNewY], &c.block);

                            replacetile_short(&mut g.g_map.warpdata[iNewX][iNewY].connection, c.warp.connection, c.warp.connection != -1);
                            // The C++ writes the direction through a `short*`; WarpEnterDirection is an int-sized enum.
                            let mut dir = g.g_map.warpdata[iNewX][iNewY].direction as i16;
                            replacetile_short(&mut dir, c.warp.direction as i16, c.warp.connection != -1);
                            g.g_map.warpdata[iNewX][iNewY].direction = (g.g_map.warpdata[iNewX][iNewY].direction & !0xFFFF) | (dir as u16 as i32);
                            replacetile_short(&mut g.g_map.warpdata[iNewX][iNewY].id, c.warp.id, c.warp.connection != -1);

                            if ed().move_replace {
                                g.g_map.mapdatatop[iNewX][iNewY] = c.tiletype;
                            } else {
                                update_tile_type(g, j as i16, kk as i16);
                            }

                            for iType in 0..NUMSPAWNAREATYPES as usize {
                                g.g_map.nospawn[iType][iNewX][iNewY] = c.nospawn[iType];
                            }

                            if (g.g_map.mapitems.len() as i32) < MAXMAPITEMS && c.item >= 0 {
                                let item = MapItem { itype: c.item as MapItemType, ix: iNewX as i16, iy: iNewY as i16 };
                                g.g_map.mapitems.push(item);
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
                if ed().selectedtiles[j as usize][kk as usize] {
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
pub fn takescreenshot(g: &mut Game) {
    unsafe {
        let iTileSizes: [i16; 3] = [TILESIZE as i16, PREVIEWTILESIZE as i16, THUMBTILESIZE as i16];
        let old_screen = g.screen;

        for iScreenshotSize in 0..3i16 {
            let iTileSize = iTileSizes[iScreenshotSize as usize];

            //Allow wrapping of path dots
            g.rm.spr_platformpath.set_wrap((640 >> iScreenshotSize) as i16);

            //Create new screenshot surface
            let screenshot = gfxSprite::blank(g, iTileSize as u32 * 20, iTileSize as u32 * 15);
            g.blitdest = screenshot.get_surface();
            g.screen = screenshot.get_surface();

            //Draw map to screenshot
            drawmap(g, true, iTileSize, false);

            //Draw platforms to screenshot
            for iPlatform in 0..ed().g_iNumPlatforms as usize {
                let platform = &ed().g_Platforms[iPlatform];
                draw_platform(
                    g,
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
                    g.g_map.platforms[iPlatform].iTileWidth,
                    g.g_map.platforms[iPlatform].iTileHeight,
                    true,
                    true,
                    g.blitdest,
                );
            }

            //Draw map hazards
            for hazard in g.g_map.maphazards.clone().iter() {
                draw_map_hazard(g, hazard, iScreenshotSize, false, g.blitdest);
            }

            //Save the screenshot with the same name as the map file
            let mut szSaveFile = String::from("maps/screenshots/");
            szSaveFile.push_str(&get_name_from_file_name(g.maplist.current_filename(), false));

            if iTileSize as i32 == PREVIEWTILESIZE {
                szSaveFile.push_str("_preview");
            } else if iTileSize as i32 == THUMBTILESIZE {
                szSaveFile.push_str("_thumb");
            }

            szSaveFile.push_str(".png");
            let c = CString::new(convert_path(g, &szSaveFile)).unwrap();
            IMG_SavePNG(screenshot.get_surface(), c.as_ptr());

            println!("Screenshot taken: {}", szSaveFile);

            g.screen = old_screen;
            g.blitdest = g.screen;
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

            ed().animatedtiletypes = vec![TileType::NonSolid; 256];

            for i in 0..256usize {
                ed().animatedtiletypes[i] = TileType::from_i32(tsf.read_i32());
            }
        } else {
            ed().animatedtiletypes = vec![TileType::NonSolid; 256];
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
            tsf.write_i32(ed().animatedtiletypes[i].0 as i32);
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

/// Replay-harness `E`/`T`/`X`/`P`/`H` records (docs/EDITOR_REPLAY.md).
pub fn dump_editor_state(g: &mut Game, out: &mut dyn Write) {
    let mut gp = g.ptr();
    unsafe {
        let _ = writeln!(
            out,
            "E state={} edit_mode={} layer={} mouse={},{} ignore={} only={} blocks={}",
            ed().state, ed().edit_mode, ed().selected_layer, ed().mouse_x, ed().mouse_y, ed().ignoreclick as i32, ed().view_only_layer as i32, ed().viewblocks as i32
        );
        let _ = writeln!(
            out,
            "T tileset={} start={},{} end={},{} size={},{} drag={} view={},{},{} block={},{} tiletype={} settype={} item={} warp={},{} nospawn={}",
            ed().set_tile_tileset,
            ed().set_tile_start_x,
            ed().set_tile_start_y,
            ed().set_tile_end_x,
            ed().set_tile_end_y,
            ed().set_tile_cols,
            ed().set_tile_rows,
            ed().set_tile_drag as i32,
            ed().view_tileset_x,
            ed().view_tileset_y,
            ed().view_animated_tileset_x,
            ed().set_block,
            ed().set_block_switch_on,
            ed().set_tiletype.0 as i32,
            ed().set_type.0 as i32,
            ed().set_mapitem,
            ed().set_direction,
            ed().set_connection,
            ed().nospawn_mode
        );
        let _ = writeln!(
            out,
            "X move={} start={},{} offset={},{} drag={},{},{},{} replace={} nodrag={} copied={}",
            ed().move_mode,
            ed().move_start_x,
            ed().move_start_y,
            ed().move_offset_x,
            ed().move_offset_y,
            ed().move_drag_start_x,
            ed().move_drag_start_y,
            ed().move_drag_offset_x,
            ed().move_drag_offset_y,
            ed().move_replace as i32,
            ed().move_nodrag as i32,
            ed().copiedlayer
        );
        let _ = writeln!(
            out,
            "P count={} edit={} state={} preview={} switch={},{} hazards={} hzstate={} hzedit={} modeitem={},{} msg={} music={}",
            ed().g_iNumPlatforms,
            ed().iEditPlatform,
            ed().iPlatformEditState,
            ed().iPlatformPreview,
            ed().iPlatformSwitchState,
            ed().iPlatformSwitchIndex,
            g.g_map.maphazards.len(),
            ed().iEditState,
            ed().iEditMapHazard,
            ed().modeitemmode,
            ed().dragmodeitem,
            ed().g_messagedisplaytimer,
            ed().g_musiccategorydisplaytimer
        );

        let mut hm = Fnv::new();
        for x in 0..MW {
            for y in 0..MH {
                for l in 0..MAPLAYERS as usize {
                    let t = g.g_map.mapdata[x][y][l];
                    hm.add(t.iID as i32);
                    hm.add(t.iCol as i32);
                    hm.add(t.iRow as i32);
                }
                hm.add(g.g_map.mapdatatop[x][y].0 as i32);
                let b = &g.g_map.objectdata[x][y];
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
                let w = g.g_map.warpdata[x][y];
                hm.add(w.direction as i32);
                hm.add(w.connection as i32);
                hm.add(w.id as i32);
                for t in 0..NUMSPAWNAREATYPES as usize {
                    hm.add(g.g_map.nospawn[t][x][y] as i32);
                }
            }
        }
        hm.add(g.g_map.mapitems.len() as i32);
        for item in g.g_map.mapitems.iter() {
            hm.add(item.itype as i32);
            hm.add(item.ix as i32);
            hm.add(item.iy as i32);
        }
        hm.add(g.g_map.maphazards.len() as i32);
        for hz in g.g_map.maphazards.iter() {
            hm.add(hz.itype as i32);
            hm.add(hz.ix as i32);
            hm.add(hz.iy as i32);
            for p in 0..NUMMAPHAZARDPARAMS as usize {
                hm.add(hz.iparam[p] as i32);
                hm.addf(hz.dparam[p]);
            }
        }
        for l in 0..3usize {
            hm.add(g.g_map.eyecandy[l] as i32);
        }
        hm.add(g.g_map.musicCategoryID as i32);
        hm.add(g.g_map.iNumRaceGoals as i32);
        for g in g.g_map.racegoallocations.iter() {
            hm.add(g.x as i32);
            hm.add(g.y as i32);
        }
        hm.add(g.g_map.iNumFlagBases as i32);
        for b in g.g_map.flagbaselocations.iter() {
            hm.add(b.x as i32);
            hm.add(b.y as i32);
        }
        for s in g.g_map.iSwitches.iter() {
            hm.add(*s as i32);
        }
        for c in g.g_map.szBackgroundFile.bytes() {
            hm.add(c as i32);
        }
        hm.add(g.g_map.platforms.len() as i32);

        let mut hp = Fnv::new();
        for i in 0..ed().g_iNumPlatforms.max(0) as usize {
            let p = &ed().g_Platforms[i];
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
        for t in ed().animatedtiletypes.iter() {
            ha.add(t.0 as i32);
        }

        let _ = writeln!(out, "H map={:08x} plat={:08x} anim={:08x}", hm.0, hp.0, ha.0);
    }
}
