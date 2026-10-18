//! Port of src/worldeditor/worldeditor.cpp
//!
//! The C++ file defines its own copies of `screen`, `blitdest`, the object containers and the
//! other game globals the shared code links against; the port reuses the crate's (smw::main,
//! smw::gs_gameplay) and only defines the editor's own state here.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::resource_manager::SpriteId;
use crate::common::keys::*;
use crate::common::gfx::soft_surface::Surface;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::cmd_args as cmd;
use crate::common::editor_harness;
use crate::common::file_io::BinaryFile;
use crate::common::file_list::{FiltersList, GraphicsList, WorldList};
use crate::common::game::ensure_settings_dir;
use crate::common::game_mode::{game_mode_owned, GAMEMODE_LAST, GAMEMODE_NUM_OPTIONS};
use crate::common::game_mode_settings_serialization::serialize_gms;
use crate::common::game_values::{controlkeys, TITLESTRING};
use crate::common::gfx::color::colors;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::gfx::{gfx_changefullscreen, gfx_show_catched_error, gfx_flipscreen, gfx_init, gfx_settitle};
use crate::common::global_constants::*;
use crate::common::input::{DEVICE_KEYBOARD, NUM_KEYS};
use crate::common::map::{read_type_preview, CMap};
use crate::common::map_list::MapList;
use crate::common::math::vec2::Vec2s;
use crate::common::path::{convert_path, get_home_directory, get_name_from_file_name, initialize_paths};
use crate::common::resource_manager::CResourceManager;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_image_select_field::MI_ImageSelectField;
use crate::common::ui::mi_map_field::MI_MapField;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::common::world_tour_stop::TourStop;
use crate::globals::*;
use crate::smw::menu::mode_options_menu::UI_ModeOptionsMenu;
use crate::smw::world::*;
use crate::common::editor_event::EditorEvent;
use crate::common::util::sdl_helpers::poll_event;
use crate::common::keys::{SDL_BUTTON_LEFT, SDL_BUTTON_RIGHT, SDL_KEYDOWN, SDL_KEYUP, SDL_MOUSEBUTTONDOWN, SDL_MOUSEBUTTONUP, SDL_MOUSEMOTION, SDL_QUIT};
use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::ptr::{null, null_mut};

const EDITOR_EDIT: i32 = 0;
const EDITOR_WATER: i32 = 1;
const EDITOR_BACKGROUND: i32 = 2;
const EDITOR_STAGEFOREGROUND: i32 = 3;
const EDITOR_STRUCTUREFOREGROUND: i32 = 4;
const EDITOR_BRIDGES: i32 = 5;
const EDITOR_PATHSPRITE: i32 = 6;
const EDITOR_VEHICLES: i32 = 7;
const EDITOR_QUIT: i32 = 8;
const SAVE_AS: i32 = 9;
const FIND: i32 = 10;
const CLEAR_WORLD: i32 = 11;
const NEW_WORLD: i32 = 12;
const RESIZE_WORLD: i32 = 13;
const SAVE: i32 = 14;
const EDITOR_WARP: i32 = 15;
const DISPLAY_HELP: i32 = 16;
const EDITOR_PATH: i32 = 17;
const EDITOR_TYPE: i32 = 18;
const EDITOR_BOUNDARY: i32 = 19;
const EDITOR_START_ITEMS: i32 = 20;
const EDITOR_STAGE: i32 = 21;

pub const MAPTITLESTRING: &str = "World Editor";

const szEditModes: [&str; 10] = [
    "Background Mode",
    "Foreground Mode",
    "Path Sprite Mode",
    "Stage Mode",
    "Path Mode",
    "Vehicle Mode",
    "Warp Mode",
    "Start/Door Mode",
    "Boundary Mode",
    "Stage Mode",
];

/// The editor's state, a part of `Game`.
/// The menu the world editor shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorMenu {
    None,
    StageSettings,
    BonusItemPicker,
    Vehicle,
    ModeSettings(u8),
    BossSettings,
}

fn current_menu(g: &mut Game) -> &'static mut UI_Menu {
    let e: *mut WorldEditorState = ed(g);
    let e = unsafe { &mut *e };
    match e.mCurrentMenu {
        EditorMenu::None => panic!("no current menu"),
        EditorMenu::StageSettings => &mut e.mStageSettingsMenu,
        EditorMenu::BonusItemPicker => &mut e.mBonusItemPicker,
        EditorMenu::Vehicle => &mut e.mVehicleMenu,
        EditorMenu::ModeSettings(mode) => &mut e.mModeOptionsMenu.as_mut().expect("mode options menu").mModeSettingsMenu[mode as usize],
        EditorMenu::BossSettings => &mut e.mModeOptionsMenu.as_mut().expect("mode options menu").mBossSettingsMenu,
    }
}

pub struct WorldEditorState {
    pub sMapSurface: *mut Surface,
    pub rectSrcSurface: Rect,
    pub rectDstSurface: Rect,
    pub fNeedBlackBackground: bool,
    pub iWorldWidth: i16,
    pub iWorldHeight: i16,
    pub event: EditorEvent,
    pub spr_dialog: SpriteId,
    pub menu_shade: Global<gfxSprite>,
    pub spr_largedialog: Global<gfxSprite>,
    pub spr_warps: Global<[gfxSprite; 3]>,
    pub spr_path: Global<gfxSprite>,
    pub spr_vehicleicons: SpriteId,
    pub set_tile: i32,
    pub fAutoPaint: bool,
    pub edit_mode: i32,
    pub draw_offset_col: i32, //col and row offset for drawing map to surface
    pub draw_offset_row: i32,
    pub draw_offset_x: i32, //x and y offset for drawing maps smaller than screensize
    pub draw_offset_y: i32,
    pub state: i32,
    pub selectedtiles: [[bool; MAPHEIGHT as usize]; MAPWIDTH as usize],
    pub moveselectedtiles: [[bool; MAPHEIGHT as usize]; MAPWIDTH as usize],
    pub mouse_x: i32,
    pub mouse_y: i32,
    //Vehicle structure that holds the current vehicle "stamp"
    pub g_wvVehicleStamp: Global<WorldVehicle>,
    pub ignoreclick: bool,
    pub findstring: String,
    pub g_musiccategorydisplaytimer: i16,
    pub g_messagedisplaytimer: i16,
    pub g_szMessageTitle: String,
    pub g_szMessageLine: [String; 3],
    //Vehicle stuff
    pub vehiclelist: Vec<Ptr<WorldVehicle>>,
    //Warp stuff
    pub warplist: Vec<Ptr<WorldWarp>>,
    pub g_fFullScreen: bool,
    pub g_fShowStagePreviews: bool,
    //Stage Mode Menu
    pub mCurrentMenu: EditorMenu,
    pub mStageSettingsMenu: Global<UI_Menu>,
    pub mBonusItemPicker: Global<UI_Menu>,
    pub miModeField: Ctl<MI_ImageSelectField>,
    pub miGoalField: [Ctl<MI_SelectField<i16>>; GAMEMODE_LAST as usize],
    pub miModeSettingsButton: Ctl<MI_Button>,
    pub miBonusItemsButton: Ctl<MI_Button>,
    pub miSpecialGoalField: [Ctl<MI_SelectField<i16>>; 3],
    pub miFinalStageField: Ctl<MI_SelectField<bool>>,
    pub miPointsField: Ctl<MI_SelectField<i16>>,
    pub miNameField: Ctl<MI_TextField>,
    pub miBonusType: Ctl<MI_SelectField<i16>>,
    pub miBonusTextField: [Ctl<MI_TextField>; 5],
    pub miMapField: Ctl<MI_MapField>,
    pub miDeleteStageButton: Ctl<MI_Button>,
    pub miDeleteStageDialogImage: Ctl<MI_Image>,
    pub miDeleteStageDialogAreYouText: Ctl<MI_Text>,
    pub miDeleteStageDialogSureText: Ctl<MI_Text>,
    pub miDeleteStageDialogYesButton: Ctl<MI_Button>,
    pub miDeleteStageDialogNoButton: Ctl<MI_Button>,
    //Vehicle Creation Menu
    pub mVehicleMenu: Global<UI_Menu>,
    pub miVehicleSpriteField: Ctl<MI_ImageSelectField>,
    pub miVehicleStageField: Ctl<MI_ImageSelectField>,
    pub miVehicleMinMovesField: Ctl<MI_SelectField<i16>>,
    pub miVehicleMaxMovesField: Ctl<MI_SelectField<i16>>,
    pub miVehiclePacesField: Ctl<MI_SelectField<bool>>,
    pub miVehicleDirectionField: Ctl<MI_SelectField<i16>>,
    pub miVehicleBoundaryField: Ctl<MI_SelectField<i16>>,
    pub miVehicleCreateButton: Ctl<MI_Button>,
    pub miTitleText: Ctl<MI_Text>,
    pub mModeOptionsMenu: Option<Box<UI_ModeOptionsMenu>>,
    pub sMapThumbnail: gfxSprite,
    pub iOldStageId: i16,
    pub stagemodes: Vec<StageMode>,
    pub rItemDst: [Rect; NUM_WORLD_ITEMS as usize],
    pub iLastStageType: i16,
}

impl WorldEditorState {
    pub const fn new() -> Self {
        WorldEditorState {
            sMapSurface: null_mut(),
            rectSrcSurface: Rect { x: 0, y: 0, w: 768, h: 608 },
            rectDstSurface: Rect { x: 0, y: 0, w: 640, h: 480 },
            fNeedBlackBackground: false,
            iWorldWidth: 0,
            iWorldHeight: 0,
            event: EditorEvent::new(),
            spr_dialog: SpriteId::NONE,
            menu_shade: Global::uninit(),
            spr_largedialog: Global::uninit(),
            spr_warps: Global::uninit(),
            spr_path: Global::uninit(),
            spr_vehicleicons: SpriteId::NONE,
            set_tile: 0,
            fAutoPaint: true,
            edit_mode: 0,
            draw_offset_col: 0,
            draw_offset_row: 0,
            draw_offset_x: 0,
            draw_offset_y: 0,
            state: 0,
            selectedtiles: [[false; MAPHEIGHT as usize]; MAPWIDTH as usize],
            moveselectedtiles: [[false; MAPHEIGHT as usize]; MAPWIDTH as usize],
            mouse_x: 0,
            mouse_y: 0,
            g_wvVehicleStamp: Global::uninit(),
            ignoreclick: false,
            findstring: String::new(),
            g_musiccategorydisplaytimer: 0,
            g_messagedisplaytimer: 0,
            g_szMessageTitle: String::new(),
            g_szMessageLine: [String::new(), String::new(), String::new()],
            vehiclelist: Vec::new(),
            warplist: Vec::new(),
            g_fFullScreen: false,
            g_fShowStagePreviews: true,
            mCurrentMenu: EditorMenu::None,
            mStageSettingsMenu: Global::uninit(),
            mBonusItemPicker: Global::uninit(),
            miModeField: Ctl::NONE,
            miGoalField: [Ctl::NONE; GAMEMODE_LAST as usize],
            miModeSettingsButton: Ctl::NONE,
            miBonusItemsButton: Ctl::NONE,
            miSpecialGoalField: [Ctl::NONE; 3],
            miFinalStageField: Ctl::NONE,
            miPointsField: Ctl::NONE,
            miNameField: Ctl::NONE,
            miBonusType: Ctl::NONE,
            miBonusTextField: [Ctl::NONE; 5],
            miMapField: Ctl::NONE,
            miDeleteStageButton: Ctl::NONE,
            miDeleteStageDialogImage: Ctl::NONE,
            miDeleteStageDialogAreYouText: Ctl::NONE,
            miDeleteStageDialogSureText: Ctl::NONE,
            miDeleteStageDialogYesButton: Ctl::NONE,
            miDeleteStageDialogNoButton: Ctl::NONE,
            mVehicleMenu: Global::uninit(),
            miVehicleSpriteField: Ctl::NONE,
            miVehicleStageField: Ctl::NONE,
            miVehicleMinMovesField: Ctl::NONE,
            miVehicleMaxMovesField: Ctl::NONE,
            miVehiclePacesField: Ctl::NONE,
            miVehicleDirectionField: Ctl::NONE,
            miVehicleBoundaryField: Ctl::NONE,
            miVehicleCreateButton: Ctl::NONE,
            miTitleText: Ctl::NONE,
            mModeOptionsMenu: None,
            sMapThumbnail: gfxSprite::new(),
            iOldStageId: -1,
            stagemodes: Vec::new(),
            rItemDst: [Rect { x: 0, y: 0, w: 0, h: 0 }; NUM_WORLD_ITEMS as usize],
            iLastStageType: 0,
        }
    }
}

fn ed(g: &mut Game) -> &mut WorldEditorState {
    &mut g.world_editor
}

fn bound_to_window_w(x: i32) -> i32 {
    0.max(x.min(640 - 1))
}

fn bound_to_window_h(y: i32) -> i32 {
    0.max(y.min(480 - 1))
}

fn bound_mouse_motion_coords(g: &mut Game) {
    let mut gp = g.ptr();
    ed(gp.get()).mouse_x = bound_to_window_w(ed(gp.get()).event.motion_x());
    ed(gp.get()).mouse_y = bound_to_window_h(ed(gp.get()).event.motion_y());
}

fn check_key(g: &mut Game, keystate: *const u8, key: Keycode) -> bool {
    unsafe { *keystate.add(crate::services::services(g).keyboard.scancode_from_key(key) as usize) != 0 }
}

//Sets up default mode options
#[derive(Clone, Default)]
pub struct StageModeOption {
    pub szName: String,
    pub iValue: i16,
}

#[derive(Clone, Default)]
pub struct StageMode {
    pub szName: String,
    pub szGoal: String,

    pub iDefaultGoal: i16,

    pub options: Vec<StageModeOption>,
}

/// `strncpy(dst, src, n); dst[n - 1] = 0;` into a `char[n]`.
fn truncate_cstr(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len().min(n - 1);
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

pub fn set_stage_mode(g: &mut Game, iIndex: i16, szModeName: &str, szGoalName: &str, iIncrement: i16, iDefault: i16) {
    let mut gp = g.ptr();
    if iIndex < 0 || iIndex as i32 >= GAMEMODE_LAST {
        return;
    }

    let sm = &mut ed(gp.get()).stagemodes[iIndex as usize];

    sm.szName = truncate_cstr(szModeName, 128);

    sm.szGoal = truncate_cstr(szGoalName, 64);

    sm.iDefaultGoal = iDefault;

    for iModeOption in 0..(GAMEMODE_NUM_OPTIONS - 1) {
        sm.options[iModeOption].iValue = ((iModeOption as i32 + 1) * iIncrement as i32) as i16;
        sm.options[iModeOption].szName = format!("{}", sm.options[iModeOption].iValue);
    }
}

fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

fn key_sym(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    ed(gp.get()).event.key_sym()
}

fn event_type(g: &mut Game) -> u32 {
    let mut gp = g.ptr();
    ed(gp.get()).event.type_()
}

const T_QUIT: u32 = SDL_QUIT;
const T_KEYDOWN: u32 = SDL_KEYDOWN;
const T_KEYUP: u32 = SDL_KEYUP;
const T_MOUSEBUTTONDOWN: u32 = SDL_MOUSEBUTTONDOWN;
const T_MOUSEBUTTONUP: u32 = SDL_MOUSEBUTTONUP;
const T_MOUSEMOTION: u32 = SDL_MOUSEMOTION;

const BUTTON_LEFT: u8 = SDL_BUTTON_LEFT as u8;
const BUTTON_RIGHT: u8 = SDL_BUTTON_RIGHT as u8;
const SDL_BUTTON_LMASK: u32 = 1 << (SDL_BUTTON_LEFT - 1);
const SDL_BUTTON_RMASK: u32 = 1 << (SDL_BUTTON_RIGHT - 1);

/// The end-of-frame wait every editor loop does; the reference harness `#define`s `SDL_Delay` to this.
fn frame_wait(g: &mut Game, framestart: i32) {
    let mut gp = g.ptr();
    let delay = ms_to_next_tick(framestart as u32, crate::services::ticks(gp.get()));
    editor_harness::frame_delay(g, delay);
}

fn tile(g: &mut Game, iCol: i16, iRow: i16) -> &'static mut WorldMapTile {
    let mut gp = g.ptr();
    gp.get().g_worldmap.tiles.at_mut(iCol as usize, iRow as usize)
}


fn dump_editor_state(g: &mut Game, out: &mut dyn Write) {
    let mut gp = g.ptr();
    let _ = writeln!(
        out,
        "E state={} edit_mode={} set_tile={} auto={} col={} row={} w={} h={} stages={} vehicles={} warps={} world={}",
        ed(gp.get()).state,
        ed(gp.get()).edit_mode,
        ed(gp.get()).set_tile,
        ed(gp.get()).fAutoPaint as i32,
        ed(gp.get()).draw_offset_col,
        ed(gp.get()).draw_offset_row,
        ed(gp.get()).iWorldWidth,
        ed(gp.get()).iWorldHeight,
        g.game_values.tourstops.len(),
        ed(gp.get()).vehiclelist.len(),
        ed(gp.get()).warplist.len(),
        g.worldlist.current_path().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    );
}

//main main main
pub fn main() {
    main_with(std::env::args().collect(), None);
}

/// `main` on the given backend instead of SDL2.
pub fn main_with(argv: Vec<String>, services: Option<smw_platform::Services>) {
    let g = Game::create();
    g.services = services;
    cmd::attach_parent_console();
    crate::globals::init_globals(g);

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
        g.RootDataDirectory = cmd.data_root.clone();
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around inner_main().
    let mut gp = g.ptr();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inner_main(gp.get())));
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

pub fn inner_main(g: &mut Game) {
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

        let mut saved_col: i32 = 0;
        let mut saved_row: i32 = 0;
        {
            let options_path = get_home_directory(g) + "worldeditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "rb");
            if editor_settings.is_open() {
                saved_col = editor_settings.read_i32();
                saved_row = editor_settings.read_i32();
                ed(gp.get()).g_fFullScreen = editor_settings.read_bool();
                ed(gp.get()).findstring = editor_settings.read_string_long(FILEBUFSIZE as usize);
            }
        }

        gfx_init(g, 640, 480, ed(gp.get()).g_fFullScreen);
        g.blitdest = g.screen;
        editor_harness::init(g);
        editor_harness::set_dumper(g, dump_editor_state);

        g.rm = Ptr::new_box(CResourceManager::new());
        let mut rm = g.rm;

        g.g_map.set(CMap::new());
        gp.g_tilesetmanager = Ptr::new_box(CTilesetManager::new(g, &convert_path(gp.get(), "gfx/Classic/tilesets")));
        g.filterslist = Ptr::new_box(FiltersList::new(g));
        g.maplist = Ptr::new_box(MapList::new(g, true));
        g.menugraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        g.gamegraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        g.worldlist = Ptr::new_box(WorldList::new(g));

        g.game_values.sound = false;
        g.game_values.music = false;

        let title = format!("{} {}", TITLESTRING, MAPTITLESTRING);
        gfx_settitle(g, &title);

        g.game_values.toplayer = true;

        println!("\n---------------- loading graphics ----------------");

        ed(gp.get()).spr_warps.init([
            ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp.png")).create(g),
            ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp_preview.png")).create(g),
            ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_warp_thumbnail.png")).create(g),
        ]);

        ed(gp.get()).spr_path.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_world_path.png")).create(g));

        *g.rm.sprite(g.rm.spr_selectedtile) = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_selectedtile.png")).with_color_key(colors::BLACK).with_alpha(128).create(g);

        ed(gp.get()).spr_dialog = gp.rm.add_sprite(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_dialog.png")).with_alpha(255).create(g));
        ed(gp.get()).menu_shade.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_shade.png")).with_alpha(128).create(g));
        ed(gp.get()).spr_largedialog.init(ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_platform.png")).with_alpha(255).create(g));

        gp.rm.menu_font_small = gfxFont::from_path(g, &convert_path(gp.get(), "gfx/packs/Classic/fonts/font_small.png"));
        gp.rm.menu_font_large = gfxFont::from_path(g, &convert_path(gp.get(), "gfx/packs/Classic/fonts/font_large.png"));

        println!("\n---------------- load world ----------------");

        *g.rm.sprite(g.rm.spr_worldbackground[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_background.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldbackground[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/preview/world_background.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldbackground[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/thumbnail/world_background.png")).create(g);

        *g.rm.sprite(g.rm.spr_worldforeground[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_foreground.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldforeground[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/preview/world_foreground.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldforeground[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/thumbnail/world_foreground.png")).create(g);

        *g.rm.sprite(g.rm.spr_worldforegroundspecial[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_foreground_special.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldforegroundspecial[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/preview/world_foreground_special.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldforegroundspecial[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/thumbnail/world_foreground_special.png")).create(g);

        *g.rm.sprite(g.rm.spr_worldpaths[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_paths.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldpaths[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/preview/world_paths.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldpaths[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/thumbnail/world_paths.png")).create(g);

        *g.rm.sprite(g.rm.spr_worldvehicle[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_vehicles.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldvehicle[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/preview/world_vehicles.png")).create(g);
        *g.rm.sprite(g.rm.spr_worldvehicle[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/thumbnail/world_vehicles.png")).create(g);

        *g.rm.sprite(g.rm.spr_worlditems) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_powerups.png")).create(g);
        *g.rm.sprite(g.rm.spr_worlditempopup) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_item_popup.png")).create(g);

        *g.rm.sprite(g.rm.spr_storedpowerupsmall) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/powerups/small.png")).create(g);
        *g.rm.sprite(g.rm.spr_worlditemssmall) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_powerupssmall.png")).create(g);
        *g.rm.sprite(g.rm.spr_worlditemsplace) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/world/world_bonusplace.png")).create(g);

        *g.rm.sprite(g.rm.menu_dialog) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_dialog.png")).create(g);

        //Mode Options Menu Gfx
        *g.rm.sprite(g.rm.menu_egg) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/menu_egg.png")).create(g);
        *g.rm.sprite(g.rm.menu_stomp) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/menu_stomp.png")).create(g);
        *g.rm.sprite(g.rm.menu_survival) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/menu_survival.png")).create(g);
        *g.rm.sprite(g.rm.spr_phanto) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/modeobjects/phanto.png")).create(g);
        *g.rm.sprite(g.rm.menu_plain_field) = ImageLoader::new(convert_path(g, "gfx/leveleditor/menu_plain_field.png")).create(g);
        *g.rm.sprite(g.rm.menu_slider_bar) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_slider_bar.png")).create(g);
        *g.rm.sprite(g.rm.spr_selectfield) = ImageLoader::new(convert_path(g, "gfx/leveleditor/menu_selectfield.png")).create(g);
        *g.rm.sprite(g.rm.menu_verticalarrows) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_vertical_arrows.png")).create(g);
        *g.rm.sprite(g.rm.spr_storedpoweruplarge) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/powerups/large.png")).create(g);

        *g.rm.sprite(g.rm.menu_mode_small) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_mode_small.png")).create(g);
        *g.rm.sprite(g.rm.menu_mode_large) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_mode_large.png")).create(g);

        ed(gp.get()).spr_vehicleicons = gp.rm.add_sprite(ImageLoader::new(convert_path(g, "gfx/leveleditor/vehicle_icons.png")).create(g));

        *g.rm.sprite(g.rm.spr_thumbnail_warps[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_warp_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_thumbnail_warps[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_warp_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_thumbnail_mapitems[0]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_mapitems_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_thumbnail_mapitems[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/menu/menu_mapitems_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_tileanimation[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_tileanimation[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/tile_animation_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_blocks[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/blocks_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_blocks[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/blocks_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_unknowntile[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/unknown_tile_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_unknowntile[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/tilesets/unknown_tile_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_hazard_fireball[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/fireball_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_hazard_fireball[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/fireball_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_hazard_rotodisc[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/rotodisc_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_hazard_rotodisc[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/rotodisc_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_hazard_bulletbill[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/bulletbill_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_hazard_bulletbill[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/bulletbill_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_hazard_flame[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/flame_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_hazard_flame[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/flame_thumbnail.png")).create(g);

        *g.rm.sprite(g.rm.spr_hazard_pirhanaplant[1]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/pirhanaplant_preview.png")).create(g);
        *g.rm.sprite(g.rm.spr_hazard_pirhanaplant[2]) = ImageLoader::new(convert_path(g, "gfx/packs/Classic/hazards/pirhanaplant_thumbnail.png")).create(g);

        gp.rm.load_menu_graphics(g);

        ed(gp.get()).sMapSurface = crate::common::gfx::soft_surface::alloc_surface(Surface::new_xrgb(768, 608));

        let fs = ed(gp.get()).findstring.clone();
        g.worldlist.find(&fs);
        g.game_values.worldindex = g.worldlist.current_index() as i16;
        loadcurrentworld(g);
        ed(gp.get()).findstring.clear(); //clear out the find string so that pressing "f" will give you the find dialog

        if saved_row >= 0 && saved_row <= ed(gp.get()).iWorldHeight as i32 - 15 && saved_col >= 0 && saved_col <= ed(gp.get()).iWorldWidth as i32 - 20 {
            ed(gp.get()).draw_offset_row = saved_row;
            ed(gp.get()).draw_offset_col = saved_col;
            updateworldsurface(g);
        }

        //Setup input for menus
        g.game_values.inputConfiguration[0][0].iDevice = DEVICE_KEYBOARD;
        for iInputState in 0..2usize {
            //for game/menu
            for iKey in 0..NUM_KEYS as usize {
                g.game_values.inputConfiguration[0][0].inputGameControls[iInputState].keys[iKey] = controlkeys[0][iInputState][0][iKey];
            }
        }

        g.game_values.playerInput.inputControls[0] = Some(((0) as u8, (0) as u8));

        ed(gp.get()).stagemodes = vec![StageMode { options: vec![StageModeOption::default(); GAMEMODE_NUM_OPTIONS - 1], ..Default::default() }; GAMEMODE_LAST as usize];

        set_stage_mode(g, 0, "Classic", "Lives", 5, 10);
        set_stage_mode(g, 1, "Frag", "Kills", 5, 20);
        set_stage_mode(g, 2, "Time Limit", "Time", 30, 60);
        set_stage_mode(g, 3, "Jail", "Kills", 5, 20);
        set_stage_mode(g, 4, "Coin Collection", "Coins", 5, 20);
        set_stage_mode(g, 5, "Stomp", "Kills", 10, 10);
        set_stage_mode(g, 6, "Yoshi's Eggs", "Eggs", 5, 20);
        set_stage_mode(g, 7, "Capture The Flag", "Points", 5, 20);
        set_stage_mode(g, 8, "Chicken", "Points", 50, 200);
        set_stage_mode(g, 9, "Tag", "Points", 50, 200);
        set_stage_mode(g, 10, "Star", "Lives", 1, 5);
        set_stage_mode(g, 11, "Domination", "Points", 50, 200);
        set_stage_mode(g, 12, "King of the Hill", "Points", 50, 200);
        set_stage_mode(g, 13, "Race", "Laps", 2, 10);
        set_stage_mode(g, 14, "Owned", "Points", 50, 200);
        set_stage_mode(g, 15, "Frenzy", "Kills", 5, 20);
        set_stage_mode(g, 16, "Survival", "Lives", 5, 10);
        set_stage_mode(g, 17, "Greed", "Coins", 10, 40);
        set_stage_mode(g, 18, "Health", "Lives", 1, 5);
        set_stage_mode(g, 19, "Card Collection", "Points", 10, 30);
        set_stage_mode(g, 20, "Phanto Chase", "Points", 50, 200);
        set_stage_mode(g, 21, "Shy Guy Tag", "Points", 50, 200);

        ed(gp.get()).mStageSettingsMenu.init(UI_Menu::new());
        ed(gp.get()).mBonusItemPicker.init(UI_Menu::new());
        ed(gp.get()).mVehicleMenu.init(UI_Menu::new());

        //Setup The Mode Menu
        ed(gp.get()).mCurrentMenu = EditorMenu::StageSettings;

        //Name
        ed(gp.get()).miNameField = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 20, "Name", 500, 120), g);
        g.ctl(ed(gp.get()).miNameField).set_disallowed_chars(",");

        ed(gp.get()).miModeField = Ctl::new(MI_ImageSelectField::new(g.rm.spr_selectfield, g.rm.menu_mode_small, 70, 60, "Mode", 500, 120, 16, 16), g);
        //miModeField->SetData(game_values.tourstops[0]->iMode, NULL, NULL);
        //miModeField->SetKey(0);
        g.ctl(ed(gp.get()).miModeField).set_item_changed_code(MENU_CODE_MODE_CHANGED);

        for iGameMode in 0..GAMEMODE_LAST as usize {
            g.ctl(ed(gp.get()).miModeField).add(ed(gp.get()).stagemodes[iGameMode].szName.clone(), iGameMode as i16);

            ed(gp.get()).miGoalField[iGameMode] = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 100, ed(gp.get()).stagemodes[iGameMode].szGoal.clone(), 352, 120), g);
            g.ctl(ed(gp.get()).miGoalField[iGameMode]).set_visible(iGameMode == 0);

            for iGameModeOption in 0..(GAMEMODE_NUM_OPTIONS - 1) {
                let option = ed(gp.get()).stagemodes[iGameMode].options[iGameModeOption].clone();
                g.ctl(ed(gp.get()).miGoalField[iGameMode]).add(option.szName, option.iValue);
            }

            //miGoalField[iGameMode]->SetData(&gamemodes[iGameMode]->goal, NULL, NULL);
            //miGoalField[iGameMode]->SetKey(gamemodes[iGameMode]->goal);
        }

        g.ctl(ed(gp.get()).miModeField).add("Bonus House", 24);
        g.ctl(ed(gp.get()).miModeField).add("Pipe Minigame", 25);
        g.ctl(ed(gp.get()).miModeField).add("Boss Minigame", 26);
        g.ctl(ed(gp.get()).miModeField).add("Boxes Minigame", 27);

        //Create goal field for pipe game
        ed(gp.get()).miSpecialGoalField[0] = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 100, "Points", 352, 120), g);
        g.ctl(ed(gp.get()).miSpecialGoalField[0]).set_visible(false);

        for iGameModeOption in 0..(GAMEMODE_NUM_OPTIONS as i16 - 1) {
            let iValue: i16 = 10 + iGameModeOption * 10;
            g.ctl(ed(gp.get()).miSpecialGoalField[0]).add(format!("{}", iValue), iValue);
        }

        //Create goal field for boss game
        ed(gp.get()).miSpecialGoalField[1] = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 100, "Lives", 352, 120), g);
        g.ctl(ed(gp.get()).miSpecialGoalField[1]).set_visible(false);

        for iGameLives in 1..=30i16 {
            g.ctl(ed(gp.get()).miSpecialGoalField[1]).add(format!("{}", iGameLives), iGameLives);
        }

        //Create goal field for boxes game
        ed(gp.get()).miSpecialGoalField[2] = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 100, "Lives", 352, 120), g);
        g.ctl(ed(gp.get()).miSpecialGoalField[2]).set_visible(false);

        for iGameLives in 1..=30i16 {
            g.ctl(ed(gp.get()).miSpecialGoalField[2]).add(format!("{}", iGameLives), iGameLives);
        }

        //Mode Settings Button
        ed(gp.get()).miModeSettingsButton = Ctl::new(MI_Button::new_left(g, gp.rm.spr_selectfield, 430, 100, "Settings", 140), g);
        g.ctl(ed(gp.get()).miModeSettingsButton).set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);

        //Points Field
        ed(gp.get()).miPointsField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 140, "Points", 245, 120), g);
        for iPoints in 0..=20i16 {
            g.ctl(ed(gp.get()).miPointsField).add(format!("{}", iPoints), iPoints);
        }

        //Final Stage Field
        ed(gp.get()).miFinalStageField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 325, 140, "End Stage", 245, 120), g);
        g.ctl(ed(gp.get()).miFinalStageField).add("No", false);
        g.ctl(ed(gp.get()).miFinalStageField).add("Yes", true);
        g.ctl(ed(gp.get()).miFinalStageField).set_auto_advance(true);

        //Map Select Field
        ed(gp.get()).miMapField = Ctl::new(MI_MapField::new(g, rm.spr_selectfield, 70, 180, "Map", 500, 120, true), g);

        //Bonus Item Picker Menu Button
        ed(gp.get()).miBonusItemsButton = Ctl::new(MI_Button::new_left(g, gp.rm.spr_selectfield, 430, 220, "Bonuses", 140), g);
        g.ctl(ed(gp.get()).miBonusItemsButton).set_code(MENU_CODE_TO_BONUS_PICKER_MENU);

        //Bonus Type
        ed(gp.get()).miBonusType = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 100, "Type", 500, 120), g);
        g.ctl(ed(gp.get()).miBonusType).add("Fixed", false as i16);
        g.ctl(ed(gp.get()).miBonusType).add("Random", true as i16);
        g.ctl(ed(gp.get()).miBonusType).set_auto_advance(true);

        //Bonus House Text * 5
        ed(gp.get()).miBonusTextField[0] = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 140, "Text", 500, 120), g);
        ed(gp.get()).miBonusTextField[1] = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 180, "Text", 500, 120), g);
        ed(gp.get()).miBonusTextField[2] = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 220, "Text", 500, 120), g);
        ed(gp.get()).miBonusTextField[3] = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 260, "Text", 500, 120), g);
        ed(gp.get()).miBonusTextField[4] = Ctl::new(MI_TextField::new(g.rm.spr_selectfield, 70, 300, "Text", 500, 120), g);

        g.ctl(ed(gp.get()).miBonusTextField[0]).set_disallowed_chars(",|");
        g.ctl(ed(gp.get()).miBonusTextField[1]).set_disallowed_chars(",|");
        g.ctl(ed(gp.get()).miBonusTextField[2]).set_disallowed_chars(",|");
        g.ctl(ed(gp.get()).miBonusTextField[3]).set_disallowed_chars(",|");
        g.ctl(ed(gp.get()).miBonusTextField[4]).set_disallowed_chars(",|");

        //Delete Stage Button
        ed(gp.get()).miDeleteStageButton = Ctl::new(MI_Button::new_left(g, gp.rm.spr_selectfield, 430, 440, "Delete", 140), g);
        g.ctl(ed(gp.get()).miDeleteStageButton).set_code(MENU_CODE_DELETE_STAGE_BUTTON);

        //Are You Sure Dialog for Delete Stage
        ed(gp.get()).miDeleteStageDialogImage = Ctl::new(MI_Image::new(ed(gp.get()).spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        ed(gp.get()).miDeleteStageDialogAreYouText = Ctl::new(MI_HeaderText::new(g, "Are You", 320, 195), g);
        ed(gp.get()).miDeleteStageDialogSureText = Ctl::new(MI_HeaderText::new(g, "Sure?", 320, 220), g);
        ed(gp.get()).miDeleteStageDialogYesButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        ed(gp.get()).miDeleteStageDialogNoButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(ed(gp.get()).miDeleteStageDialogYesButton).set_code(MENU_CODE_DELETE_STAGE_YES);
        g.ctl(ed(gp.get()).miDeleteStageDialogNoButton).set_code(MENU_CODE_DELETE_STAGE_NO);

        g.ctl(ed(gp.get()).miDeleteStageDialogImage).set_visible(false);
        g.ctl(ed(gp.get()).miDeleteStageDialogAreYouText).set_visible(false);
        g.ctl(ed(gp.get()).miDeleteStageDialogSureText).set_visible(false);
        g.ctl(ed(gp.get()).miDeleteStageDialogYesButton).set_visible(false);
        g.ctl(ed(gp.get()).miDeleteStageDialogNoButton).set_visible(false);

        let null = ControlId::NONE;

        //Add Name Field
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miNameField.id(), ed(gp.get()).miDeleteStageButton.id(), ed(gp.get()).miModeField.id(), null, null);

        //Add Mode Field
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miModeField.id(), ed(gp.get()).miNameField.id(), ed(gp.get()).miGoalField[0].id(), null, null);

        //Add Mode Goal Fields
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miGoalField[0].id(), ed(gp.get()).miModeField.id(), ed(gp.get()).miGoalField[1].id(), null, ed(gp.get()).miModeSettingsButton.id());

        for iGoalField in 1..(GAMEMODE_LAST as usize - 1) {
            ed(gp.get()).mStageSettingsMenu.add_control(g, 
                ed(gp.get()).miGoalField[iGoalField].id(),
                ed(gp.get()).miGoalField[iGoalField - 1].id(),
                ed(gp.get()).miGoalField[iGoalField + 1].id(),
                ed(gp.get()).miGoalField[iGoalField - 1].id(),
                ed(gp.get()).miModeSettingsButton.id(),
            );
        }

        let last = GAMEMODE_LAST as usize - 1;
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miGoalField[last].id(), ed(gp.get()).miGoalField[last - 1].id(), ed(gp.get()).miSpecialGoalField[0].id(), ed(gp.get()).miGoalField[last - 1].id(), ed(gp.get()).miModeSettingsButton.id());

        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miSpecialGoalField[0].id(), ed(gp.get()).miGoalField[last].id(), ed(gp.get()).miSpecialGoalField[1].id(), ed(gp.get()).miGoalField[last].id(), ed(gp.get()).miSpecialGoalField[1].id());
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miSpecialGoalField[1].id(), ed(gp.get()).miSpecialGoalField[0].id(), ed(gp.get()).miSpecialGoalField[2].id(), ed(gp.get()).miSpecialGoalField[0].id(), ed(gp.get()).miSpecialGoalField[2].id());
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miSpecialGoalField[2].id(), ed(gp.get()).miSpecialGoalField[1].id(), ed(gp.get()).miPointsField.id(), ed(gp.get()).miSpecialGoalField[1].id(), ed(gp.get()).miModeSettingsButton.id());

        //Add Mode Settings Button
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miModeSettingsButton.id(), ed(gp.get()).miModeField.id(), ed(gp.get()).miFinalStageField.id(), ed(gp.get()).miSpecialGoalField[2].id(), null);

        //Add Points Field
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miPointsField.id(), ed(gp.get()).miSpecialGoalField[2].id(), ed(gp.get()).miMapField.id(), null, ed(gp.get()).miFinalStageField.id());

        //Add Final Stage Field
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miFinalStageField.id(), ed(gp.get()).miSpecialGoalField[2].id(), ed(gp.get()).miMapField.id(), ed(gp.get()).miPointsField.id(), null);

        //Add Map Field
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miMapField.id(), ed(gp.get()).miFinalStageField.id(), ed(gp.get()).miBonusType.id(), null, ed(gp.get()).miBonusItemsButton.id());

        //Add Bonus House Fields
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusType.id(), ed(gp.get()).miMapField.id(), ed(gp.get()).miBonusTextField[0].id(), null, null);
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusTextField[0].id(), ed(gp.get()).miBonusType.id(), ed(gp.get()).miBonusTextField[1].id(), null, null);
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusTextField[1].id(), ed(gp.get()).miBonusTextField[0].id(), ed(gp.get()).miBonusTextField[2].id(), null, null);
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusTextField[2].id(), ed(gp.get()).miBonusTextField[1].id(), ed(gp.get()).miBonusTextField[3].id(), null, null);
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusTextField[3].id(), ed(gp.get()).miBonusTextField[2].id(), ed(gp.get()).miBonusTextField[4].id(), null, null);
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusTextField[4].id(), ed(gp.get()).miBonusTextField[3].id(), ed(gp.get()).miBonusItemsButton.id(), null, null);

        //Add Bonus Button
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miBonusItemsButton.id(), ed(gp.get()).miBonusTextField[4].id(), ed(gp.get()).miDeleteStageButton.id(), ed(gp.get()).miMapField.id(), null);

        //Add Delete Stage Button
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miDeleteStageButton.id(), ed(gp.get()).miBonusItemsButton.id(), ed(gp.get()).miNameField.id(), ed(gp.get()).miMapField.id(), null);

        //Add Are You Sure Dialog
        ed(gp.get()).mStageSettingsMenu.add_non_control(ed(gp.get()).miDeleteStageDialogImage.id());
        ed(gp.get()).mStageSettingsMenu.add_non_control(ed(gp.get()).miDeleteStageDialogAreYouText.id());
        ed(gp.get()).mStageSettingsMenu.add_non_control(ed(gp.get()).miDeleteStageDialogSureText.id());

        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miDeleteStageDialogYesButton.id(), null, null, null, ed(gp.get()).miDeleteStageDialogNoButton.id());
        ed(gp.get()).mStageSettingsMenu.add_control(g, ed(gp.get()).miDeleteStageDialogNoButton.id(), null, null, ed(gp.get()).miDeleteStageDialogYesButton.id(), null);

        ed(gp.get()).mStageSettingsMenu.set_initial_focus(g, ed(gp.get()).miNameField.id());
        ed(gp.get()).mStageSettingsMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        ed(gp.get()).mBonusItemPicker.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        let menu = UI_ModeOptionsMenu::new(g);
        ed(gp.get()).mModeOptionsMenu = Some(menu);

        ed(gp.get()).g_wvVehicleStamp.init(WorldVehicle::new());
        ed(gp.get()).g_wvVehicleStamp.iDrawSprite = 0;
        ed(gp.get()).g_wvVehicleStamp.iActionId = 0;
        ed(gp.get()).g_wvVehicleStamp.currentTile.x = 0;
        ed(gp.get()).g_wvVehicleStamp.currentTile.y = 0;
        ed(gp.get()).g_wvVehicleStamp.iMinMoves = 5;
        ed(gp.get()).g_wvVehicleStamp.iMaxMoves = 8;
        ed(gp.get()).g_wvVehicleStamp.fSpritePaces = true;
        ed(gp.get()).g_wvVehicleStamp.iDrawDirection = 0;
        ed(gp.get()).g_wvVehicleStamp.iBoundary = 0;

        //Create Vehicle Menu
        ed(gp.get()).miVehicleSpriteField = Ctl::new(MI_ImageSelectField::new(g.rm.spr_selectfield, ed(gp.get()).spr_vehicleicons, 70, 80, "Sprite", 500, 150, 16, 16), g);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Hammer Brother", 0);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Boomerang Brother", 1);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Fire Brother", 2);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Tank 1", 3);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Boat 1", 4);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Boat 2", 5);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Airship 1", 6);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Airship 2", 7);
        g.ctl(ed(gp.get()).miVehicleSpriteField).add("Tank 2", 8);
        g.ctl(ed(gp.get()).miVehicleSpriteField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iDrawSprite as *mut i16);
        g.ctl(ed(gp.get()).miVehicleSpriteField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iDrawSprite);

        ed(gp.get()).miVehicleStageField = Ctl::new(MI_ImageSelectField::new(g.rm.spr_selectfield, g.rm.menu_mode_small, 70, 120, "Stage", 500, 150, 16, 16), g);
        g.ctl(ed(gp.get()).miVehicleStageField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iActionId as *mut i16);

        ed(gp.get()).miVehicleMinMovesField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 160, "Min Moves", 500, 150), g);

        for iMinMoves in 0..=100i16 {
            g.ctl(ed(gp.get()).miVehicleMinMovesField).add(format!("{}", iMinMoves), iMinMoves);
        }

        g.ctl(ed(gp.get()).miVehicleMinMovesField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iMinMoves as *mut i16);
        g.ctl(ed(gp.get()).miVehicleMinMovesField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iMinMoves);
        g.ctl(ed(gp.get()).miVehicleMinMovesField).set_item_changed_code(MENU_CODE_VEHICLE_MIN_MOVES_CHANGED);
        g.ctl(ed(gp.get()).miVehicleMinMovesField).allow_wrap(false);
        g.ctl(ed(gp.get()).miVehicleMinMovesField).allow_fast_scroll(true);

        ed(gp.get()).miVehicleMaxMovesField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 200, "Max Moves", 500, 150), g);

        for iMaxMoves in 0..=100i16 {
            g.ctl(ed(gp.get()).miVehicleMaxMovesField).add(format!("{}", iMaxMoves), iMaxMoves);
        }

        g.ctl(ed(gp.get()).miVehicleMaxMovesField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iMaxMoves as *mut i16);
        g.ctl(ed(gp.get()).miVehicleMaxMovesField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iMaxMoves);
        g.ctl(ed(gp.get()).miVehicleMaxMovesField).set_item_changed_code(MENU_CODE_VEHICLE_MAX_MOVES_CHANGED);
        g.ctl(ed(gp.get()).miVehicleMaxMovesField).allow_wrap(false);
        g.ctl(ed(gp.get()).miVehicleMaxMovesField).allow_fast_scroll(true);

        ed(gp.get()).miVehiclePacesField = Ctl::new(MI_SelectField::<bool>::new(g.rm.spr_selectfield, 70, 240, "Paces", 500, 150), g);
        g.ctl(ed(gp.get()).miVehiclePacesField).add("No", false);
        g.ctl(ed(gp.get()).miVehiclePacesField).add("Yes", true);
        g.ctl(ed(gp.get()).miVehiclePacesField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.fSpritePaces as *mut bool);
        g.ctl(ed(gp.get()).miVehiclePacesField).set_current_value(if ed(gp.get()).g_wvVehicleStamp.fSpritePaces { 1 } else { 0 } != 0);
        g.ctl(ed(gp.get()).miVehiclePacesField).set_auto_advance(true);

        ed(gp.get()).miVehicleDirectionField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 280, "Direction", 500, 150), g);
        g.ctl(ed(gp.get()).miVehicleDirectionField).add("Left", false as i16);
        g.ctl(ed(gp.get()).miVehicleDirectionField).add("Right", true as i16);
        g.ctl(ed(gp.get()).miVehicleDirectionField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iDrawDirection as *mut i16);
        g.ctl(ed(gp.get()).miVehicleDirectionField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iDrawDirection);
        g.ctl(ed(gp.get()).miVehicleDirectionField).set_auto_advance(true);

        ed(gp.get()).miVehicleBoundaryField = Ctl::new(MI_SelectField::<i16>::new(g.rm.spr_selectfield, 70, 320, "Boundary", 500, 150), g);
        g.ctl(ed(gp.get()).miVehicleBoundaryField).add("No Boundary", 0);

        for iBoundary in 1..=100i16 {
            g.ctl(ed(gp.get()).miVehicleBoundaryField).add(format!("{}", iBoundary), iBoundary);
        }

        g.ctl(ed(gp.get()).miVehicleBoundaryField).set_output_ptr(&mut ed(gp.get()).g_wvVehicleStamp.iBoundary as *mut i16);
        g.ctl(ed(gp.get()).miVehicleBoundaryField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iBoundary);
        g.ctl(ed(gp.get()).miVehicleBoundaryField).allow_fast_scroll(true);

        ed(gp.get()).miVehicleCreateButton = Ctl::new(MI_Button::new(g, gp.rm.spr_selectfield, 430, 360, "OK", 140, TextAlign::CENTER), g);
        g.ctl(ed(gp.get()).miVehicleCreateButton).set_code(MENU_CODE_CREATE_VEHICLE);

        ed(gp.get()).miTitleText = Ctl::new(MI_Text::new(g, "Clicking on the map will add the vehicle configured below", 320, 50, 640, true, TextAlign::CENTER), g);

        ed(gp.get()).mVehicleMenu.add_non_control(ed(gp.get()).miTitleText.id());

        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleSpriteField.id(), ed(gp.get()).miVehicleCreateButton.id(), ed(gp.get()).miVehicleStageField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleStageField.id(), ed(gp.get()).miVehicleSpriteField.id(), ed(gp.get()).miVehicleMinMovesField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleMinMovesField.id(), ed(gp.get()).miVehicleStageField.id(), ed(gp.get()).miVehicleMaxMovesField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleMaxMovesField.id(), ed(gp.get()).miVehicleMinMovesField.id(), ed(gp.get()).miVehiclePacesField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehiclePacesField.id(), ed(gp.get()).miVehicleMaxMovesField.id(), ed(gp.get()).miVehicleDirectionField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleDirectionField.id(), ed(gp.get()).miVehiclePacesField.id(), ed(gp.get()).miVehicleBoundaryField.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleBoundaryField.id(), ed(gp.get()).miVehicleDirectionField.id(), ed(gp.get()).miVehicleCreateButton.id(), null, null);
        ed(gp.get()).mVehicleMenu.add_control(g, ed(gp.get()).miVehicleCreateButton.id(), ed(gp.get()).miVehicleBoundaryField.id(), ed(gp.get()).miVehicleSpriteField.id(), null, null);

        ed(gp.get()).mVehicleMenu.set_initial_focus(g, ed(gp.get()).miVehicleSpriteField.id());
        ed(gp.get()).mVehicleMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        println!("\n---------------- ready, steady, go! ----------------");

        println!("entering world editor loop...");
        done = false;
        while !done {
            match ed(gp.get()).state {
                EDITOR_EDIT => ed(gp.get()).state = editor_edit(g),
                EDITOR_WATER => ed(gp.get()).state = editor_water(g),
                EDITOR_BACKGROUND => ed(gp.get()).state = editor_background(g),
                EDITOR_STAGEFOREGROUND => ed(gp.get()).state = editor_stageforeground(g),
                EDITOR_STRUCTUREFOREGROUND => ed(gp.get()).state = editor_structureforeground(g),
                EDITOR_BRIDGES => ed(gp.get()).state = editor_bridges(g),
                EDITOR_VEHICLES => ed(gp.get()).state = editor_vehicles(g),
                EDITOR_PATH => ed(gp.get()).state = editor_path(g),
                EDITOR_PATHSPRITE => ed(gp.get()).state = editor_pathsprite(g),
                EDITOR_WARP => ed(gp.get()).state = editor_warp(g),
                EDITOR_START_ITEMS => ed(gp.get()).state = editor_start_items(g),
                EDITOR_BOUNDARY => ed(gp.get()).state = editor_boundary(g),
                EDITOR_TYPE => ed(gp.get()).state = editor_type(g),
                EDITOR_STAGE => ed(gp.get()).state = editor_stage(g),
                EDITOR_QUIT => done = true,
                DISPLAY_HELP => ed(gp.get()).state = display_help(g),
                SAVE_AS => ed(gp.get()).state = save_as(g),
                FIND => ed(gp.get()).state = find(g),
                CLEAR_WORLD => ed(gp.get()).state = clear_world(g),
                NEW_WORLD => ed(gp.get()).state = new_world(g),
                RESIZE_WORLD => ed(gp.get()).state = resize_world(g),
                SAVE => ed(gp.get()).state = savecurrentworld(g),
                _ => println!(" PANIC: WEIRD GAMESTATE: {}", ed(gp.get()).state),
            }
        }

        crate::common::gfx::soft_surface::free_surface(ed(gp.get()).sMapSurface);

        println!("\n---------------- save world ----------------");

        WriteVehiclesIntoWorld(g);
        WriteWarpsIntoWorld(g);
        gp.g_worldmap.save(g, &convert_path(gp.get(), "worlds/ZZworldeditor.txt"));

        {
            let options_path = get_home_directory(g) + "worldeditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "wb");
            if editor_settings.is_open() {
                editor_settings.write_i32(ed(gp.get()).draw_offset_col);
                editor_settings.write_i32(ed(gp.get()).draw_offset_row);
                editor_settings.write_bool(ed(gp.get()).g_fFullScreen);
                editor_settings.write_string_long(&g.worldlist.current_path().to_string_lossy());
            }
        }

        println!("\n---------------- shutdown ----------------");
    }
}

/// Painting with the left button (`SDL_MOUSEBUTTONDOWN` and `SDL_MOUSEMOTION` repeat the same block in C++).
fn paint_left(g: &mut Game, iCol: i16, iRow: i16, fromMotion: bool) {
    let mut gp = g.ptr();
    if ed(gp.get()).edit_mode == 0 {
        //selected background
        if tile(g, iCol, iRow).iBackgroundSprite as i32 != ed(gp.get()).set_tile || ed(gp.get()).fAutoPaint {
            let mut fNeedUpdate = false;
            if tile(g, iCol, iRow).iBackgroundSprite as i32 != ed(gp.get()).set_tile {
                tile(g, iCol, iRow).iBackgroundSprite = ed(gp.get()).set_tile as i16;
                fNeedUpdate = true;
            }

            fNeedUpdate |= UpdateForeground(g, iCol, iRow);

            if (ed(gp.get()).set_tile % WORLD_BACKGROUND_SPRITE_SET_SIZE as i32) < 2 && ed(gp.get()).fAutoPaint {
                fNeedUpdate |= UpdateCoastline(g, iCol, iRow);
            }

            if fNeedUpdate {
                updateworldsurface(g);
            }
        }
    } else if ed(gp.get()).edit_mode == 1 {
        //selected foreground
        if tile(g, iCol, iRow).iForegroundSprite as i32 != ed(gp.get()).set_tile {
            tile(g, iCol, iRow).iForegroundSprite = ed(gp.get()).set_tile as i16;
            updateworldsurface(g);

            if !fromMotion && ed(gp.get()).set_tile >= WORLD_BRIDGE_SPRITE_OFFSET as i32 && ed(gp.get()).set_tile <= WORLD_BRIDGE_SPRITE_OFFSET as i32 + 3 {
                tile(g, iCol, iRow).iConnectionType = (ed(gp.get()).set_tile - WORLD_BRIDGE_SPRITE_OFFSET as i32 + 12) as i16;
            }
        }
    } else if ed(gp.get()).edit_mode == 2 {
        //selected connection
        tile(g, iCol, iRow).iConnectionType = ed(gp.get()).set_tile as i16;

        if ed(gp.get()).fAutoPaint {
            UpdatePath(g, iCol, iRow);
        }
    } else if ed(gp.get()).edit_mode == 3 {
        //selected type
        //start tiles
        if ed(gp.get()).set_tile <= 1 {
            if tile(g, iCol, iRow).iForegroundSprite as i32 != ed(gp.get()).set_tile + WORLD_START_SPRITE_OFFSET as i32 {
                tile(g, iCol, iRow).iType = 1;
                tile(g, iCol, iRow).iForegroundSprite = (ed(gp.get()).set_tile + WORLD_START_SPRITE_OFFSET as i32) as i16;
                updateworldsurface(g);
            }
        } else if ed(gp.get()).set_tile <= 5 {
            //doors
            if tile(g, iCol, iRow).iType as i32 != ed(gp.get()).set_tile {
                //if the door was placed on a start tile
                if tile(g, iCol, iRow).iType == 1 {
                    tile(g, iCol, iRow).iForegroundSprite = 0;
                }

                tile(g, iCol, iRow).iType = ed(gp.get()).set_tile as i16;
                updateworldsurface(g);
            }
        }
    } else if ed(gp.get()).edit_mode == 4 {
        //selected path sprite
        let iAdjustedTile = AdjustForeground(g, ed(gp.get()).set_tile as i16, iCol, iRow);
        let mut fNeedUpdate = false;

        if !ed(gp.get()).fAutoPaint && tile(g, iCol, iRow).iForegroundSprite != iAdjustedTile {
            tile(g, iCol, iRow).iForegroundSprite = iAdjustedTile;
            fNeedUpdate = true;
        }

        //Detect if there was a change so we can repaint the screen
        if ed(gp.get()).fAutoPaint {
            let mut iOldTiles = [0i16; 9];
            GetForegroundTileValues(g, iCol, iRow, &mut iOldTiles);
            tile(g, iCol, iRow).iForegroundSprite = iAdjustedTile;
            UpdatePathSprite(g, iCol, iRow);

            if ForegroundTileValuesChanged(g, iCol, iRow, &iOldTiles) {
                fNeedUpdate = true;
            }
        }

        if fNeedUpdate {
            updateworldsurface(g);
        }
    } else if ed(gp.get()).edit_mode == 5 {
        //selected vehicle
        AddVehicleToTile(g, iCol, iRow, ed(gp.get()).set_tile as i16);
    } else if ed(gp.get()).edit_mode == 6 {
        //selected warp
        AddWarpToTile(g, iCol, iRow, ed(gp.get()).set_tile as i16);
    } else if ed(gp.get()).edit_mode == 7 {
        //water
        if tile(g, iCol, iRow).iBackgroundWater as i32 != ed(gp.get()).set_tile {
            tile(g, iCol, iRow).iBackgroundWater = ed(gp.get()).set_tile as i16;
            updateworldsurface(g);
        }
    } else if ed(gp.get()).edit_mode == 8 {
        //boundary
        if !fromMotion {
            tile(g, iCol, iRow).iVehicleBoundary = ed(gp.get()).set_tile as i16;
        }
    } else if ed(gp.get()).edit_mode == 9 {
        //if the stage was placed on a start tile
        if tile(g, iCol, iRow).iType == 1 {
            tile(g, iCol, iRow).iForegroundSprite = 0;
            updateworldsurface(g);
        }

        tile(g, iCol, iRow).iType = ed(gp.get()).set_tile as i16;
    }
}

/// Erasing with the right button; returns the new `iStageDisplay` when C++ resets it.
fn paint_right(g: &mut Game, iCol: i16, iRow: i16, fromMotion: bool, iStageDisplay: &mut i16) {
    let mut gp = g.ptr();
    if ed(gp.get()).edit_mode == 0 {
        if tile(g, iCol, iRow).iBackgroundSprite != 0 || ed(gp.get()).fAutoPaint {
            let mut fNeedUpdate = false;

            if tile(g, iCol, iRow).iBackgroundSprite != 0 {
                tile(g, iCol, iRow).iBackgroundSprite = 0;
                fNeedUpdate = true;
            }

            fNeedUpdate |= UpdateForeground(g, iCol, iRow);

            if ed(gp.get()).fAutoPaint {
                fNeedUpdate |= UpdateCoastline(g, iCol, iRow);
            }

            if fNeedUpdate {
                updateworldsurface(g);
            }
        }
    } else if ed(gp.get()).edit_mode == 1 {
        if tile(g, iCol, iRow).iForegroundSprite != 0 {
            tile(g, iCol, iRow).iForegroundSprite = 0;
            updateworldsurface(g);
        }
    } else if ed(gp.get()).edit_mode == 2 {
        tile(g, iCol, iRow).iConnectionType = 0;

        if ed(gp.get()).fAutoPaint {
            UpdatePath(g, iCol, iRow);
        }
    } else if ed(gp.get()).edit_mode == 3 {
        //selected start/door
        if tile(g, iCol, iRow).iType == 1 {
            tile(g, iCol, iRow).iForegroundSprite = 0;
            updateworldsurface(g);
        } else if tile(g, iCol, iRow).iType <= 5 {
            tile(g, iCol, iRow).iType = 0;
            updateworldsurface(g);
        }

        tile(g, iCol, iRow).iType = 0;
    } else if ed(gp.get()).edit_mode == 4 {
        let mut fNeedUpdate = false;

        if !ed(gp.get()).fAutoPaint && tile(g, iCol, iRow).iForegroundSprite != 0 {
            tile(g, iCol, iRow).iForegroundSprite = 0;
            fNeedUpdate = true;
        }

        //Detect if there was a change so we can repaint the screen
        if ed(gp.get()).fAutoPaint {
            let mut iOldTiles = [0i16; 9];
            GetForegroundTileValues(g, iCol, iRow, &mut iOldTiles);
            tile(g, iCol, iRow).iForegroundSprite = 0;
            UpdatePathSprite(g, iCol, iRow);

            if ForegroundTileValuesChanged(g, iCol, iRow, &iOldTiles) {
                fNeedUpdate = true;
            }
        }

        if fNeedUpdate {
            updateworldsurface(g);
        }
    } else if ed(gp.get()).edit_mode == 5 {
        //vehicles
        RemoveVehicleFromTile(g, iCol, iRow);
        *iStageDisplay = -1;
    } else if ed(gp.get()).edit_mode == 6 {
        //Warps
        RemoveWarpFromTile(g, iCol, iRow);
    } else if ed(gp.get()).edit_mode == 7 {
        //water
        if tile(g, iCol, iRow).iBackgroundWater != 0 {
            tile(g, iCol, iRow).iBackgroundWater = 0;
            updateworldsurface(g);
        }
    } else if ed(gp.get()).edit_mode == 8 {
        //boundary
        if !fromMotion {
            tile(g, iCol, iRow).iVehicleBoundary = 0;
        }
    } else if ed(gp.get()).edit_mode == 9 {
        //stage
        tile(g, iCol, iRow).iType = 0;
        *iStageDisplay = -1;
    }
}

fn copy_vehicle_into_stamp(g: &mut Game, vehicle: &WorldVehicle) {
    let mut gp = g.ptr();
    ed(gp.get()).g_wvVehicleStamp.iDrawSprite = vehicle.iDrawSprite;
    ed(gp.get()).g_wvVehicleStamp.iActionId = vehicle.iActionId;
    ed(gp.get()).g_wvVehicleStamp.iMinMoves = vehicle.iMinMoves;
    ed(gp.get()).g_wvVehicleStamp.iMaxMoves = vehicle.iMaxMoves;
    ed(gp.get()).g_wvVehicleStamp.fSpritePaces = vehicle.fSpritePaces;
    ed(gp.get()).g_wvVehicleStamp.iDrawDirection = vehicle.iDrawDirection;
    ed(gp.get()).g_wvVehicleStamp.iBoundary = vehicle.iBoundary;

    g.ctl(ed(gp.get()).miVehicleSpriteField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iDrawSprite);
    g.ctl(ed(gp.get()).miVehicleStageField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iActionId);
    g.ctl(ed(gp.get()).miVehicleMinMovesField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iMinMoves);
    g.ctl(ed(gp.get()).miVehicleMaxMovesField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iMaxMoves);
    g.ctl(ed(gp.get()).miVehiclePacesField).set_current_value((if ed(gp.get()).g_wvVehicleStamp.fSpritePaces { 1 } else { 0 }) != 0);
    g.ctl(ed(gp.get()).miVehicleDirectionField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iDrawDirection);
    g.ctl(ed(gp.get()).miVehicleBoundaryField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iBoundary);
}

pub fn editor_edit(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;
        ed(gp.get()).g_musiccategorydisplaytimer = 0;

        let mut view_repeat_direction: i16 = -1;
        let mut view_repeat_timer: i16 = 0;

        let mut fExiting = false;
        let mut fSelectedYes = false;

        let mut iStageDisplay: i16 = -1;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            if fExiting {
                //handle messages
                while poll_event(g, &mut ed(gp.get()).event) != 0 {
                    if event_type(g) == T_KEYDOWN {
                        let key = key_sym(g);

                        if key == SDLK_LEFT {
                            fSelectedYes = true;
                        } else if key == SDLK_RIGHT {
                            fSelectedYes = false;
                        } else if key_sym(g) == SDLK_KP_ENTER || key_sym(g) == SDLK_RETURN {
                            if fSelectedYes {
                                return EDITOR_QUIT;
                            }

                            fExiting = false;
                        }
                    }
                }
            } else {
                //handle messages
                while poll_event(g, &mut ed(gp.get()).event) != 0 {
                    match event_type(g) {
                        T_QUIT => {
                            done = true;
                        }

                        T_KEYDOWN => {
                            let keystate = crate::services::services(gp.get()).keyboard.state().as_ptr();
                            if key_sym(g) == SDLK_ESCAPE {
                                if ed(gp.get()).g_musiccategorydisplaytimer > 0 {
                                    ed(gp.get()).g_musiccategorydisplaytimer = 0;
                                } else if ed(gp.get()).edit_mode != 0 {
                                    ed(gp.get()).edit_mode = 0;
                                    ed(gp.get()).set_tile = 0;
                                } else {
                                    fSelectedYes = false;
                                    fExiting = true;
                                }
                            }

                            if ed(gp.get()).event.key_mod() & (KMOD_LALT | KMOD_RALT) != 0 {
                                if key_sym(g) == SDLK_RETURN {
                                    ed(gp.get()).g_fFullScreen = !ed(gp.get()).g_fFullScreen;
                                    gfx_changefullscreen(g, ed(gp.get()).g_fFullScreen);
                                    g.blitdest = g.screen;
                                }
                            }

                            if key_sym(g) == SDLK_INSERT {
                                takescreenshot(g);
                            }

                            if key_sym(g) == SDLK_1 {
                                return EDITOR_WATER;
                            }

                            if key_sym(g) == SDLK_2 {
                                return EDITOR_BACKGROUND;
                            }

                            if key_sym(g) == SDLK_3 {
                                return EDITOR_STAGEFOREGROUND;
                            }

                            if key_sym(g) == SDLK_4 {
                                return EDITOR_PATHSPRITE;
                            }

                            if key_sym(g) == SDLK_5 {
                                return EDITOR_STRUCTUREFOREGROUND;
                            }

                            if key_sym(g) == SDLK_6 {
                                return EDITOR_BRIDGES;
                            }

                            if key_sym(g) == SDLK_t {
                                return EDITOR_TYPE;
                            }

                            if key_sym(g) == SDLK_e {
                                return EDITOR_STAGE;
                            }

                            if key_sym(g) == SDLK_p {
                                return EDITOR_PATH;
                            }

                            if key_sym(g) == SDLK_v {
                                return EDITOR_VEHICLES;
                            }

                            if key_sym(g) == SDLK_SPACE {
                                ed(gp.get()).g_fShowStagePreviews = !ed(gp.get()).g_fShowStagePreviews;
                                if ed(gp.get()).g_fShowStagePreviews {
                                    SetDisplayMessage(g, 60, "Stage Previews", "Preview popups", "have been", "enabled.");
                                } else {
                                    SetDisplayMessage(g, 60, "Stage Previews", "Preview popups", "have been", "disabled.");
                                }
                            }

                            if ed(gp.get()).edit_mode == 5 && key_sym(g) == SDLK_c {
                                let iButtonX: i16 = (ed(gp.get()).mouse_x - ed(gp.get()).draw_offset_x) as i16;
                                let iButtonY: i16 = (ed(gp.get()).mouse_y - ed(gp.get()).draw_offset_y) as i16;
                                let iCol: i16 = (iButtonX as i32 / TILESIZE + ed(gp.get()).draw_offset_col) as i16;
                                let iRow: i16 = (iButtonY as i32 / TILESIZE + ed(gp.get()).draw_offset_row) as i16;

                                for vehicle in ed(gp.get()).vehiclelist.iter() {
                                    if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                                        copy_vehicle_into_stamp(g, vehicle);
                                        return EDITOR_VEHICLES;
                                    }
                                }
                            }

                            if key_sym(g) == SDLK_w {
                                return EDITOR_WARP;
                            }

                            if key_sym(g) == SDLK_i {
                                return EDITOR_START_ITEMS;
                            }

                            if key_sym(g) == SDLK_b {
                                return EDITOR_BOUNDARY;
                            }

                            if key_sym(g) == SDLK_a {
                                ed(gp.get()).fAutoPaint = !ed(gp.get()).fAutoPaint;
                            }

                            if key_sym(g) == SDLK_r {
                                if ed(gp.get()).g_musiccategorydisplaytimer > 0 {
                                    g.g_worldmap.iMusicCategory.post_increment();
                                }

                                ed(gp.get()).g_musiccategorydisplaytimer = 90;
                            }

                            if key_sym(g) == SDLK_s {
                                if check_key(gp.get(), keystate, SDLK_LSHIFT) || check_key(gp.get(), keystate, SDLK_RSHIFT) {
                                    return SAVE_AS;
                                }

                                return SAVE;
                            }

                            if key_sym(g) == SDLK_f {
                                if check_key(gp.get(), keystate, SDLK_LSHIFT) || check_key(gp.get(), keystate, SDLK_RSHIFT) || ed(gp.get()).findstring.is_empty() {
                                    return FIND;
                                }

                                findcurrentstring(g);
                            }

                            if key_sym(g) == SDLK_DELETE && (check_key(gp.get(), keystate, SDLK_LCTRL) || check_key(gp.get(), keystate, SDLK_RCTRL)) {
                                return CLEAR_WORLD;
                            }

                            if key_sym(g) == SDLK_n {
                                return NEW_WORLD;
                            }

                            if key_sym(g) == SDLK_k {
                                return RESIZE_WORLD;
                            }

                            if key_sym(g) == SDLK_h || key_sym(g) == SDLK_F1 {
                                return DISPLAY_HELP;
                            }

                            if key_sym(g) == SDLK_UP {
                                if ed(gp.get()).draw_offset_row > 0 {
                                    ed(gp.get()).draw_offset_row -= 1;
                                    updateworldsurface(g);

                                    view_repeat_direction = 0;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym(g) == SDLK_DOWN {
                                if ed(gp.get()).draw_offset_row < ed(gp.get()).iWorldHeight as i32 - 15 {
                                    ed(gp.get()).draw_offset_row += 1;
                                    updateworldsurface(g);

                                    view_repeat_direction = 1;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym(g) == SDLK_LEFT {
                                if ed(gp.get()).draw_offset_col > 0 {
                                    ed(gp.get()).draw_offset_col -= 1;
                                    updateworldsurface(g);

                                    view_repeat_direction = 2;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym(g) == SDLK_RIGHT {
                                if ed(gp.get()).draw_offset_col < ed(gp.get()).iWorldWidth as i32 - 20 {
                                    ed(gp.get()).draw_offset_col += 1;
                                    updateworldsurface(g);

                                    view_repeat_direction = 3;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            }

                            if key_sym(g) == SDLK_PAGEUP {
                                g.game_values.worldindex -= 1;
                                if g.game_values.worldindex < 0 {
                                    g.game_values.worldindex = g.worldlist.count() as i16 - 1;
                                }

                                g.worldlist.prev();

                                loadcurrentworld(g);

                                ed(gp.get()).iOldStageId = -1;
                            }

                            if key_sym(g) == SDLK_PAGEDOWN {
                                g.game_values.worldindex += 1;
                                if g.game_values.worldindex as usize >= g.worldlist.count() {
                                    g.game_values.worldindex = 0;
                                }

                                g.worldlist.next();

                                loadcurrentworld(g);

                                ed(gp.get()).iOldStageId = -1;
                            }
                        }

                        T_KEYUP => {
                            if key_sym(g) == SDLK_UP || key_sym(g) == SDLK_DOWN || key_sym(g) == SDLK_LEFT || key_sym(g) == SDLK_RIGHT {
                                view_repeat_direction = -1;
                                view_repeat_timer = 0;
                            }
                        }

                        T_MOUSEBUTTONDOWN => {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) - ed(gp.get()).draw_offset_x) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) - ed(gp.get()).draw_offset_y) as i16;
                            let iCol: i16 = (iButtonX as i32 / TILESIZE + ed(gp.get()).draw_offset_col) as i16;
                            let iRow: i16 = (iButtonY as i32 / TILESIZE + ed(gp.get()).draw_offset_row) as i16;

                            if iButtonX >= 0 && iButtonY >= 0 && (iButtonX as i32) < ed(gp.get()).iWorldWidth as i32 * TILESIZE && (iButtonY as i32) < ed(gp.get()).iWorldHeight as i32 * TILESIZE {
                                if ed(gp.get()).event.button_button() == BUTTON_LEFT && !ed(gp.get()).ignoreclick {
                                    paint_left(g, iCol, iRow, false);
                                } else if ed(gp.get()).event.button_button() == BUTTON_RIGHT {
                                    paint_right(g, iCol, iRow, false, &mut iStageDisplay);
                                }
                            }
                        }
                        //Painting tiles with mouse movement
                        T_MOUSEMOTION => {
                            bound_mouse_motion_coords(g);
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.motion_x()) - ed(gp.get()).draw_offset_x) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.motion_y()) - ed(gp.get()).draw_offset_y) as i16;
                            let iCol: i16 = ((iButtonX as i32 >> 5) + ed(gp.get()).draw_offset_col) as i16;
                            let iRow: i16 = ((iButtonY as i32 >> 5) + ed(gp.get()).draw_offset_row) as i16;

                            if iButtonX >= 0 && iButtonY >= 0 && (iButtonX as i32) < ed(gp.get()).iWorldWidth as i32 * TILESIZE && (iButtonY as i32) < ed(gp.get()).iWorldHeight as i32 * TILESIZE {
                                if ed(gp.get()).event.motion_state() == SDL_BUTTON_LMASK && !ed(gp.get()).ignoreclick {
                                    paint_left(g, iCol, iRow, true);
                                } else if ed(gp.get()).event.motion_state() == SDL_BUTTON_RMASK {
                                    paint_right(g, iCol, iRow, true, &mut iStageDisplay);
                                }
                            }

                            //Scan to see if we are mousing over a stage
                            if ed(gp.get()).edit_mode == 5 {
                                iStageDisplay = -1;

                                for vehicle in ed(gp.get()).vehiclelist.iter() {
                                    if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                                        iStageDisplay = vehicle.iActionId;
                                        break;
                                    }
                                }
                            } else if ed(gp.get()).edit_mode == 9 {
                                iStageDisplay = -1;
                                if iCol >= 0 && iRow >= 0 && iCol < ed(gp.get()).iWorldWidth && iRow < ed(gp.get()).iWorldHeight {
                                    let iType: i16 = tile(g, iCol, iRow).iType - 6;
                                    if iType >= 0 {
                                        iStageDisplay = iType;
                                    }
                                }
                            }
                        }

                        T_MOUSEBUTTONUP => {
                            if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                                ed(gp.get()).ignoreclick = false;
                            }
                        }

                        _ => {}
                    }
                }

                //Allow auto-scrolling of world when the arrow keys are held down
                if view_repeat_direction >= 0 && view_repeat_timer > 0 {
                    view_repeat_timer -= 1;
                    if view_repeat_timer <= 0 {
                        view_repeat_timer = 5;

                        if view_repeat_direction == 0 && ed(gp.get()).draw_offset_row > 0 {
                            ed(gp.get()).draw_offset_row -= 1;
                            updateworldsurface(g);
                        } else if view_repeat_direction == 1 && ed(gp.get()).draw_offset_row < ed(gp.get()).iWorldHeight as i32 - 15 {
                            ed(gp.get()).draw_offset_row += 1;
                            updateworldsurface(g);
                        } else if view_repeat_direction == 2 && ed(gp.get()).draw_offset_col > 0 {
                            ed(gp.get()).draw_offset_col -= 1;
                            updateworldsurface(g);
                        } else if view_repeat_direction == 3 && ed(gp.get()).draw_offset_col < ed(gp.get()).iWorldWidth as i32 - 20 {
                            ed(gp.get()).draw_offset_col += 1;
                            updateworldsurface(g);
                        }
                    }
                }
            }

            drawmap(g, false, TILESIZE as i16);

            //Ask if you are sure you want to exit
            if fExiting {
                gp.rm.sprite(ed(gp.get()).spr_dialog).draw_src(g, 224, 176, &r(0, 0, 192, 128));
                gp.rm.menu_font_large.draw_centered(g, 320, 195, "Exit");
                gp.rm.menu_font_large.draw_centered(g, 320, 220, "Are You Sure?");
                gp.rm.menu_font_large.draw_centered(g, 282, 254, "Yes");
                gp.rm.menu_font_large.draw_centered(g, 356, 254, "No");

                gp.rm.sprite(ed(gp.get()).spr_dialog).draw_src(g, if fSelectedYes { 250 } else { 326 }, 250, &r(192, 0, 64, 32));
            } else {
                //Draw Paths
                if ed(gp.get()).edit_mode == 2 {
                    let mut iRow = ed(gp.get()).draw_offset_row as i16;
                    while (iRow as i32) < ed(gp.get()).draw_offset_row + 15 && iRow < ed(gp.get()).iWorldHeight {
                        let mut iCol = ed(gp.get()).draw_offset_col as i16;
                        while iCol as i32 <= ed(gp.get()).draw_offset_col + 20 && iCol < ed(gp.get()).iWorldWidth {
                            let iConnection: i16 = tile(g, iCol, iRow).iConnectionType;

                            if iConnection > 0 {
                                ed(gp.get()).spr_path.draw_src(
                                    g,
                                    (iCol as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x,
                                    (iRow as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y,
                                    &r((iConnection as i32 - 1) << 5, 0, TILESIZE, TILESIZE),
                                );
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }
                } else if ed(gp.get()).edit_mode == 6 {
                    //draw warps
                    for warp in ed(gp.get()).warplist.iter() {
                        let mut ix: i16;
                        let mut iy: i16;

                        if warp.posA.x >= 0 {
                            ix = ((warp.posA.x as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x) as i16;
                            iy = ((warp.posA.y as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y) as i16;

                            ed(gp.get()).spr_warps[0].draw_src(g, ix as i32, iy as i32, &r((warp.id as i32) << 5, 0, 32, 32));
                        }

                        if warp.posB.x >= 0 {
                            ix = ((warp.posB.x as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x) as i16;
                            iy = ((warp.posB.y as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y) as i16;

                            ed(gp.get()).spr_warps[0].draw_src(g, ix as i32, iy as i32, &r((warp.id as i32) << 5, 0, 32, 32));
                        }
                    }
                } else if ed(gp.get()).edit_mode == 8 {
                    //draw boundaries
                    let color = (*g.blitdest).map_rgb(255, 0, 255);
                    let mut iRow = ed(gp.get()).draw_offset_row as i16;
                    while (iRow as i32) < ed(gp.get()).draw_offset_row + 15 && iRow < ed(gp.get()).iWorldHeight {
                        let mut iCol = ed(gp.get()).draw_offset_col as i16;
                        while iCol as i32 <= ed(gp.get()).draw_offset_col + 20 && iCol < ed(gp.get()).iWorldWidth {
                            let iBoundary: i16 = tile(g, iCol, iRow).iVehicleBoundary - 1;

                            if iBoundary >= 0 {
                                let ix: i16 = ((iCol as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x) as i16;
                                let iy: i16 = ((iRow as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y) as i16;
                                let mut rr = r(ix as i32, iy as i32, 32, 32);
                                crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

                                gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(((iBoundary % 10) as i32) << 5, ((iBoundary / 10) as i32) << 5, 32, 32));
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }
                } else if ed(gp.get()).edit_mode == 9 {
                    //draw stages
                    let color = (*g.blitdest).map_rgb(0, 0, 255);
                    let mut iRow = ed(gp.get()).draw_offset_row as i16;
                    while (iRow as i32) < ed(gp.get()).draw_offset_row + 15 && iRow < ed(gp.get()).iWorldHeight {
                        let mut iCol = ed(gp.get()).draw_offset_col as i16;
                        while iCol as i32 <= ed(gp.get()).draw_offset_col + 20 && iCol < ed(gp.get()).iWorldWidth {
                            let iType: i16 = tile(g, iCol, iRow).iType - 6;

                            if iType >= 0 {
                                let ix: i16 = ((iCol as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x) as i16;
                                let iy: i16 = ((iRow as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y) as i16;
                                let mut rr = r(ix as i32, iy as i32, 32, 32);
                                crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

                                gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(((iType % 10) as i32) << 5, ((iType / 10) as i32) << 5, 32, 32));
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }

                    if iStageDisplay >= 0 && ed(gp.get()).g_fShowStagePreviews {
                        DisplayStageDetails(g, false, iStageDisplay, ed(gp.get()).mouse_x as i16, ed(gp.get()).mouse_y as i16);
                    }
                }

                if ed(gp.get()).edit_mode == 5 || ed(gp.get()).edit_mode == 8 {
                    //draw vehicles
                    let color = (*g.blitdest).map_rgb(0, 0, 128);
                    for vehicle in ed(gp.get()).vehiclelist.iter() {
                        let ix: i16 = ((vehicle.currentTile.x as i32 - ed(gp.get()).draw_offset_col) * TILESIZE + ed(gp.get()).draw_offset_x) as i16;
                        let iy: i16 = ((vehicle.currentTile.y as i32 - ed(gp.get()).draw_offset_row) * TILESIZE + ed(gp.get()).draw_offset_y) as i16;

                        let mut rr = r(ix as i32, iy as i32, 32, 32);
                        crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

                        gp.rm.sprite(gp.rm.spr_worldvehicle[0]).draw_src(g, ix as i32, iy as i32, &r((vehicle.iDrawDirection as i32) << 5, (vehicle.iDrawSprite as i32) << 5, 32, 32));

                        if ed(gp.get()).edit_mode == 5 {
                            gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(((vehicle.iActionId % 10) as i32) << 5, ((vehicle.iActionId / 10) as i32) << 5, 32, 32));
                        }

                        if ed(gp.get()).edit_mode == 8 {
                            let iBoundary: i16 = vehicle.iBoundary - 1;
                            if iBoundary == -1 {
                                gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(288, 288, 32, 32));
                            } else {
                                gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(((iBoundary % 10) as i32) << 5, ((iBoundary / 10) as i32) << 5, 32, 32));
                            }
                        }
                    }

                    if ed(gp.get()).edit_mode == 5 {
                        if iStageDisplay >= 0 {
                            DisplayStageDetails(g, false, iStageDisplay, ed(gp.get()).mouse_x as i16, ed(gp.get()).mouse_y as i16);
                        }
                    }
                }

                gp.rm.menu_font_small.draw(g, 0, 0, szEditModes[ed(gp.get()).edit_mode as usize]);

                if ed(gp.get()).fAutoPaint {
                    gp.rm.menu_font_small.draw(g, 0, 16, "Auto Paint");
                }

                gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

                ed(gp.get()).g_musiccategorydisplaytimer -= 1;
                if ed(gp.get()).g_musiccategorydisplaytimer > 0 {
                    gp.rm.sprite(ed(gp.get()).spr_dialog).draw_src(g, 224, 176, &r(0, 0, 192, 128));
                    gp.rm.menu_font_small.draw_centered(g, 320, 195, "Music Category");
                    gp.rm.menu_font_large.draw_centered(g, 320, 220, &crate::common::file_list::world_music_category_to_string(g.g_worldmap.iMusicCategory));

                    gp.rm.menu_font_small.draw_centered(g, 320, 255, "Press 'R' Again");
                    gp.rm.menu_font_small.draw_centered(g, 320, 270, "To Change");
                }

                DrawMessage(g);
            }

            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn DrawMessage(g: &mut Game) {
    let mut gp = g.ptr();
    if ed(gp.get()).g_messagedisplaytimer > 0 {
        ed(gp.get()).g_messagedisplaytimer -= 1;

        gp.rm.sprite(ed(gp.get()).spr_dialog).draw_src(g, 224, 176, &r(0, 0, 192, 128));
        gp.rm.menu_font_large.draw_centered(g, 320, 195, &ed(gp.get()).g_szMessageTitle);
        gp.rm.menu_font_large.draw_centered(g, 320, 220, &ed(gp.get()).g_szMessageLine[0]);
        gp.rm.menu_font_large.draw_centered(g, 320, 240, &ed(gp.get()).g_szMessageLine[1]);
        gp.rm.menu_font_large.draw_centered(g, 320, 260, &ed(gp.get()).g_szMessageLine[2]);
    }
}

pub fn GetForegroundTileValues(g: &mut Game, iCol: i16, iRow: i16, iOldTiles: &mut [i16; 9]) {
    let mut gp = g.ptr();
    let mut iIndex: i16 = 0;
    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                iOldTiles[iIndex as usize] = tile(g, iAutoCol, iAutoRow).iForegroundSprite;
                iIndex += 1;
            }
        }
    }

    for i in iIndex..9 {
        iOldTiles[i as usize] = 0;
    }
}

pub fn ForegroundTileValuesChanged(g: &mut Game, iCol: i16, iRow: i16, iOldTiles: &[i16; 9]) -> bool {
    let mut gp = g.ptr();
    let mut iIndex: usize = 0;
    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                let old = iOldTiles[iIndex];
                iIndex += 1;
                if tile(g, iAutoCol, iAutoRow).iForegroundSprite != old {
                    return true;
                }
            }
        }
    }

    false
}

pub fn ReadVehiclesIntoEditor(g: &mut Game) {
    let mut gp = g.ptr();
    for v in ed(gp.get()).vehiclelist.drain(..) {
        v.delete();
    }

    for vehicle in g.g_worldmap.vehicles.iter() {
        let mut vehiclecopy = WorldVehicle::new();

        vehiclecopy.iDrawSprite = vehicle.iDrawSprite;
        vehiclecopy.iActionId = vehicle.iActionId;
        vehiclecopy.currentTile.x = vehicle.currentTile.x;
        vehiclecopy.currentTile.y = vehicle.currentTile.y;
        vehiclecopy.iMinMoves = vehicle.iMinMoves;
        vehiclecopy.iMaxMoves = vehicle.iMaxMoves;
        vehiclecopy.fSpritePaces = vehicle.fSpritePaces;
        vehiclecopy.iDrawDirection = vehicle.iDrawDirection;
        vehiclecopy.iBoundary = vehicle.iBoundary;

        ed(gp.get()).vehiclelist.push(Ptr::new_box(vehiclecopy));
    }
}

pub fn WriteVehiclesIntoWorld(g: &mut Game) {
    let mut gp = g.ptr();
    //Cleanup old vehicles
    g.g_worldmap.vehicles.clear();

    //Insert new vehicles
    g.g_worldmap.vehicles.reserve(ed(gp.get()).vehiclelist.len());

    for vehicle in ed(gp.get()).vehiclelist.iter() {
        let mut vehiclecopy = WorldVehicle::new();
        vehiclecopy.iDrawSprite = vehicle.iDrawSprite;
        vehiclecopy.iActionId = vehicle.iActionId;
        vehiclecopy.currentTile.x = vehicle.currentTile.x;
        vehiclecopy.currentTile.y = vehicle.currentTile.y;
        vehiclecopy.iMinMoves = vehicle.iMinMoves;
        vehiclecopy.iMaxMoves = vehicle.iMaxMoves;
        vehiclecopy.fSpritePaces = vehicle.fSpritePaces;
        vehiclecopy.iDrawDirection = vehicle.iDrawDirection;
        vehiclecopy.iBoundary = vehicle.iBoundary;
        g.g_worldmap.vehicles.push(vehiclecopy);
    }
}

pub fn AddVehicleToTile(g: &mut Game, iCol: i16, iRow: i16, iType: i16) {
    let mut gp = g.ptr();
    let mut newvehicle: Ptr<WorldVehicle> = Ptr::null();
    for vehicle in ed(gp.get()).vehiclelist.iter() {
        if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
            newvehicle = *vehicle;
            break;
        }
    }

    if newvehicle.is_null() {
        newvehicle = Ptr::new_box(WorldVehicle::new());
        newvehicle.currentTile.x = iCol;
        newvehicle.currentTile.y = iRow;
        ed(gp.get()).vehiclelist.push(newvehicle);
    }

    newvehicle.iDrawSprite = ed(gp.get()).g_wvVehicleStamp.iDrawSprite;
    newvehicle.iActionId = ed(gp.get()).g_wvVehicleStamp.iActionId;
    newvehicle.iMinMoves = ed(gp.get()).g_wvVehicleStamp.iMinMoves;
    newvehicle.iMaxMoves = ed(gp.get()).g_wvVehicleStamp.iMaxMoves;
    newvehicle.fSpritePaces = ed(gp.get()).g_wvVehicleStamp.fSpritePaces;
    newvehicle.iDrawDirection = ed(gp.get()).g_wvVehicleStamp.iDrawDirection;
    newvehicle.iBoundary = ed(gp.get()).g_wvVehicleStamp.iBoundary;
}

pub fn RemoveVehicleFromTile(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    for i in 0..ed(g).vehiclelist.len() {
        let vehicle = ed(gp.get()).vehiclelist[i];
        if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
            vehicle.delete();

            ed(gp.get()).vehiclelist.remove(i);

            return;
        }
    }
}

pub fn ReadWarpsIntoEditor(g: &mut Game) {
    let mut gp = g.ptr();
    for w in ed(gp.get()).warplist.drain(..) {
        w.delete();
    }

    for warp in g.g_worldmap.warps.iter() {
        ed(gp.get()).warplist.push(Ptr::new_box(warp.clone()));
    }
}

pub fn WriteWarpsIntoWorld(g: &mut Game) {
    let mut gp = g.ptr();
    //Cleanup old vehicles
    g.g_worldmap.warps.clear();

    //Insert new vehicles
    g.g_worldmap.warps.reserve(ed(gp.get()).warplist.len());

    for warp in ed(gp.get()).warplist.iter() {
        g.g_worldmap.warps.push((**warp).clone());
    }
}

pub fn AddWarpToTile(g: &mut Game, iCol: i16, iRow: i16, iType: i16) {
    let mut gp = g.ptr();
    let mut newwarp: Ptr<WorldWarp> = Ptr::null();
    for warp in ed(gp.get()).warplist.iter() {
        if warp.id == iType {
            newwarp = *warp;
            break;
        }
    }

    if newwarp.is_null() {
        newwarp = Ptr::new_box(WorldWarp::new(iType, Vec2s::new(iCol, iRow), Vec2s::new(-1, -1)));
        ed(gp.get()).warplist.push(newwarp);
    } else if newwarp.posA.x == -1 {
        newwarp.posA = Vec2s::new(iCol, iRow);
    } else if newwarp.posA.x != iCol || newwarp.posA.y != iRow {
        newwarp.posB = Vec2s::new(iCol, iRow);
    }
}

pub fn RemoveWarpFromTile(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    let mut i = 0;
    while i < ed(gp.get()).warplist.len() {
        let mut warp = ed(gp.get()).warplist[i];
        if warp.posA.x == iCol && warp.posA.y == iRow {
            if warp.posB.x == -1 && warp.posB.y == -1 {
                warp.delete();

                ed(gp.get()).warplist.remove(i);

                return;
            } else {
                warp.posA = Vec2s::new(-1, -1);
            }
        } else if warp.posB.x == iCol && warp.posB.y == iRow {
            if warp.posA.x == -1 && warp.posA.y == -1 {
                warp.delete();

                ed(gp.get()).warplist.remove(i);

                return;
            } else {
                warp.posB = Vec2s::new(-1, -1);
            }
        }

        i += 1;
    }
}

pub fn UpdatePathSprite(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                AutoSetPathSprite(g, iAutoCol, iAutoRow);
            }
        }
    }
}

pub fn AutoSetPathSprite(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    let iPathTypes: [i16; 16] = [6, 4, 3, 5, 6, 4, 1, 5, 6, 2, 3, 5, 6, 4, 3, 5];

    let mut iPath: i16 = 0;
    let mut iNeighborIndex: i16 = 0;

    let mut iForegroundSprite: i16 = tile(g, iCol, iRow).iForegroundSprite;
    let iForegroundStyle: i16 = iForegroundSprite / WORLD_PATH_SPRITE_SET_SIZE;

    if iForegroundSprite == 0 || iForegroundSprite >= WORLD_FOREGROUND_STAGE_OFFSET {
        return;
    }

    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoCol == iCol && iAutoRow == iRow {
                continue;
            }

            if (iAutoCol == iCol && iAutoRow != iRow) || (iAutoCol != iCol && iAutoRow == iRow) {
                if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                    iForegroundSprite = tile(g, iAutoCol, iAutoRow).iForegroundSprite;

                    if (iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 3)
                        || (iForegroundSprite >= WORLD_FOREGROUND_STAGE_OFFSET && iForegroundSprite <= WORLD_FOREGROUND_STAGE_OFFSET + 399)
                        || (iForegroundSprite >= WORLD_START_SPRITE_OFFSET && iForegroundSprite <= WORLD_START_SPRITE_OFFSET + 1)
                    {
                        iPath += 1 << iNeighborIndex;
                    } else if iForegroundSprite >= 0 && iForegroundSprite < WORLD_FOREGROUND_STAGE_OFFSET {
                        let iPathSprite: i16 = iForegroundSprite % WORLD_PATH_SPRITE_SET_SIZE;

                        if iPathSprite >= 1 && iPathSprite <= 18 {
                            iPath += 1 << iNeighborIndex;
                        }
                    }
                }

                iNeighborIndex += 1;
            }
        }
    }

    //#1 == -  2 == |  3 == -o  4 == !  5 == -`  6 == o
    tile(g, iCol, iRow).iForegroundSprite = AdjustForeground(g, iPathTypes[iPath as usize] + iForegroundStyle * WORLD_PATH_SPRITE_SET_SIZE, iCol, iRow);
}

//Convert foreground sprite to match the background sprite
pub fn AdjustForeground(g: &mut Game, mut iSprite: i16, iCol: i16, iRow: i16) -> i16 {
    if iSprite >= WORLD_FOREGROUND_STAGE_OFFSET {
        return iSprite;
    }

    let iBackgroundSprite: i16 = tile(g, iCol, iRow).iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE;

    let iPathStyle: i16 = iSprite / WORLD_PATH_SPRITE_SET_SIZE;
    iSprite %= WORLD_PATH_SPRITE_SET_SIZE;

    //Convert already adjusted sprites back to their "base" sprite
    if iSprite >= 11 && iSprite <= 18 {
        iSprite = 2 - (iSprite % 2);
    } else if iSprite >= 7 && iSprite <= 10 {
        iSprite -= 4;
    }

    if iBackgroundSprite == 1 {
        return iSprite + iPathStyle * WORLD_PATH_SPRITE_SET_SIZE;
    }

    let b = iBackgroundSprite;
    if iSprite == 2 && (b == 12 || b == 20 || b == 23 || b == 24 || b == 27 || b == 32 || b == 36 || b == 37 || b == 40) {
        iSprite = 14;
    } else if iSprite == 2 && (b == 13 || b == 21 || b == 22 || b == 25 || b == 26 || b == 33 || b == 34 || b == 35 || b == 41) {
        iSprite = 16;
    } else if iSprite == 1 && (b == 14 || b == 18 || b == 19 || b == 26 || b == 27 || b == 31 || b == 34 || b == 37 || b == 39) {
        iSprite = 13;
    } else if iSprite == 1 && (b == 15 || b == 16 || b == 17 || b == 24 || b == 25 || b == 30 || b == 35 || b == 36 || b == 38) {
        iSprite = 15;
    } else if iSprite == 2 && (b == 28 || b == 38 || b == 39 || b == 43) {
        iSprite = 12;
    } else if iSprite == 1 && (b == 28 || b == 40 || b == 41 || b == 42) {
        iSprite = 11;
    } else if iSprite == 1 || iSprite == 2 {
        iSprite += 16;
    } else if iSprite >= 3 && iSprite <= 6 {
        iSprite += 4;
    }

    iSprite + iPathStyle * WORLD_PATH_SPRITE_SET_SIZE
}

pub fn UpdatePath(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                AutoSetPath(g, iAutoCol, iAutoRow);
            }
        }
    }
}

pub fn AutoSetPath(g: &mut Game, iCol: i16, iRow: i16) {
    let mut gp = g.ptr();
    let iPathTypes: [i16; 16] = [11, 1, 2, 3, 2, 4, 2, 8, 1, 1, 6, 7, 5, 9, 10, 11];

    let mut iPath: i16 = 0;
    let mut iNeighborIndex: i16 = 0;

    if tile(g, iCol, iRow).iConnectionType == 0 {
        return;
    }

    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoCol == iCol && iAutoRow == iRow {
                continue;
            }

            if (iAutoCol == iCol && iAutoRow != iRow) || (iAutoCol != iCol && iAutoRow == iRow) {
                if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                    if tile(g, iAutoCol, iAutoRow).iConnectionType > 0 {
                        iPath += 1 << iNeighborIndex;
                    }
                }

                iNeighborIndex += 1;
            }
        }
    }

    //#1 == |  2 == -  3 == -!  4 == L  5 == ,-  6 == -,
    //#7 == -|  8 == -`-  9 == |-  10 == -,-  11 == +

    let mut iPathType: i16 = iPathTypes[iPath as usize];
    let iForegroundSprite: i16 = tile(g, iCol, iRow).iForegroundSprite;

    if iPathType == 2 && iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 1 {
        iPathType = iForegroundSprite - WORLD_BRIDGE_SPRITE_OFFSET + 12;
    } else if iPathType == 1 && iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET + 2 && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 3 {
        iPathType = iForegroundSprite - WORLD_BRIDGE_SPRITE_OFFSET + 12;
    }

    tile(g, iCol, iRow).iConnectionType = iPathType;
}

pub fn UpdateForeground(g: &mut Game, iCol: i16, iRow: i16) -> bool {
    let iForeground = tile(g, iCol, iRow).iForegroundSprite;
    let iNewForeground: i16 = AdjustForeground(g, iForeground, iCol, iRow);

    if tile(g, iCol, iRow).iForegroundSprite != iNewForeground {
        tile(g, iCol, iRow).iForegroundSprite = iNewForeground;
        return true;
    }

    false
}

pub fn UpdateCoastline(g: &mut Game, iCol: i16, iRow: i16) -> bool {
    let mut gp = g.ptr();
    let iStartCol: i16 = if iCol == 0 { 0 } else { iCol - 1 };
    let iEndCol: i16 = if iCol == ed(gp.get()).iWorldWidth - 1 { ed(gp.get()).iWorldWidth - 1 } else { iCol + 1 };

    let iStartRow: i16 = if iRow == 0 { 0 } else { iRow - 1 };
    let iEndRow: i16 = if iRow == ed(gp.get()).iWorldHeight - 1 { ed(gp.get()).iWorldHeight - 1 } else { iRow + 1 };

    let mut fRet = false;
    for iAutoRow in iStartRow..=iEndRow {
        for iAutoCol in iStartCol..=iEndCol {
            fRet |= AutoSetTile(g, iAutoCol, iAutoRow);
            fRet |= UpdateForeground(g, iAutoCol, iAutoRow);
        }
    }

    fRet
}

pub fn AutoSetTile(g: &mut Game, iCol: i16, iRow: i16) -> bool {
    let mut gp = g.ptr();
    //Don't need to do anything if this tile is solid
    if tile(g, iCol, iRow).iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE == 1 {
        return false;
    }

    let mut iTile = [false; 8];
    let mut iNeighborIndex: usize = 0;

    let mut iNeighborStyle = [0i16; 10];

    for iAutoRow in (iRow - 1)..=(iRow + 1) {
        for iAutoCol in (iCol - 1)..=(iCol + 1) {
            if iAutoCol == iCol && iAutoRow == iRow {
                continue;
            }

            if iAutoRow >= 0 && iAutoRow < ed(gp.get()).iWorldHeight && iAutoCol >= 0 && iAutoCol < ed(gp.get()).iWorldWidth {
                let iBackgroundSprite: i16 = tile(g, iAutoCol, iAutoRow).iBackgroundSprite;

                if iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE == 1 {
                    iTile[iNeighborIndex] = true;
                    iNeighborStyle[(iBackgroundSprite / WORLD_BACKGROUND_SPRITE_SET_SIZE) as usize] += 1;
                }
            }

            iNeighborIndex += 1;
        }
    }

    let mut iMaxStyle: i16 = 0;
    let mut iTileStyleOffset: i16 = 0;
    for iStyle in 0..10i16 {
        if iNeighborStyle[iStyle as usize] > iMaxStyle {
            iMaxStyle = iNeighborStyle[iStyle as usize];
            iTileStyleOffset = iStyle * WORLD_BACKGROUND_SPRITE_SET_SIZE;
        }
    }

    let t = &iTile;
    let iNewTile: i16 = if t[0] && !t[1] && t[2] && !t[3] && !t[4] && t[5] && !t[6] && t[7] {
        iTileStyleOffset + 44
    } else if t[0] && !t[1] && !t[3] && t[4] && t[5] && !t[6] {
        iTileStyleOffset + 30
    } else if !t[1] && t[2] && t[3] && !t[4] && !t[6] && t[7] {
        iTileStyleOffset + 31
    } else if t[1] && !t[3] && !t[4] && t[5] && !t[6] && t[7] {
        iTileStyleOffset + 32
    } else if t[0] && !t[1] && t[2] && !t[3] && !t[4] && t[6] {
        iTileStyleOffset + 33
    } else if !t[1] && t[2] && t[3] && !t[4] && t[6] {
        iTileStyleOffset + 34
    } else if t[0] && !t[1] && !t[3] && t[4] && t[6] {
        iTileStyleOffset + 35
    } else if t[1] && !t[3] && t[4] && t[5] && !t[6] {
        iTileStyleOffset + 36
    } else if t[1] && t[3] && !t[4] && !t[6] && t[7] {
        iTileStyleOffset + 37
    } else if t[0] && !t[1] && t[2] && !t[4] && t[7] {
        iTileStyleOffset + 45
    } else if t[2] && !t[4] && t[5] && !t[6] && t[7] {
        iTileStyleOffset + 46
    } else if t[0] && !t[3] && t[5] && !t[6] && t[7] {
        iTileStyleOffset + 47
    } else if t[0] && !t[1] && t[2] && !t[3] && t[5] {
        iTileStyleOffset + 48
    } else if t[1] && t[3] && t[4] && t[6] {
        iTileStyleOffset + 28
    } else if t[1] && !t[3] && t[4] && t[6] {
        iTileStyleOffset + 38
    } else if t[1] && t[3] && !t[4] && t[6] {
        iTileStyleOffset + 39
    } else if t[1] && t[3] && t[4] && !t[6] {
        iTileStyleOffset + 40
    } else if !t[1] && t[3] && t[4] && t[6] {
        iTileStyleOffset + 41
    } else if !t[1] && t[3] && t[4] && !t[6] {
        iTileStyleOffset + 42
    } else if t[1] && !t[3] && !t[4] && t[6] {
        iTileStyleOffset + 43
    } else if t[0] && !t[1] && t[4] {
        iTileStyleOffset + 16
    } else if t[4] && t[5] && !t[6] {
        iTileStyleOffset + 17
    } else if t[3] && !t[6] && t[7] {
        iTileStyleOffset + 18
    } else if !t[1] && t[2] && t[3] {
        iTileStyleOffset + 19
    } else if t[1] && !t[4] && t[7] {
        iTileStyleOffset + 20
    } else if t[2] && !t[4] && t[6] {
        iTileStyleOffset + 21
    } else if t[0] && !t[3] && t[6] {
        iTileStyleOffset + 22
    } else if t[1] && !t[3] && t[5] {
        iTileStyleOffset + 23
    } else if t[0] && !t[1] && t[2] {
        iTileStyleOffset + 6
    } else if t[5] && !t[6] && t[7] {
        iTileStyleOffset + 7
    } else if t[0] && !t[3] && t[5] {
        iTileStyleOffset + 8
    } else if t[2] && !t[4] && t[7] {
        iTileStyleOffset + 9
    } else if t[1] && t[4] {
        iTileStyleOffset + 24
    } else if t[4] && t[6] {
        iTileStyleOffset + 25
    } else if t[3] && t[6] {
        iTileStyleOffset + 26
    } else if t[1] && t[3] {
        iTileStyleOffset + 27
    } else if t[2] && t[5] {
        iTileStyleOffset + 10
    } else if t[0] && t[7] {
        iTileStyleOffset + 11
    } else if t[1] {
        iTileStyleOffset + 12
    } else if t[6] {
        iTileStyleOffset + 13
    } else if t[3] {
        iTileStyleOffset + 14
    } else if t[4] {
        iTileStyleOffset + 15
    } else if t[2] {
        iTileStyleOffset + 2
    } else if t[0] {
        iTileStyleOffset + 3
    } else if t[5] {
        iTileStyleOffset + 4
    } else if t[7] {
        iTileStyleOffset + 5
    } else {
        iTileStyleOffset + 0
    };

    if tile(g, iCol, iRow).iBackgroundSprite != iNewTile {
        tile(g, iCol, iRow).iBackgroundSprite = iNewTile;
        return true;
    }

    false
}

pub fn updateworldsurface(g: &mut Game) {
        let mut gp = g.ptr();
    gp.get().g_worldmap.draw_map_to_surface(g, -1, true, ed(gp.get()).sMapSurface, ed(gp.get()).draw_offset_col as i16, ed(gp.get()).draw_offset_row as i16, 0);
}

pub fn drawmap(g: &mut Game, _fScreenshot: bool, _iBlockSize: i16) {
    let mut gp = g.ptr();
    unsafe {
        if ed(gp.get()).fNeedBlackBackground {
            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);
        }

        crate::common::gfx::blit::upper_blit(ed(gp.get()).sMapSurface, &ed(gp.get()).rectSrcSurface, g.blitdest, &mut ed(gp.get()).rectDstSurface);
    }
}

pub fn editor_warp(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    let mut done = false;

    while !done {
        let framestart = crate::services::ticks(gp.get()) as i32;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => {
                    done = true;
                }

                T_KEYDOWN => {
                    ed(gp.get()).edit_mode = 6; //change to edit mode using warps
                    return EDITOR_EDIT;
                }

                T_MOUSEBUTTONDOWN => {
                    let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                    let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                    if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                        if iButtonX >= 0 && iButtonX <= 9 && iButtonY == 0 {
                            ed(gp.get()).set_tile = iButtonX as i32;

                            ed(gp.get()).edit_mode = 6; //change to edit mode using warps

                            //The user must release the mouse button before trying to add a tile
                            ed(gp.get()).ignoreclick = true;

                            return EDITOR_EDIT;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16);
        ed(gp.get()).menu_shade.draw(g, 0, 0);

        ed(gp.get()).spr_warps[0].draw_src(g, 0, 0, &r(0, 0, 320, 32));

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

        DrawMessage(g);
        gfx_flipscreen(g);

        frame_wait(g, framestart);
    }

    EDITOR_QUIT
}

fn point_in(x: i16, y: i16, rect: &Rect) -> bool {
    x as i32 >= rect.x && (x as i32) < rect.w + rect.x && y as i32 >= rect.y && (y as i32) < rect.h + rect.y
}

pub fn editor_start_items(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    let mut done = false;

    let mut rStartItemDst = [r(0, 0, 0, 0); (NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize];

    let mut rPickedItemDst = [r(0, 0, 0, 0); 32];

    let mut iColCount: i16 = 0;
    let mut iRowCount: i16 = 0;
    for iItem in 0..(NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize {
        rStartItemDst[iItem].x = 16 + iColCount as i32 * 48;
        rStartItemDst[iItem].y = 16 + iRowCount as i32 * 48;
        rStartItemDst[iItem].w = 32;
        rStartItemDst[iItem].h = 32;

        iColCount += 1;
        if iColCount > 12 {
            iColCount = 0;
            iRowCount += 1;
        }
    }

    let mut iPickedItem: usize = 0;
    for iPickedItemY in 0..4 {
        for iPickedItemX in 0..8 {
            rPickedItemDst[iPickedItem].x = 122 + iPickedItemX * 52;
            rPickedItemDst[iPickedItem].y = 240 + iPickedItemY * 64;
            rPickedItemDst[iPickedItem].w = 32;
            rPickedItemDst[iPickedItem].h = 32;

            iPickedItem += 1;
        }
    }

    while !done {
        let framestart = crate::services::ticks(gp.get()) as i32;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => {
                    done = true;
                }

                T_KEYDOWN => {
                    return EDITOR_EDIT;
                }

                T_MOUSEBUTTONDOWN => {
                    let iButtonX: i16 = bound_to_window_w(ed(gp.get()).event.button_x()) as i16;
                    let iButtonY: i16 = bound_to_window_h(ed(gp.get()).event.button_y()) as i16;

                    if ed(gp.get()).event.button_button() == BUTTON_LEFT || ed(gp.get()).event.button_button() == BUTTON_RIGHT {
                        if g.g_worldmap.iNumInitialBonuses < 32 {
                            for iItem in 0..(NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize {
                                if point_in(iButtonX, iButtonY, &rStartItemDst[iItem]) {
                                    let n = g.g_worldmap.iNumInitialBonuses as usize;
                                    g.g_worldmap.iInitialBonuses[n] = iItem as i16;
                                    g.g_worldmap.iNumInitialBonuses += 1;
                                    break;
                                }
                            }
                        }

                        let mut iRemoveItem: i16 = 0;
                        while iRemoveItem < g.g_worldmap.iNumInitialBonuses {
                            if point_in(iButtonX, iButtonY, &rPickedItemDst[iRemoveItem as usize]) {
                                let mut iAdjust = iRemoveItem;
                                while iAdjust < g.g_worldmap.iNumInitialBonuses - 1 {
                                    g.g_worldmap.iInitialBonuses[iAdjust as usize] = g.g_worldmap.iInitialBonuses[iAdjust as usize + 1];
                                    iAdjust += 1;
                                }

                                g.g_worldmap.iNumInitialBonuses -= 1;

                                break;
                            }
                            iRemoveItem += 1;
                        }
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16);
        ed(gp.get()).menu_shade.draw(g, 0, 0);

        for iItem in 0..NUM_POWERUPS as usize {
            gp.rm.sprite(gp.rm.spr_storedpoweruplarge).draw_src(g, rStartItemDst[iItem].x, rStartItemDst[iItem].y, &r((iItem as i32) << 5, 0, 32, 32));
        }

        for iWorldItem in 0..NUM_WORLD_POWERUPS as usize {
            gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, rStartItemDst[iWorldItem + NUM_POWERUPS as usize].x, rStartItemDst[iWorldItem + NUM_POWERUPS as usize].y, &r((iWorldItem as i32) << 5, 0, 32, 32));
        }

        for iPopup in 0..4i32 {
            gp.rm.sprite(gp.rm.spr_worlditempopup).draw_src(g, 0, 416 - (iPopup << 6), &r(0, 0, 320, 64));
            gp.rm.sprite(gp.rm.spr_worlditempopup).draw_src(g, 320, 416 - (iPopup << 6), &r(192, 0, 320, 64));
        }

        for iPickedItem in 0..g.g_worldmap.iNumInitialBonuses as usize {
            let iPowerup: i16 = g.g_worldmap.iInitialBonuses[iPickedItem];
            if iPowerup as i32 >= NUM_POWERUPS {
                gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, rPickedItemDst[iPickedItem].x, rPickedItemDst[iPickedItem].y, &r((iPowerup as i32 - NUM_POWERUPS) << 5, 0, 32, 32));
            } else {
                gp.rm.sprite(gp.rm.spr_storedpoweruplarge).draw_src(g, rPickedItemDst[iPickedItem].x, rPickedItemDst[iPickedItem].y, &r((iPowerup as i32) << 5, 0, 32, 32));
            }
        }

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

        DrawMessage(g);
        gfx_flipscreen(g);

        frame_wait(g, framestart);
    }

    EDITOR_QUIT
}

pub fn editor_boundary(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 8; //change to edit mode using warps
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                        let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            if iButtonX >= 0 && iButtonX <= 9 && iButtonY >= 0 && iButtonY <= 9 {
                                ed(gp.get()).set_tile = iButtonX as i32 + 10 * iButtonY as i32 + 1;

                                ed(gp.get()).edit_mode = 8; //change to edit mode using warps

                                //The user must release the mouse button before trying to add a tile
                                ed(gp.get()).ignoreclick = true;

                                return EDITOR_EDIT;
                            }
                        }
                    }

                    _ => {}
                }
            }

            drawmap(g, false, TILESIZE as i16);
            ed(gp.get()).menu_shade.draw(g, 0, 0);

            let color = (*g.blitdest).map_rgb(255, 0, 255);
            let mut rr = r(0, 0, 320, 320);
            crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

            gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 0, 0, &r(0, 0, 320, 320));

            gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_type(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    let mut done = false;

    while !done {
        let framestart = crate::services::ticks(gp.get()) as i32;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => {
                    done = true;
                }

                T_KEYDOWN => {
                    ed(gp.get()).edit_mode = 3; //change to edit mode using doors/start
                    return EDITOR_EDIT;
                }

                T_MOUSEBUTTONDOWN => {
                    if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                        let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                        let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                        //Start and doors
                        if iButtonX >= 0 && iButtonX <= 5 && iButtonY == 0 {
                            ed(gp.get()).set_tile = iButtonX as i32;
                        }

                        ed(gp.get()).edit_mode = 3; //change to edit mode using warps

                        //The user must release the mouse button before trying to add a tile
                        ed(gp.get()).ignoreclick = true;

                        return EDITOR_EDIT;
                    }
                }

                _ => {}
            }
        }

        drawmap(g, false, TILESIZE as i16);
        ed(gp.get()).menu_shade.draw(g, 0, 0);

        gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 0, 0, &r(320, 128, 64, 32));
        gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 64, 0, &r(320, 192, 128, 32));

        gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 64, 0, &r(448, 64, 128, 32));

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

        DrawMessage(g);
        gfx_flipscreen(g);

        frame_wait(g, framestart);
    }

    EDITOR_QUIT
}

pub fn editor_water(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 7;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonY == 0 {
                                if iButtonX >= 0 && iButtonX <= 2 {
                                    ed(gp.get()).set_tile = iButtonX as i32 + 4;
                                }
                            }

                            ed(gp.get()).ignoreclick = true;
                            ed(gp.get()).edit_mode = 7;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            for iWater in 0..3i32 {
                gp.rm.sprite(gp.rm.spr_worldbackground[0]).draw_src(g, iWater << 5, 0, &r(512 + (iWater << 7), 0, 32, 32));
            }

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_background(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;
        let mut iPage: i16 = 0;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        let key = key_sym(g);
                        if key >= SDLK_1 && key <= SDLK_2 {
                            iPage = (key - SDLK_1) as i16;
                        } else {
                            ed(gp.get()).edit_mode = 0;
                            return EDITOR_EDIT;
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let mut iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            let iTileStyleOffset: i16 = ((iButtonX / 4) + (iPage * 5)) * WORLD_BACKGROUND_SPRITE_SET_SIZE;

                            iButtonX %= 4;

                            if iButtonX == 0 {
                                if iButtonY == 0 {
                                    ed(gp.get()).set_tile = 0;
                                } else if iButtonY >= 1 && iButtonY < 15 {
                                    ed(gp.get()).set_tile = iButtonY as i32 + 1;
                                }
                            } else if iButtonX == 1 {
                                if iButtonY == 0 {
                                    ed(gp.get()).set_tile = 1;
                                } else if iButtonY >= 1 && iButtonY < 15 {
                                    ed(gp.get()).set_tile = iButtonY as i32 + 15;
                                }
                            } else if iButtonX == 2 {
                                if iButtonY >= 0 && iButtonY < 15 {
                                    ed(gp.get()).set_tile = iButtonY as i32 + 30;
                                }
                            } else if iButtonX == 3 {
                                if iButtonY >= 0 && iButtonY < 15 {
                                    ed(gp.get()).set_tile = iButtonY as i32 + 45;
                                }
                            }

                            ed(gp.get()).set_tile += iTileStyleOffset as i32;

                            ed(gp.get()).ignoreclick = true;
                            ed(gp.get()).edit_mode = 0;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            gp.rm.sprite(gp.rm.spr_worldbackground[0]).draw_src(g, 0, 0, &r(iPage as i32 * 640, 32, 640, 480));

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_stageforeground(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;
        let mut iForegroundScreen: i16 = 0;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        let key = key_sym(g);

                        if key >= SDLK_1 && key <= SDLK_4 {
                            iForegroundScreen = (key - SDLK_1) as i16;
                        } else {
                            ed(gp.get()).edit_mode = 1;
                            return EDITOR_EDIT;
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 10 {
                                if iButtonY >= 0 && iButtonY < 10 {
                                    ed(gp.get()).set_tile = WORLD_FOREGROUND_STAGE_OFFSET as i32 + iButtonY as i32 * 10 + iButtonX as i32 + iForegroundScreen as i32 * 100;

                                    ed(gp.get()).ignoreclick = true;
                                    ed(gp.get()).edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            for iRow in 0..10i32 {
                for iCol in 0..10i32 {
                    gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, iCol << 5, iRow << 5, &r(384, (iForegroundScreen as i32) << 5, 32, 32));
                }
            }

            gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 0, 0, &r(0, 0, 320, 320));

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_bridges(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 1;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 4 {
                                if iButtonY >= 0 && iButtonY < 1 {
                                    ed(gp.get()).set_tile = WORLD_BRIDGE_SPRITE_OFFSET as i32 + iButtonX as i32;

                                    ed(gp.get()).ignoreclick = true;
                                    ed(gp.get()).edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, 0, 0, &r(320, 224, 128, 32));

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_structureforeground(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 1;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonY >= 0 && iButtonY < 15 {
                                if iButtonX >= 0 && iButtonX < 12 {
                                    ed(gp.get()).set_tile = WORLD_FOREGROUND_SPRITE_OFFSET as i32 + iButtonX as i32 + iButtonY as i32 * 12;

                                    ed(gp.get()).ignoreclick = true;
                                    ed(gp.get()).edit_mode = 1;
                                    return EDITOR_EDIT;
                                } else if iButtonX >= 12 && iButtonX < 14 {
                                    ed(gp.get()).set_tile = WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET as i32 + iButtonY as i32 + (iButtonX as i32 - 12) * 15;

                                    ed(gp.get()).ignoreclick = true;
                                    ed(gp.get()).edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            gp.rm.sprite(gp.rm.spr_worldforeground[0]).draw_src(g, 0, 0, &r(0, 0, 416, 480));
            gp.rm.sprite(gp.rm.spr_worldforeground[0]).draw_src(g, 416, 0, &r(512, 0, 32, 480));

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_pathsprite(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 4;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 8 {
                                if iButtonY >= 0 && iButtonY < 6 {
                                    ed(gp.get()).set_tile = iButtonY as i32 + 1 + iButtonX as i32 * WORLD_PATH_SPRITE_SET_SIZE as i32;
                                }
                            }

                            ed(gp.get()).ignoreclick = true;
                            ed(gp.get()).edit_mode = 4;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);

            for iPath in 0..8i32 {
                gp.rm.sprite(gp.rm.spr_worldpaths[0]).draw_src(g, iPath << 5, 0, &r((iPath % 4) * 160, (iPath / 4) * 320, 32, 192));
            }

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

//Display stages over vehicles
//allow setting of stages on vehicles

pub fn editor_vehicles(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    if g.g_worldmap.iNumStages <= 0 {
        SetDisplayMessage(g, 120, "No Stages", "You need to create", "stages before you", "can create vehicles");
        return EDITOR_EDIT;
    }

    ed(gp.get()).mCurrentMenu = EditorMenu::Vehicle;
    current_menu(gp.get()).reset_menu(g);

    let done = false;

    g.ctl(ed(gp.get()).miVehicleStageField).clear();

    for iStage in 0..g.g_worldmap.iNumStages {
        let ts = g.game_values.tour_stop(iStage as usize as usize);
        let szStageName = format!("({}) {}", iStage + 1, ts.szName);

        let item = g.ctl(ed(gp.get()).miVehicleStageField).add(szStageName, iStage);
        item.iconOverride = if ts.iStageType == 1 { 24 } else if ts.iMode >= 1000 { ts.iMode - 975 } else { ts.iMode };
    }

    g.ctl(ed(gp.get()).miVehicleStageField).set_current_value(ed(gp.get()).g_wvVehicleStamp.iActionId);

    while !done {
        let framestart = crate::services::ticks(gp.get()) as i32;

        g.game_values.playerInput.clear_pressed_keys(1);

        let mut code: MenuCodeEnum = MENU_CODE_NONE;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => {
                    ed(gp.get()).edit_mode = 5;
                    g.game_values.playerInput.reset_keys();
                    return EDITOR_EDIT;
                }

                T_KEYDOWN => {
                    if key_sym(g) == SDLK_v {
                        if !current_menu(gp.get()).is_modifying() {
                            ed(gp.get()).edit_mode = 5;
                            g.game_values.playerInput.reset_keys();
                            return EDITOR_EDIT;
                        }
                    }
                }

                T_MOUSEBUTTONDOWN => {
                    let iButtonX: i16 = bound_to_window_w(ed(gp.get()).event.button_x()) as i16;
                    let iButtonY: i16 = bound_to_window_h(ed(gp.get()).event.button_y()) as i16;

                    if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                        code = current_menu(gp.get()).mouse_click(g, iButtonX, iButtonY);
                    }
                }

                _ => {}
            }

            gp.get().game_values.playerInput.update_input(g, ed(gp.get()).event.input(), 1);
        }

        if MENU_CODE_NONE == code {
            code = current_menu(gp.get()).send_input(g, Ptr::from_mut(&mut gp.game_values.playerInput));
        }

        if MENU_CODE_EXIT_APPLICATION == code {
            ed(gp.get()).edit_mode = 5;
            g.game_values.playerInput.reset_keys();
            return EDITOR_EDIT;
        } else if MENU_CODE_VEHICLE_MIN_MOVES_CHANGED == code {
            let iMaxMoves: i16 = g.ctl(ed(gp.get()).miVehicleMaxMovesField).current_value();
            if g.ctl(ed(gp.get()).miVehicleMinMovesField).current_value() > iMaxMoves {
                g.ctl(ed(gp.get()).miVehicleMinMovesField).set_current_value(iMaxMoves);
            }
        } else if MENU_CODE_VEHICLE_MAX_MOVES_CHANGED == code {
            let iMinMoves: i16 = g.ctl(ed(gp.get()).miVehicleMinMovesField).current_value();
            if g.ctl(ed(gp.get()).miVehicleMaxMovesField).current_value() < iMinMoves {
                g.ctl(ed(gp.get()).miVehicleMaxMovesField).set_current_value(iMinMoves);
            }
        } else if MENU_CODE_CREATE_VEHICLE == code {
            ed(gp.get()).edit_mode = 5;
            ed(gp.get()).ignoreclick = true;
            g.game_values.playerInput.reset_keys();
            return EDITOR_EDIT;
        }

        drawmap(g, false, TILESIZE as i16);
        ed(gp.get()).menu_shade.draw(g, 0, 0);

        current_menu(gp.get()).update(g);
        current_menu(gp.get()).draw(g);

        gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

        DrawMessage(g);
        gfx_flipscreen(g);

        frame_wait(g, framestart);
    }

    EDITOR_QUIT
}

pub fn editor_path(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    unsafe {
        let mut done = false;

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        ed(gp.get()).edit_mode = 2;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX <= 15 && iButtonY == 0 {
                                ed(gp.get()).set_tile = iButtonX as i32 + 1;

                                ed(gp.get()).ignoreclick = true;
                                ed(gp.get()).edit_mode = 2;
                                return EDITOR_EDIT;
                            }
                        }
                    }

                    _ => {}
                }
            }

            crate::common::gfx::blit::fill_rect(g.screen, null(), 0x0);
            ed(gp.get()).spr_path.draw_src(g, 0, 0, &r(0, 0, 480, 32));

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn DisplayStageDetails(g: &mut Game, fForce: bool, iStageId: i16, mut iMouseX: i16, mut iMouseY: i16) {
        let mut gp = g.ptr();
    let ts = g.game_values.tour_stop(iStageId as usize as usize);

    //If we're pointing to a new stage or no stage at all
    if iStageId != ed(gp.get()).iOldStageId || fForce {
        if ts.iStageType == 1 {
            ed(gp.get()).sMapThumbnail = gfxSprite::new();
        } else if !ts.pszMapFile.is_empty() {
            ed(gp.get()).sMapThumbnail = gfxSprite::new();

            if g.maplist.findexact(&ts.pszMapFile, false) {
                let file = g.maplist.current_filename().to_string();
                gp.g_map.load_map(g, &file, read_type_preview);
                ed(gp.get()).sMapThumbnail = gp.g_map.create_thumbnail_surface(g, true);
            } else {
                //otherwise show a unknown map icon
                ed(gp.get()).sMapThumbnail = ImageLoader::new(convert_path(g, "gfx/leveleditor/leveleditor_mapnotfound.png")).without_color_key().create(g);
            }
        }
    }

    ed(gp.get()).iOldStageId = iStageId;

    let mut iMode: i16 = ts.iMode;
    if ts.iStageType == 1 {
        iMode = 24;
    } else if ts.iMode >= 1000 {
        iMode = ts.iMode - 975; //Convert mode id from map file to internal id for special stage modes
    }

    //Make sure we're displaying it on the screen
    if iMouseX > 408 {
        iMouseX = 408;
    }

    if (iMode as i32) < GAMEMODE_LAST || (iMode >= 25 && iMode <= 27) {
        if iMouseY > 248 {
            iMouseY = 248;
        }

        let (x, y) = (iMouseX as i32, iMouseY as i32);
        ed(gp.get()).spr_largedialog.draw_src(g, x, y, &r(0, 0, 116, 116));
        ed(gp.get()).spr_largedialog.draw_src(g, x + 116, y, &r(140, 0, 116, 116));
        ed(gp.get()).spr_largedialog.draw_src(g, x, y + 116, &r(0, 108, 116, 116));
        ed(gp.get()).spr_largedialog.draw_src(g, x + 116, y + 116, &r(140, 108, 116, 116));
    } else if iMode == 24 {
        //Make sure we're displaying it on the screen
        if iMouseY > 392 {
            iMouseY = 392;
        }

        let (x, y) = (iMouseX as i32, iMouseY as i32);
        ed(gp.get()).spr_largedialog.draw_src(g, x, y, &r(0, 0, 116, 44));
        ed(gp.get()).spr_largedialog.draw_src(g, x + 116, y, &r(140, 0, 116, 44));
        ed(gp.get()).spr_largedialog.draw_src(g, x, y + 44, &r(0, 180, 116, 44));
        ed(gp.get()).spr_largedialog.draw_src(g, x + 116, y + 44, &r(140, 180, 116, 44));
    }

    let (x, y) = (iMouseX as i32, iMouseY as i32);

    gp.rm.sprite(gp.rm.menu_mode_large).draw_src(g, x + 16, y + 16, &r((iMode as i32) << 5, 0, 32, 32));

    gp.rm.menu_font_small.draw_chop_right(g, x + 52, y + 16, 164, &ts.szName);

    if iMode != 24 {
        let szPrint = format!("Goal: {}", ts.iGoal);
        gp.rm.menu_font_small.draw_chop_right(g, x + 52, y + 34, 164, &szPrint);

        let szPrint = format!("Points: {}", ts.iPoints);
        gp.rm.menu_font_small.draw_chop_right(g, x + 16, y + 176, 100, &szPrint);

        let szPrint = format!("End: {}", if ts.fEndStage { "Yes" } else { "No" });
        gp.rm.menu_font_small.draw_chop_right(g, x + 126, y + 176, 80, &szPrint);

        for iBonus in 0..ts.iNumBonuses as i32 {
            let wsb = &ts.wsbBonuses[iBonus as usize];
            gp.rm.sprite(gp.rm.spr_worlditemsplace).draw_src(g, x + iBonus * 20 + 16, y + 194, &r(wsb.iWinnerPlace as i32 * 20, 0, 20, 20));

            let iBonusIcon: i32 = wsb.iBonus as i32;
            let spr_icon: &gfxSprite = if iBonusIcon < NUM_POWERUPS { &gp.rm.sprite(gp.rm.spr_storedpowerupsmall) } else { &gp.rm.sprite(gp.rm.spr_worlditemssmall) };
            let src = (if iBonusIcon < NUM_POWERUPS { iBonusIcon } else { iBonusIcon - NUM_POWERUPS }) << 4;
            spr_icon.draw_src(g, x + iBonus * 20 + 18, y + 196, &r(src, 0, 16, 16));
        }

        if !ed(gp.get()).sMapThumbnail.get_surface().is_null() {
            let rSrc = r(0, 0, 160, 120);
            let rDst = r(x + 16, y + 52, 160, 120);
            ed(gp.get()).sMapThumbnail.draw_src_to(&rSrc, g.blitdest, &rDst);
        }
    } else {
        let szPrint = format!("Sort: {}", if ts.iBonusType == 0 { "Fixed" } else { "Random" });
        gp.rm.menu_font_small.draw_chop_right(g, x + 52, y + 34, 164, &szPrint);

        let rm = g.rm;
        for iBonus in 0..ts.iNumBonuses as i32 {
            let iBonusIcon: i32 = ts.wsbBonuses[iBonus as usize].iBonus as i32;
            let spr_icon: &gfxSprite;
            let iSrcX: i32;
            let iSrcY: i32;

            if iBonusIcon < NUM_POWERUPS {
                spr_icon = rm.sprite(rm.spr_storedpowerupsmall);
                iSrcX = iBonusIcon << 4;
                iSrcY = 0;
            } else if iBonusIcon < NUM_POWERUPS + NUM_WORLD_POWERUPS {
                spr_icon = rm.sprite(rm.spr_worlditemssmall);
                iSrcX = (iBonusIcon - NUM_POWERUPS) << 4;
                iSrcY = 0;
            } else if iBonusIcon < NUM_POWERUPS + NUM_WORLD_POWERUPS + 10 {
                spr_icon = rm.sprite(rm.spr_worlditemssmall);
                iSrcX = (iBonusIcon - NUM_POWERUPS - NUM_WORLD_POWERUPS) << 4;
                iSrcY = 16;
            } else {
                spr_icon = rm.sprite(rm.spr_worlditemssmall);
                iSrcX = (iBonusIcon - NUM_POWERUPS - NUM_WORLD_POWERUPS - 10) << 4;
                iSrcY = 32;
            }

            spr_icon.draw_src(g, x + iBonus * 20 + 18, y + 52, &r(iSrcX, iSrcY, 16, 16));
        }
    }
}

pub static g_iNumGameModeSettings: [i16; GAMEMODE_LAST as usize] = [2, 2, 3, 4, 3, 10, 9, 6, 2, 1, 3, 5, 3, 3, 0, 22, 6, 4, 3, 4, 4, 3];

pub fn SetBonusString(szString: &mut String, iPlace: i16, mut iItem: i16, iStageType: i16) {
    let mut cType = 'p';

    if iItem as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
        cType = 's';
        iItem -= (NUM_POWERUPS + NUM_WORLD_POWERUPS) as i16;
    } else if iItem as i32 >= NUM_POWERUPS {
        cType = 'w';
        iItem -= NUM_POWERUPS as i16;
    }

    let buf = if iStageType == 0 { format!("{}{}{}", iPlace, cType, iItem) } else { format!("{}{}", cType, iItem) };
    *szString = buf;
}

pub fn TestAndSetBonusItem(g: &mut Game, mut ts: Ptr<TourStop>, iPlace: i16, iButtonX: i16, iButtonY: i16) {
    let mut gp = g.ptr();
    let mut iMaxBonusesAllowed: i16 = 10;

    if ts.iStageType == 1 {
        iMaxBonusesAllowed = MAX_BONUS_CHESTS as i16;
    }

    if ts.iNumBonuses < iMaxBonusesAllowed {
        let mut iNumSelectableItems: i16 = NUM_WORLD_ITEMS as i16;
        if ts.iStageType == 0 {
            iNumSelectableItems = (NUM_POWERUPS + NUM_WORLD_POWERUPS) as i16;
        }

        for iItem in 0..iNumSelectableItems {
            if point_in(iButtonX, iButtonY, &ed(gp.get()).rItemDst[iItem as usize]) {
                //If this is a normal stage, then alert the player that they need to select the place for this item
                if ts.iStageType == 0 && iPlace == 0 {
                    SetDisplayMessage(g, 120, "Use Number Keys", "Hover over item", "use keys 1 to 4", "to select bonus");
                    return;
                }

                //Set the bonus item for the place selected
                let n = ts.iNumBonuses as usize;
                ts.wsbBonuses[n].iBonus = iItem;
                ts.wsbBonuses[n].iWinnerPlace = iPlace - 1;

                let iStageType = ts.iStageType;
                SetBonusString(&mut ts.wsbBonuses[n].szBonusString, iPlace, iItem, iStageType);

                ts.iNumBonuses += 1;

                break;
            }
        }
    }
}

pub fn AdjustBonuses(g: &mut Game, mut ts: Ptr<TourStop>) {
    let mut gp = g.ptr();
    //No need to do anything if we were a stage and we're still a stage
    //(or a house and still a house)
    if ts.iStageType == ed(gp.get()).iLastStageType {
        return;
    }

    if ts.iStageType == 0 {
        //Remove any score bonuses
        let mut iBonus: i16 = 0;
        while iBonus < ts.iNumBonuses {
            if ts.wsbBonuses[iBonus as usize].iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                ts.iNumBonuses -= 1;
                let mut iRemoveBonus = iBonus;
                while iRemoveBonus < ts.iNumBonuses {
                    let next = ts.wsbBonuses[iRemoveBonus as usize + 1].clone();
                    ts.wsbBonuses[iRemoveBonus as usize].iBonus = next.iBonus;
                    ts.wsbBonuses[iRemoveBonus as usize].iWinnerPlace = next.iWinnerPlace;
                    ts.wsbBonuses[iRemoveBonus as usize].szBonusString = next.szBonusString;
                    iRemoveBonus += 1;
                }
            }
            iBonus += 1;
        }

        //Add places to bonuses
        for iBonus in 0..ts.iNumBonuses as usize {
            if ts.wsbBonuses[iBonus].iWinnerPlace < 0 || ts.wsbBonuses[iBonus].iWinnerPlace > 3 {
                ts.wsbBonuses[iBonus].iWinnerPlace = 0;
            }

            let (place, bonus, stageType) = (ts.wsbBonuses[iBonus].iWinnerPlace + 1, ts.wsbBonuses[iBonus].iBonus, ts.iStageType);
            SetBonusString(&mut ts.wsbBonuses[iBonus].szBonusString, place, bonus, stageType);
        }
    } else if ts.iStageType == 1 {
        //Cap the number of
        if ts.iNumBonuses > MAX_BONUS_CHESTS as i16 {
            ts.iNumBonuses = MAX_BONUS_CHESTS as i16;
        }

        for iBonus in 0..ts.iNumBonuses as usize {
            let (bonus, stageType) = (ts.wsbBonuses[iBonus].iBonus, ts.iStageType);
            SetBonusString(&mut ts.wsbBonuses[iBonus].szBonusString, 0, bonus, stageType);
        }
    }

    ed(gp.get()).iLastStageType = ts.iStageType;
}

pub fn SaveStage(g: &mut Game, iEditStage: i16) {
    let mut ts = g.game_values.tour_stop(iEditStage as usize as usize);

    if ts.iMode >= 25 && ts.iMode <= 27 {
        ts.iMode += 975;
    }

    //Set the number of game mode settings to the maximum so we write them all out
    ts.fUseSettings = true;
    ts.iNumUsedSettings = serialize_gms(ts.iMode, &g.game_values.gamemodemenusettings).len() as i16;

    //Copy the working values back into the structure that will be saved
    ts.gmsSettings = g.game_values.gamemodemenusettings.clone();
}

pub fn EditStage(g: &mut Game, iEditStage: i16) {
    let mut gp = g.ptr();
    let mut ts = g.game_values.tour_stop(iEditStage as usize as usize);

    g.game_values.gamemodemenusettings = ts.gmsSettings.clone();

    //Set fields to write data to the selected stage
    g.ctl(ed(gp.get()).miModeField).set_output_ptr(&mut ts.iMode as *mut i16);
    g.ctl(ed(gp.get()).miNameField).set_data(g, &mut ts.szName, 128);

    g.ctl(ed(gp.get()).miPointsField).set_output_ptr(&mut ts.iPoints as *mut i16);
    g.ctl(ed(gp.get()).miFinalStageField).set_output_ptr(&mut ts.fEndStage as *mut bool);

    if ts.pszMapFile.is_empty() {
        ts.pszMapFile = g.maplist.current_shortmapname().to_string();
    }

    let mapfile = ts.pszMapFile.clone();
    g.ctl(ed(gp.get()).miMapField).set_map(g, &mapfile, false);

    g.ctl(ed(gp.get()).miBonusType).set_output_ptr(&mut ts.iBonusType as *mut i16);
    g.ctl(ed(gp.get()).miBonusTextField[0]).set_data(g, &mut ts.szBonusText[0], 128);
    g.ctl(ed(gp.get()).miBonusTextField[1]).set_data(g, &mut ts.szBonusText[1], 128);
    g.ctl(ed(gp.get()).miBonusTextField[2]).set_data(g, &mut ts.szBonusText[2], 128);
    g.ctl(ed(gp.get()).miBonusTextField[3]).set_data(g, &mut ts.szBonusText[3], 128);
    g.ctl(ed(gp.get()).miBonusTextField[4]).set_data(g, &mut ts.szBonusText[4], 128);

    ts.iBonusTextLines = 5;

    let mut iMode: i16 = ts.iMode;
    let iStageType: i16 = ts.iStageType;

    if iMode >= 25 && iMode <= 27 {
        iMode += 975;
    }

    //Show fields applicable for this mode
    g.ctl(ed(gp.get()).miPointsField).set_visible(iStageType == 0);
    g.ctl(ed(gp.get()).miFinalStageField).set_visible(iStageType == 0);
    g.ctl(ed(gp.get()).miMapField).set_visible(iStageType == 0);

    g.ctl(ed(gp.get()).miBonusType).set_visible(iStageType == 1);
    g.ctl(ed(gp.get()).miBonusTextField[0]).set_visible(iStageType == 1);
    g.ctl(ed(gp.get()).miBonusTextField[1]).set_visible(iStageType == 1);
    g.ctl(ed(gp.get()).miBonusTextField[2]).set_visible(iStageType == 1);
    g.ctl(ed(gp.get()).miBonusTextField[3]).set_visible(iStageType == 1);
    g.ctl(ed(gp.get()).miBonusTextField[4]).set_visible(iStageType == 1);

    g.ctl(ed(gp.get()).miSpecialGoalField[0]).set_visible(iMode == 1000);
    g.ctl(ed(gp.get()).miSpecialGoalField[1]).set_visible(iMode == 1001);
    g.ctl(ed(gp.get()).miSpecialGoalField[2]).set_visible(iMode == 1002);

    g.ctl(ed(gp.get()).miBonusItemsButton).set_position(430, if iStageType == 0 { 220 } else { 340 });

    if iStageType == 0 && iMode >= 0 && (iMode as i32) < GAMEMODE_LAST {
        g.ctl(ed(gp.get()).miModeField).set_current_value(iMode);

        g.ctl(ed(gp.get()).miModeSettingsButton).set_visible(iMode as i32 != game_mode_owned);

        for iGameMode in 0..GAMEMODE_LAST as usize {
            g.ctl(ed(gp.get()).miGoalField[iGameMode]).set_visible(iMode as usize == iGameMode);
        }

        g.ctl(ed(gp.get()).miGoalField[iMode as usize]).set_output_ptr(&mut ts.iGoal as *mut i16);
        g.ctl(ed(gp.get()).miGoalField[iMode as usize]).set_current_value(ts.iGoal);
        g.ctl(ed(gp.get()).miPointsField).set_current_value(ts.iPoints);
        g.ctl(ed(gp.get()).miFinalStageField).set_current_value((if ts.fEndStage { 1 } else { 0 }) != 0);

        ts.fUseSettings = true;
        ts.iNumUsedSettings = g_iNumGameModeSettings[iMode as usize];
    } else {
        //Show the settings button for boss mode
        g.ctl(ed(gp.get()).miModeSettingsButton).set_visible(iMode == 1001);

        for iGameMode in 0..GAMEMODE_LAST as usize {
            g.ctl(ed(gp.get()).miGoalField[iGameMode]).set_visible(false);
        }

        if iStageType == 1 {
            //Bonus House
            g.ctl(ed(gp.get()).miModeField).set_current_value(24);
        } else if iMode >= 1000 && iMode <= 1002 {
            //Pipe, Boss and Boxes Game
            g.ctl(ed(gp.get()).miModeField).set_current_value(iMode - 975);
            g.ctl(ed(gp.get()).miSpecialGoalField[(iMode - 1000) as usize]).set_output_ptr(&mut ts.iGoal as *mut i16);
            g.ctl(ed(gp.get()).miSpecialGoalField[(iMode - 1000) as usize]).set_current_value(ts.iGoal);
            g.ctl(ed(gp.get()).miPointsField).set_current_value(ts.iPoints);
            g.ctl(ed(gp.get()).miFinalStageField).set_current_value((if ts.fEndStage { 1 } else { 0 }) != 0);

            ts.iMode = iMode;
        }
    }
}

pub fn EnableStageMenu(g: &mut Game, fEnable: bool) {
    let mut gp = g.ptr();
    g.ctl(ed(gp.get()).miNameField).disable(!fEnable);
    g.ctl(ed(gp.get()).miModeField).disable(!fEnable);

    for iGameMode in 0..GAMEMODE_LAST as usize {
        g.ctl(ed(gp.get()).miGoalField[iGameMode]).disable(!fEnable);
    }

    g.ctl(ed(gp.get()).miSpecialGoalField[0]).disable(!fEnable);
    g.ctl(ed(gp.get()).miSpecialGoalField[1]).disable(!fEnable);
    g.ctl(ed(gp.get()).miSpecialGoalField[2]).disable(!fEnable);

    g.ctl(ed(gp.get()).miModeSettingsButton).disable(!fEnable);

    g.ctl(ed(gp.get()).miPointsField).disable(!fEnable);
    g.ctl(ed(gp.get()).miFinalStageField).disable(!fEnable);
    g.ctl(ed(gp.get()).miMapField).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusItemsButton).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusType).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusTextField[0]).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusTextField[1]).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusTextField[2]).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusTextField[3]).disable(!fEnable);
    g.ctl(ed(gp.get()).miBonusTextField[4]).disable(!fEnable);
    g.ctl(ed(gp.get()).miDeleteStageButton).disable(!fEnable);
}

pub fn NewStage(g: &mut Game, iEditStage: &mut i16) {
    let mut ts = Box::new(TourStop::default());

    ts.iStageType = 0;

    ts.szBonusText[0].clear();
    ts.szBonusText[1].clear();
    ts.szBonusText[2].clear();
    ts.szBonusText[3].clear();
    ts.szBonusText[4].clear();

    ts.pszMapFile = g.maplist.current_shortmapname().to_string();
    ts.iMode = 0;

    ts.fUseSettings = true;
    ts.iNumUsedSettings = g_iNumGameModeSettings[0];

    ts.iGoal = 10;
    ts.iPoints = 1;

    ts.iBonusType = 0;
    ts.iNumBonuses = 0;

    ts.szName = format!("Tour Stop {}", g.game_values.tourstops.len() + 1);

    ts.fEndStage = false;

    //Copy in default values first
    ts.gmsSettings = g.game_values.gamemodemenusettings.clone();

    g.game_values.tourstops.push(ts);
    g.g_worldmap.iNumStages += 1;

    *iEditStage = (g.game_values.tourstops.len() - 1) as i16;
    EditStage(g, *iEditStage);
}

pub fn editor_stage(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    unsafe {
        ed(gp.get()).mCurrentMenu = EditorMenu::StageSettings;
        current_menu(gp.get()).reset_menu(g);

        let mut done = false;
        let mut iStageDisplay: i16 = -1;
        let mut fForceStageDisplay = false;
        let mut iEditStage: i16 = -1;

        let mut rStageBonusDst = [r(0, 0, 0, 0); 10];
        let mut rHouseBonusDst = [r(0, 0, 0, 0); MAX_BONUS_CHESTS as usize];

        let mut iColCount: i16 = 0;
        let mut iRowCount: i16 = 0;
        for iItem in 0..NUM_WORLD_ITEMS as usize {
            ed(gp.get()).rItemDst[iItem].x = 16 + iColCount as i32 * 48;
            ed(gp.get()).rItemDst[iItem].y = 16 + iRowCount as i32 * 48;
            ed(gp.get()).rItemDst[iItem].w = 32;
            ed(gp.get()).rItemDst[iItem].h = 32;

            iColCount += 1;
            if iColCount > 12 {
                iColCount = 0;
                iRowCount += 1;
            }
        }

        for iStageBonus in 0..10usize {
            rStageBonusDst[iStageBonus].x = 35 + iStageBonus as i32 * 58;
            rStageBonusDst[iStageBonus].y = 360;
            rStageBonusDst[iStageBonus].w = 32;
            rStageBonusDst[iStageBonus].h = 32;
        }

        let iStartItemX: i16 = ((640 - (MAX_BONUS_CHESTS * 58 - 10)) >> 1) as i16;

        for iHouseBonus in 0..MAX_BONUS_CHESTS as usize {
            rHouseBonusDst[iHouseBonus].x = iStartItemX as i32 + iHouseBonus as i32 * 58;
            rHouseBonusDst[iHouseBonus].y = 360;
            rHouseBonusDst[iHouseBonus].w = 32;
            rHouseBonusDst[iHouseBonus].h = 32;
        }

        while !done {
            let framestart = crate::services::ticks(gp.get()) as i32;

            //Reset the keys that were down the last frame
            g.game_values.playerInput.clear_pressed_keys(1);

            let mut code: MenuCodeEnum = MENU_CODE_NONE;

            //handle messages
            while poll_event(g, &mut ed(gp.get()).event) != 0 {
                match event_type(g) {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        if iEditStage == -1 && key_sym(g) == SDLK_n {
                            NewStage(g, &mut iEditStage);
                        } else if (key_sym(g) == SDLK_ESCAPE || key_sym(g) == SDLK_e) && iEditStage == -1 {
                            if g.g_worldmap.iNumStages == 0 {
                                ed(gp.get()).edit_mode = 1;
                            } else if ed(gp.get()).set_tile < 6 || ed(gp.get()).set_tile >= g.g_worldmap.iNumStages as i32 + 6 {
                                ed(gp.get()).set_tile = 6;
                                ed(gp.get()).edit_mode = 9;
                            } else {
                                ed(gp.get()).edit_mode = 9; //change to edit mode using stages
                            }

                            return EDITOR_EDIT;
                        } else if ed(gp.get()).mCurrentMenu == EditorMenu::BonusItemPicker && key_sym(g) >= SDLK_1 && key_sym(g) <= SDLK_4 {
                            let ts = g.game_values.tour_stop(iEditStage as usize as usize);
                            if ts.iStageType == 0 {
                                let iPlace: i16 = (key_sym(g) - SDLK_1 + 1) as i16;

                                TestAndSetBonusItem(g, ts, iPlace, ed(gp.get()).mouse_x as i16, ed(gp.get()).mouse_y as i16);
                            }
                        } else if (key_sym(g) == SDLK_PAGEUP && iEditStage > 0) || (key_sym(g) == SDLK_PAGEDOWN && iEditStage < g.g_worldmap.iNumStages - 1) {
                            if iEditStage != -1 && ed(gp.get()).mCurrentMenu == EditorMenu::StageSettings {
                                SaveStage(g, iEditStage);

                                if key_sym(g) == SDLK_PAGEUP {
                                    iEditStage -= 1;
                                } else if key_sym(g) == SDLK_PAGEDOWN {
                                    iEditStage += 1;
                                }

                                EditStage(g, iEditStage);
                                current_menu(gp.get()).reset_menu(g);

                                code = MENU_CODE_MODE_CHANGED;

                                ed(gp.get()).mModeOptionsMenu.as_mut().expect("mode options menu").refresh(g);
                            }
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iTileX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                        let iTileY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;
                        let iButtonX: i16 = bound_to_window_w(ed(gp.get()).event.button_x()) as i16;
                        let iButtonY: i16 = bound_to_window_h(ed(gp.get()).event.button_y()) as i16;

                        if ed(gp.get()).event.button_button() == BUTTON_LEFT {
                            //Stages
                            if iEditStage == -1 {
                                if iTileX >= 0 && (iTileX as i32) < g.g_worldmap.iNumStages as i32 - (iTileY as i32 * 20) && iTileY >= 0 && iTileY <= (g.g_worldmap.iNumStages - 1) / 20 {
                                    ed(gp.get()).set_tile = iTileX as i32 + (iTileY as i32 * 20) + 6;

                                    ed(gp.get()).edit_mode = 9; //change to edit mode using warps

                                    //The user must release the mouse button before trying to add a tile
                                    ed(gp.get()).ignoreclick = true;

                                    return EDITOR_EDIT;
                                }
                                //New stage button
                                else if iButtonX >= 256 && iButtonX < 384 && iButtonY >= 420 && iButtonY < 452 {
                                    NewStage(g, &mut iEditStage);
                                }
                            } else if ed(gp.get()).mCurrentMenu == EditorMenu::BonusItemPicker {
                                let mut ts = g.game_values.tour_stop(iEditStage as usize as usize);

                                //See if we clicked an item and add it if we did
                                TestAndSetBonusItem(g, ts, 0, iButtonX, iButtonY);

                                //See if we clicked an already added item and remove it
                                let mut iRemoveItem: i16 = 0;
                                while iRemoveItem < ts.iNumBonuses {
                                    let rects: &[Rect] = if ts.iStageType == 1 { &rHouseBonusDst } else { &rStageBonusDst };

                                    if point_in(iButtonX, iButtonY, &rects[iRemoveItem as usize]) {
                                        let mut iAdjust = iRemoveItem;
                                        while iAdjust < ts.iNumBonuses - 1 {
                                            let next = ts.wsbBonuses[iAdjust as usize + 1].clone();
                                            ts.wsbBonuses[iAdjust as usize].iBonus = next.iBonus;
                                            ts.wsbBonuses[iAdjust as usize].iWinnerPlace = next.iWinnerPlace;
                                            ts.wsbBonuses[iAdjust as usize].szBonusString = next.szBonusString;
                                            iAdjust += 1;
                                        }

                                        ts.iNumBonuses -= 1;

                                        break;
                                    }
                                    iRemoveItem += 1;
                                }
                            } else {
                                code = current_menu(gp.get()).mouse_click(g, iButtonX, iButtonY);
                            }
                        } else if ed(gp.get()).event.button_button() == BUTTON_RIGHT {
                            if iEditStage == -1 {
                                if iTileX >= 0 && (iTileX as i32) < g.g_worldmap.iNumStages as i32 - (iTileY as i32 * 20) && iTileY >= 0 && iTileY <= (g.g_worldmap.iNumStages - 1) / 20 {
                                    iEditStage = iTileX + (iTileY * 20);
                                    EditStage(g, iEditStage);

                                    code = MENU_CODE_MODE_CHANGED;

                                    ed(gp.get()).mModeOptionsMenu.as_mut().expect("mode options menu").refresh(g);
                                }
                            }
                        }
                    }

                    T_MOUSEMOTION => {
                        bound_mouse_motion_coords(g);
                        iStageDisplay = -1;

                        if iEditStage == -1 {
                            let iMouseX: i16 = (bound_to_window_w(ed(gp.get()).event.button_x()) / TILESIZE) as i16;
                            let iMouseY: i16 = (bound_to_window_h(ed(gp.get()).event.button_y()) / TILESIZE) as i16;

                            if iMouseX >= 0 && (iMouseX as i32) < g.g_worldmap.iNumStages as i32 - (iMouseY as i32 * 20) && iMouseY >= 0 && iMouseY <= (g.g_worldmap.iNumStages - 1) / 20 {
                                iStageDisplay = iMouseX + (iMouseY * 20);
                            }
                        }
                    }

                    _ => {}
                }

                gp.get().game_values.playerInput.update_input(g, ed(gp.get()).event.input(), 1);
            }

            if iEditStage >= 0 {
                if MENU_CODE_NONE == code {
                    code = current_menu(gp.get()).send_input(g, Ptr::from_mut(&mut gp.game_values.playerInput));
                }

                if MENU_CODE_EXIT_APPLICATION == code {
                    //Save the current stage
                    SaveStage(g, iEditStage);

                    iEditStage = -1;
                } else if MENU_CODE_MODE_CHANGED == code {
                    let iMode: i16 = g.ctl(ed(gp.get()).miModeField).current_value();

                    g.ctl(ed(gp.get()).miPointsField).set_visible(iMode != 24);
                    g.ctl(ed(gp.get()).miFinalStageField).set_visible(iMode != 24);

                    g.ctl(ed(gp.get()).miMapField).set_visible(iMode != 24);

                    g.ctl(ed(gp.get()).miBonusType).set_visible(iMode == 24);
                    g.ctl(ed(gp.get()).miBonusTextField[0]).set_visible(iMode == 24);
                    g.ctl(ed(gp.get()).miBonusTextField[1]).set_visible(iMode == 24);
                    g.ctl(ed(gp.get()).miBonusTextField[2]).set_visible(iMode == 24);
                    g.ctl(ed(gp.get()).miBonusTextField[3]).set_visible(iMode == 24);
                    g.ctl(ed(gp.get()).miBonusTextField[4]).set_visible(iMode == 24);

                    g.ctl(ed(gp.get()).miSpecialGoalField[0]).set_visible(iMode == 25);
                    g.ctl(ed(gp.get()).miSpecialGoalField[1]).set_visible(iMode == 26);
                    g.ctl(ed(gp.get()).miSpecialGoalField[2]).set_visible(iMode == 27);

                    g.ctl(ed(gp.get()).miBonusItemsButton).set_position(430, if iMode != 24 { 220 } else { 340 });

                    let mut ts = g.game_values.tour_stop(iEditStage as usize as usize);

                    if iMode >= 0 && (iMode as i32) < GAMEMODE_LAST {
                        g.ctl(ed(gp.get()).miModeSettingsButton).set_visible(iMode as i32 != game_mode_owned);

                        for iGameMode in 0..GAMEMODE_LAST as usize {
                            g.ctl(ed(gp.get()).miGoalField[iGameMode]).set_visible(iMode as usize == iGameMode);
                        }

                        g.ctl(ed(gp.get()).miGoalField[iMode as usize]).set_output_ptr(&mut ts.iGoal as *mut i16);
                        g.ctl(ed(gp.get()).miGoalField[iMode as usize]).update_output();

                        ts.iStageType = 0;

                        ts.fUseSettings = true;
                        ts.iNumUsedSettings = g_iNumGameModeSettings[iMode as usize];
                    } else {
                        //Show the settings button for boss mode
                        g.ctl(ed(gp.get()).miModeSettingsButton).set_visible(iMode == 26);

                        for iGameMode in 0..GAMEMODE_LAST as usize {
                            g.ctl(ed(gp.get()).miGoalField[iGameMode]).set_visible(false);
                        }

                        if iMode == 24 {
                            ts.iStageType = 1;
                            ts.iBonusTextLines = 5;
                        } else if iMode >= 25 && iMode <= 27 {
                            ts.iStageType = 0;
                        }
                    }

                    //Removes bonuses if we went from a stage to a bonus house
                    //(and there were more than the max bonuses for a house)
                    AdjustBonuses(g, ts);
                } else if MENU_CODE_TO_MODE_SETTINGS_MENU == code {
                    let mut fModeFound = false;
                    for iGameMode in 0..GAMEMODE_LAST as usize {
                        if g.ctl(ed(gp.get()).miGoalField[iGameMode]).is_visible() {
                            ed(gp.get()).mCurrentMenu = EditorMenu::ModeSettings(iGameMode as u8);
                            current_menu(gp.get()).reset_menu(g);
                            fModeFound = true;
                            break;
                        }
                    }

                    //Look to see if this is the boss mode and go to boss settings
                    if !fModeFound {
                        if g.ctl(ed(gp.get()).miSpecialGoalField[1]).is_visible() {
                            ed(gp.get()).mCurrentMenu = EditorMenu::BossSettings;
                            current_menu(gp.get()).reset_menu(g);
                        }
                    }
                } else if MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED == code {
                    ed(gp.get()).mModeOptionsMenu.as_mut().expect("mode options menu").health_mode_start_life_changed(g);
                } else if MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED == code {
                    ed(gp.get()).mModeOptionsMenu.as_mut().expect("mode options menu").health_mode_max_life_changed(g);
                } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS == code {
                    ed(gp.get()).mCurrentMenu = EditorMenu::StageSettings;
                    current_menu(gp.get()).reset_menu(g);
                } else if MENU_CODE_MAP_CHANGED == code {
                    let mut ts = g.game_values.tour_stop(iEditStage as usize as usize);
                    ts.pszMapFile = g.maplist.current_shortmapname().to_string();
                    fForceStageDisplay = true;
                } else if MENU_CODE_TO_BONUS_PICKER_MENU == code {
                    ed(gp.get()).mCurrentMenu = EditorMenu::BonusItemPicker;
                    current_menu(gp.get()).reset_menu(g);
                } else if MENU_CODE_DELETE_STAGE_BUTTON == code {
                    g.ctl(ed(gp.get()).miDeleteStageDialogImage).set_visible(true);
                    g.ctl(ed(gp.get()).miDeleteStageDialogAreYouText).set_visible(true);
                    g.ctl(ed(gp.get()).miDeleteStageDialogSureText).set_visible(true);
                    g.ctl(ed(gp.get()).miDeleteStageDialogYesButton).set_visible(true);
                    g.ctl(ed(gp.get()).miDeleteStageDialogNoButton).set_visible(true);

                    EnableStageMenu(g, false);

                    ed(gp.get()).mStageSettingsMenu.remember_current();

                    ed(gp.get()).mStageSettingsMenu.set_initial_focus(g, ed(gp.get()).miDeleteStageDialogNoButton.id());
                    ed(gp.get()).mStageSettingsMenu.set_cancel_code(MENU_CODE_DELETE_STAGE_NO);
                    ed(gp.get()).mStageSettingsMenu.reset_menu(g);
                } else if MENU_CODE_DELETE_STAGE_YES == code || MENU_CODE_DELETE_STAGE_NO == code {
                    g.ctl(ed(gp.get()).miDeleteStageDialogImage).set_visible(false);
                    g.ctl(ed(gp.get()).miDeleteStageDialogAreYouText).set_visible(false);
                    g.ctl(ed(gp.get()).miDeleteStageDialogSureText).set_visible(false);
                    g.ctl(ed(gp.get()).miDeleteStageDialogYesButton).set_visible(false);
                    g.ctl(ed(gp.get()).miDeleteStageDialogNoButton).set_visible(false);

                    ed(gp.get()).mStageSettingsMenu.set_initial_focus(g, ed(gp.get()).miNameField.id());
                    ed(gp.get()).mStageSettingsMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);
                    ed(gp.get()).mStageSettingsMenu.restore_current(g);

                    //Yes was selected to delete this stage
                    if MENU_CODE_DELETE_STAGE_YES == code {
                        //Scan the grid of stages and remove any references to this stage
                        //and decrement stage numbers greater than this stage
                        for iRow in 0..ed(g).iWorldHeight {
                            for iCol in 0..ed(g).iWorldWidth {
                                if tile(g, iCol, iRow).iType == iEditStage + 6 {
                                    tile(g, iCol, iRow).iType = 0;
                                } else if tile(g, iCol, iRow).iType > iEditStage + 6 {
                                    tile(g, iCol, iRow).iType -= 1;
                                }
                            }
                        }

                        //Scan vehicles and remove references to deleted stage
                        let mut iVehicle = 0;
                        while iVehicle < ed(gp.get()).vehiclelist.len() {
                            let mut vehicle = ed(gp.get()).vehiclelist[iVehicle];
                            if vehicle.iActionId == iEditStage {
                                vehicle.delete();
                                ed(gp.get()).vehiclelist.remove(iVehicle);
                                continue;
                            } else if vehicle.iActionId > iEditStage {
                                vehicle.iActionId -= 1;
                            }
                            iVehicle += 1;
                        }

                        //Remove stage from tourstops vector
                        let mut iIndex: i16 = 0;
                        while (iIndex as usize) < g.game_values.tourstops.len() {
                            if iIndex == iEditStage {

                                g.game_values.tourstops.remove(iIndex as usize);
                                g.g_worldmap.iNumStages -= 1;

                                break;
                            }

                            iIndex += 1;
                        }

                        iEditStage = -1;
                        ed(gp.get()).mCurrentMenu = EditorMenu::StageSettings;
                        current_menu(gp.get()).reset_menu(g);
                    }

                    EnableStageMenu(g, true);
                }
            }

            drawmap(g, false, TILESIZE as i16);
            ed(gp.get()).menu_shade.draw(g, 0, 0);

            let color = (*g.blitdest).map_rgb(0, 0, 255);

            if iEditStage == -1 {
                for iStage in 0..g.g_worldmap.iNumStages as i32 {
                    let ix: i16 = ((iStage % 20) << 5) as i16;
                    let iy: i16 = ((iStage / 20) << 5) as i16;

                    let mut rr = r(ix as i32, iy as i32, 32, 32);
                    crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

                    gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r((iStage % 10) << 5, (iStage / 10) << 5, 32, 32));
                }

                if iStageDisplay >= 0 {
                    DisplayStageDetails(g, fForceStageDisplay, iStageDisplay, ed(gp.get()).mouse_x as i16, ed(gp.get()).mouse_y as i16);
                    fForceStageDisplay = false;
                }

                //Display New button
                gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 256, 420, &r(0, 0, 64, 32));
                gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 320, 420, &r(448, 0, 64, 32));

                gp.rm.menu_font_large.draw_centered(g, 320, 425, "New Stage");

                gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[LMB] Select Stage, [RMB] Edit Stage, [n] New Stage");
            } else {
                current_menu(gp.get()).update(g);
                current_menu(gp.get()).draw(g);

                if ed(gp.get()).mCurrentMenu != EditorMenu::BonusItemPicker {
                    let ix: i16 = 20;
                    let iy: i16 = 20;

                    let mut rr = r(ix as i32, iy as i32, 32, 32);
                    crate::common::gfx::blit::fill_rect(g.blitdest, &mut rr, color);

                    gp.rm.sprite(gp.rm.spr_worldforegroundspecial[0]).draw_src(g, ix as i32, iy as i32, &r(((iEditStage % 10) as i32) << 5, ((iEditStage / 10) as i32) << 5, 32, 32));
                }
            }

            if ed(gp.get()).mCurrentMenu == EditorMenu::BonusItemPicker {
                let ts = g.game_values.tour_stop(iEditStage as usize as usize);

                //Game powerups
                for iItem in 0..NUM_POWERUPS as usize {
                    gp.rm.sprite(gp.rm.spr_storedpoweruplarge).draw_src(g, ed(gp.get()).rItemDst[iItem].x, ed(gp.get()).rItemDst[iItem].y, &r((iItem as i32) << 5, 0, 32, 32));
                }

                //World Powerups
                for iWorldItem in 0..NUM_WORLD_POWERUPS as usize {
                    let d = ed(gp.get()).rItemDst[iWorldItem + NUM_POWERUPS as usize];
                    gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, d.x, d.y, &r((iWorldItem as i32) << 5, 0, 32, 32));
                }

                //Score Bonuses
                if ts.iStageType == 1 {
                    for iScoreBonus in 0..NUM_WORLD_SCORE_BONUSES {
                        let d = ed(gp.get()).rItemDst[(iScoreBonus + NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize];
                        gp.rm.sprite(gp.rm.spr_worlditems).draw_src(
                            g,
                            d.x,
                            d.y,
                            &r(if iScoreBonus < 10 { iScoreBonus << 5 } else { (iScoreBonus - 10) << 5 }, if iScoreBonus < 10 { 32 } else { 64 }, 32, 32),
                        );
                    }
                }

                //Draw background container
                gp.rm.sprite(gp.rm.spr_worlditempopup).draw_src(g, 0, 344, &r(0, 0, 320, 64));
                gp.rm.sprite(gp.rm.spr_worlditempopup).draw_src(g, 320, 344, &r(192, 0, 320, 64));

                let rects: &[Rect] = if ts.iStageType == 1 { &rHouseBonusDst } else { &rStageBonusDst };

                for iPickedItem in 0..ts.iNumBonuses as usize {
                    let iBonus: i32 = ts.wsbBonuses[iPickedItem].iBonus as i32;
                    let iPlace: i32 = ts.wsbBonuses[iPickedItem].iWinnerPlace as i32;

                    //Draw place behind bonus
                    if ts.iStageType == 0 {
                        gp.rm.sprite(gp.rm.spr_worlditempopup).draw_src(g, rects[iPickedItem].x - 8, rects[iPickedItem].y - 8, &r(iPlace * 48, 256, 48, 48));
                    }

                    if iBonus >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                        let iBonusIndex: i32 = iBonus - NUM_POWERUPS - NUM_WORLD_POWERUPS;
                        gp.rm.sprite(gp.rm.spr_worlditems).draw_src(
                            g,
                            rects[iPickedItem].x,
                            rects[iPickedItem].y,
                            &r(if iBonusIndex < 10 { iBonusIndex << 5 } else { (iBonusIndex - 10) << 5 }, if iBonusIndex < 10 { 32 } else { 64 }, 32, 32),
                        );
                    } else if iBonus >= NUM_POWERUPS {
                        gp.rm.sprite(gp.rm.spr_worlditems).draw_src(g, rects[iPickedItem].x, rects[iPickedItem].y, &r((iBonus - NUM_POWERUPS) << 5, 0, 32, 32));
                    } else {
                        gp.rm.sprite(gp.rm.spr_storedpoweruplarge).draw_src(g, rects[iPickedItem].x, rects[iPickedItem].y, &r(iBonus << 5, 0, 32, 32));
                    }
                }

                if ts.iStageType == 0 {
                    gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[1-4] Select Items, [LMB] Remove Items");
                } else {
                    gp.rm.menu_font_small.draw(g, 0, 480 - gp.rm.menu_font_small.get_height(), "[LMB] Select Items, [LMB] Remove Items");
                }
            }

            gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());

            DrawMessage(g);
            gfx_flipscreen(g);

            frame_wait(g, framestart);
        }

        EDITOR_QUIT
    }
}

pub fn display_help(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    drawmap(g, false, TILESIZE as i16);
    ed(gp.get()).menu_shade.draw(g, 0, 0);
    gp.rm.menu_font_large.draw_centered(g, 320, 15, "Help");

    let mut offsety: i32 = 55;
    let mut offsetx: i32 = 30;
    let lh = g.rm.menu_font_small.get_height();
    let lines_left: [&str; 15] = [
        "[1] - Water Mode",
        "[2] - Land Mode",
        "[3] - Stage Objects Mode",
        "[4] - Path Mode",
        "[5] - Objects Mode",
        "[6] - Bridges Mode",
        "[p] - Connection Mode",
        "[w] - Warp Mode",
        "[v] - Vehicle",
        "      [c] - Copy Vehicle",
        "[t] - Start and Doors",
        "[b] - Vehicle Boundaries",
        "[i] - Initial Powerups",
        "[e] - Edit Stages",
        "",
    ];
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "Modes:");
    offsety += lh + 2;
    for (i, line) in lines_left.iter().take(14).enumerate() {
        gp.rm.menu_font_small.draw(g, offsetx, offsety, line);
        offsety += lh + if i == 13 { 20 } else { 2 };
    }

    gp.rm.menu_font_small.draw(g, offsetx, offsety, "File:");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[n] - New World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[s] - Save World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[shift] + [s] - Save As");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[f] - Find World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[shift] + [f] - New Search");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[pageup] - Go To Previous World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[pagedown] - Go To Next World");

    offsetx = 300;
    offsety = 55;

    gp.rm.menu_font_small.draw(g, offsetx, offsety, "Place Tiles:");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[Left Mouse Button] - Place Item");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[Right Mouse Button] - Remove Item");
    offsety += lh + 20;

    gp.rm.menu_font_small.draw(g, offsetx, offsety, "Miscellaneous:");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[r] - Change Music Category");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[Arrow Keys] - Navigate World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[a] - Automatic Path/Land");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[k] - Resize World");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[ctrl] + [delete] - Clear All");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[insert] - Screenshot");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[alt] + [enter] - Full Screen/Window");
    offsety += lh + 2;
    gp.rm.menu_font_small.draw(g, offsetx, offsety, "[space] - Toggle Stage Previews");

    gfx_flipscreen(g);

    loop {
        let framestart = crate::services::ticks(gp.get()) as i32;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => return 0,
                T_KEYDOWN => return 0,
                _ => {}
            }
        }

        frame_wait(g, framestart);
    }
}

pub fn save_as(g: &mut Game) -> i32 {
    let mut fileName = String::new();
    let mut mapLocation = String::from("worlds/");

    if dialog(g, "Save As", "Enter name:", &mut fileName, 64) {
        fileName.push_str(".txt");
        mapLocation.push_str(&fileName);
        g.worldlist.add(mapLocation.into());
        g.worldlist.find(&fileName);
        g.game_values.worldindex = g.worldlist.current_index() as i16;
        savecurrentworld(g);
        loadcurrentworld(g);
    }

    0
}

fn draw_dialog(g: &mut Game, title: &str, instructions: &str, input: Option<&str>) {
        let mut gp = g.ptr();
    drawmap(g, false, TILESIZE as i16);
    ed(gp.get()).menu_shade.draw(g, 0, 0);
    gp.rm.sprite(ed(gp.get()).spr_dialog).draw_src(g, 224, 176, &r(0, 0, 192, 128));
    gp.rm.menu_font_large.draw_centered(g, 320, 200, title);
    gp.rm.menu_font_small.draw(g, 240, 235, instructions);
    if let Some(input) = input {
        gp.rm.menu_font_small.draw(g, 240, 255, input);
    }
    gp.rm.menu_font_small.draw_right_justified(g, 640, 0, &gp.worldlist.current_path().to_string_lossy());
    gfx_flipscreen(g);
}

/// `char* input` of capacity `inputsize`: the caller's buffer keeps whatever was typed.
pub fn dialog(g: &mut Game, title: &str, instructions: &str, input: &mut String, inputsize: i32) -> bool {
    let mut gp = g.ptr();
    let mut currentChar: u32 = 0;

    draw_dialog(g, title, instructions, None);

    loop {
        let framestart = crate::services::ticks(gp.get()) as i32;

        //handle messages
        while poll_event(g, &mut ed(gp.get()).event) != 0 {
            match event_type(g) {
                T_QUIT => return false,

                T_KEYDOWN => {
                    let sym = key_sym(g);
                    if sym == SDLK_KP_ENTER || sym == SDLK_RETURN {
                        return true;
                    } else if sym == SDLK_ESCAPE {
                        return false;
                    } else if sym == SDLK_BACKSPACE {
                        if currentChar > 0 {
                            input.truncate(currentChar as usize - 1);

                            draw_dialog(g, title, instructions, Some(input));

                            currentChar -= 1;
                        }
                    } else {
                        /* I realize the if statement below is long and can be substituted with
                        the function isalnum(event.key_sym()) but I did it this way because
                        isalnum acts funny (ie wrong) when the number pad is pressed. */
                        let isdigit = (b'0' as i32..=b'9' as i32).contains(&sym);
                        if (isdigit || sym == 45 || sym == 32 || sym == 61 || (sym >= 95 && sym <= 122)) && currentChar < (inputsize as u32).wrapping_sub(1) {
                            //insert character into fileName and onScreenText and increment current char
                            let mut key: u8 = sym as u8;

                            let keystate = crate::services::services(gp.get()).keyboard.state().as_ptr();
                            if check_key(gp.get(), keystate, SDLK_LSHIFT) || check_key(gp.get(), keystate, SDLK_RSHIFT) {
                                if sym == 45 {
                                    key = 95;
                                } else if sym >= 95 && sym <= 122 {
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
                }

                _ => {}
            }
        }

        frame_wait(g, framestart);
    }
}

pub fn find(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    let mut fileName = String::new();
    //char mapLocation[FILEBUFSIZE] = "maps/";

    if dialog(g, "Find Map", "Enter name:", &mut fileName, 64) {
        ed(gp.get()).findstring = fileName.clone();

        let fs = ed(gp.get()).findstring.clone();
        if g.worldlist.find(&fs) {
            g.game_values.worldindex = g.worldlist.current_index() as i16;
            loadcurrentworld(g);
        }
    }

    0
}

pub fn clear_world(g: &mut Game) -> i32 {
    g.g_worldmap.clear();
    updateworldsurface(g);

    println!("World Cleared");
    0
}

pub fn loadcurrentworld(g: &mut Game) {
    let mut gp = g.ptr();
    g.game_values.worldindex = g.worldlist.current_index() as i16;
    let path = g.worldlist.at(g.game_values.worldindex as usize).to_string_lossy().into_owned();
    *g.g_worldmap = WorldMap::new_path(g, &path, TILESIZE as i16);
    ReadVehiclesIntoEditor(g);
    ReadWarpsIntoEditor(g);

    ed(gp.get()).draw_offset_col = 0;
    ed(gp.get()).draw_offset_row = 0;

    ed(gp.get()).rectSrcSurface.x = 0;
    ed(gp.get()).rectSrcSurface.y = 0;
    ed(gp.get()).fNeedBlackBackground = false;

    g.g_worldmap.get_world_size(&mut ed(gp.get()).iWorldWidth, &mut ed(gp.get()).iWorldHeight);

    if ed(gp.get()).iWorldWidth >= 20 {
        ed(gp.get()).rectSrcSurface.w = 640;
        ed(gp.get()).draw_offset_x = 0;
    } else {
        ed(gp.get()).rectSrcSurface.w = ed(gp.get()).iWorldWidth as i32 * TILESIZE;
        ed(gp.get()).draw_offset_x = (640 - ed(gp.get()).iWorldWidth as i32 * TILESIZE) >> 1;
        ed(gp.get()).fNeedBlackBackground = true;
    }

    if ed(gp.get()).iWorldHeight >= 15 {
        ed(gp.get()).rectSrcSurface.h = 480;
        ed(gp.get()).draw_offset_y = 0;
    } else {
        ed(gp.get()).rectSrcSurface.h = ed(gp.get()).iWorldHeight as i32 * TILESIZE;
        ed(gp.get()).draw_offset_y = (480 - ed(gp.get()).iWorldHeight as i32 * TILESIZE) >> 1;
        ed(gp.get()).fNeedBlackBackground = true;
    }

    ed(gp.get()).rectDstSurface.x = ed(gp.get()).draw_offset_x;
    ed(gp.get()).rectDstSurface.y = ed(gp.get()).draw_offset_y;

    updateworldsurface(g);
}

pub fn savecurrentworld(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    SetDisplayMessage(g, 60, "Saved", "Your world has", "been saved.", "");

    WriteVehiclesIntoWorld(g);
    WriteWarpsIntoWorld(g);
    let path = g.worldlist.at(g.game_values.worldindex as usize).to_string_lossy().into_owned();
    gp.g_worldmap.save(g, &path);
    0
}

pub fn SetDisplayMessage(g: &mut Game, iTime: i16, szTitle: &str, szLine1: &str, szLine2: &str, szLine3: &str) {
    let mut gp = g.ptr();
    ed(gp.get()).g_messagedisplaytimer = iTime;
    ed(gp.get()).g_szMessageTitle = szTitle.to_string();
    ed(gp.get()).g_szMessageLine[0] = szLine1.to_string();
    ed(gp.get()).g_szMessageLine[1] = szLine2.to_string();
    ed(gp.get()).g_szMessageLine[2] = szLine3.to_string();
}

pub fn findcurrentstring(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    if !ed(gp.get()).findstring.is_empty() {
        let fs = ed(gp.get()).findstring.clone();
        if g.worldlist.find(&fs) {
            g.game_values.worldindex = g.worldlist.current_index() as i16;
            loadcurrentworld(g);
        }
    }

    0
}

/// C `atoi`: optional whitespace and sign, then digits.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, body) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut v: i32 = 0;
    for c in body.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        v = v.wrapping_mul(10).wrapping_add((c - b'0') as i32);
    }
    if neg {
        -v
    } else {
        v
    }
}

fn delete_all_vehicles_and_warps(g: &mut Game, fVehicles: bool) {
    let mut gp = g.ptr();
    if fVehicles {
        for v in ed(gp.get()).vehiclelist.drain(..) {
            v.delete();
        }
    }

    for w in ed(gp.get()).warplist.drain(..) {
        w.delete();
    }
}

pub fn new_world(g: &mut Game) -> i32 {
        let mut gp = g.ptr();
    let mut fileName = String::new();
    let mut worldLocation = String::from("worlds/");
    let mut szWidth = String::new();
    let mut szHeight = String::new();

    if dialog(g, "New World", "Enter name:", &mut fileName, 64) && dialog(g, "New World", "Width:", &mut szWidth, 4) && dialog(g, "New World", "Height:", &mut szHeight, 4) {
        let mut iWidth: i16 = atoi(&szWidth) as i16;
        let mut iHeight: i16 = atoi(&szHeight) as i16;

        if iWidth < 1 {
            iWidth = 1;
        }

        if iHeight < 1 {
            iHeight = 1;
        }

        delete_all_vehicles_and_warps(g, true);

        *g.g_worldmap = WorldMap::new(g, iWidth, iHeight);
        fileName.push_str(".txt");
        worldLocation.push_str(&fileName);
        gp.worldlist.add(convert_path(g, &worldLocation).into());
        g.worldlist.find(&fileName);
        g.game_values.worldindex = g.worldlist.current_index() as i16;
        savecurrentworld(g);
        loadcurrentworld(g);
    }

    0
}

pub fn resize_world(g: &mut Game) -> i32 {
    let mut gp = g.ptr();
    let mut szWidth = String::new();
    let mut szHeight = String::new();

    if dialog(g, "Resize World", "Width:", &mut szWidth, 4) && dialog(g, "Resize World", "Height:", &mut szHeight, 4) {
        let mut iWidth: i16 = atoi(&szWidth) as i16;
        let mut iHeight: i16 = atoi(&szHeight) as i16;

        if iWidth < 1 {
            iWidth = 1;
        }

        if iHeight < 1 {
            iHeight = 1;
        }

        let mut i = 0;
        while i < ed(gp.get()).vehiclelist.len() {
            let v = ed(gp.get()).vehiclelist[i];
            if v.currentTile.x >= iWidth || v.currentTile.y >= iHeight {
                RemoveVehicleFromTile(g, v.currentTile.x, v.currentTile.y);
                //List was modified, restart.
                i = 0;
            } else {
                i += 1;
            }
        }

        delete_all_vehicles_and_warps(g, false);

        g.g_worldmap.resize(iWidth, iHeight);

        savecurrentworld(g);
        loadcurrentworld(g);
    }

    0
}

//take screenshots in full and thumbnail sizes
pub fn takescreenshot(g: &mut Game) {
        let mut gp = g.ptr();
    unsafe {
        let iTileSizes: [i16; 3] = [32, 16, 8];

        for iScreenshotSize in 0..3usize {
            let iTileSize: i16 = iTileSizes[iScreenshotSize];
            let path = g.worldlist.at(g.game_values.worldindex as usize).to_string_lossy().into_owned();
            *g.g_worldmap = WorldMap::new_path(g, &path, iTileSize);

            let mut w: i16 = 0;
            let mut h: i16 = 0;
            g.g_worldmap.get_world_size(&mut w, &mut h);

            //Draw most of the world to screenshot
            let sScreenshot = crate::common::gfx::soft_surface::alloc_surface(Surface::new_xrgb(iTileSize as i32 * w as i32, iTileSize as i32 * h as i32));
            g.blitdest = sScreenshot;

            gp.g_worldmap.draw_map_to_surface_full(g, sScreenshot);

            //Draw vehicles to screenshot
            for vehicle in ed(gp.get()).vehiclelist.iter() {
                let ix: i16 = vehicle.currentTile.x * iTileSize;
                let iy: i16 = vehicle.currentTile.y * iTileSize;

                gp.rm.sprite(gp.rm.spr_worldvehicle[iScreenshotSize]).draw_src(
                    g,
                    ix as i32,
                    iy as i32,
                    &r(vehicle.iDrawDirection as i32 * iTileSize as i32, vehicle.iDrawSprite as i32 * iTileSize as i32, iTileSize as i32, iTileSize as i32),
                );
            }

            //Draw warps to screenshot
            for warp in ed(gp.get()).warplist.iter() {
                if warp.posA.x >= 0 {
                    ed(gp.get()).spr_warps[iScreenshotSize].draw_src(
                        g,
                        warp.posA.x as i32 * iTileSize as i32,
                        warp.posA.y as i32 * iTileSize as i32,
                        &r(warp.id as i32 * iTileSize as i32, 0, iTileSize as i32, iTileSize as i32),
                    );
                }

                if warp.posB.x >= 0 {
                    ed(gp.get()).spr_warps[iScreenshotSize].draw_src(
                        g,
                        warp.posB.x as i32 * iTileSize as i32,
                        warp.posB.y as i32 * iTileSize as i32,
                        &r(warp.id as i32 * iTileSize as i32, 0, iTileSize as i32, iTileSize as i32),
                    );
                }
            }

            //Save the screenshot with the same name as the map file
            let mut szSaveFile = String::from("worlds/screenshots/");
            szSaveFile += &get_name_from_file_name(&g.worldlist.current_path().to_string_lossy(), false);

            if iTileSize as i32 == PREVIEWTILESIZE {
                szSaveFile += "_preview";
            } else if iTileSize as i32 == THUMBTILESIZE {
                szSaveFile += "_thumb";
            }

            szSaveFile += ".png";
            let _ = (*sScreenshot).save_png(std::path::Path::new(&convert_path(g, &szSaveFile)));

            crate::common::gfx::soft_surface::free_surface(sScreenshot);
        }

        let path = g.worldlist.at(g.game_values.worldindex as usize).to_string_lossy().into_owned();
        *g.g_worldmap = WorldMap::new_path(g, &path, iTileSizes[0]);
        g.blitdest = g.screen;
    }
}
