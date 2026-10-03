//! Port of src/worldeditor/worldeditor.cpp
//!
//! The C++ file defines its own copies of `screen`, `blitdest`, the object containers and the
//! other game globals the shared code links against; the port reuses the crate's (smw::main,
//! smw::gs_gameplay) and only defines the editor's own state here.

use crate::common::cmd_args as cmd;
use crate::common::editor_harness;
use crate::common::file_io::BinaryFile;
use crate::common::file_list::{FiltersList, GraphicsList, WorldList};
use crate::common::game::ensure_settings_dir;
use crate::common::game_mode::{game_mode_owned, GAMEMODE_LAST, GAMEMODE_NUM_OPTIONS};
use crate::common::game_values::{controlkeys, TITLESTRING};
use crate::common::gfx::color::colors;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::gfx::{gfx_changefullscreen, gfx_flipscreen, gfx_init, gfx_settitle};
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
use crate::common::uicontrol::{ctl_ptr, TextAlign, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::common::world_tour_stop::TourStop;
use crate::globals::*;
use crate::smw::menu::mode_options_menu::UI_ModeOptionsMenu;
use crate::smw::world::*;
use sdl2::sys::image::{IMG_Load, IMG_SavePNG};
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::{
    SDL_CreateRGBSurface, SDL_Event, SDL_EventType, SDL_FillRect, SDL_FreeSurface, SDL_GetKeyboardState, SDL_GetScancodeFromKey, SDL_GetTicks, SDL_Keycode, SDL_Keymod,
    SDL_MapRGB, SDL_PollEvent, SDL_Rect, SDL_Surface, SDL_UpperBlit, SDL_BUTTON_LEFT, SDL_BUTTON_RIGHT,
};
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

pub static mut sMapSurface: *mut SDL_Surface = null_mut();

pub static mut rectSrcSurface: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 768, h: 608 };
pub static mut rectDstSurface: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 640, h: 480 };
pub static mut fNeedBlackBackground: bool = false;
pub static mut iWorldWidth: i16 = 0;
pub static mut iWorldHeight: i16 = 0;

pub static mut event: SDL_Event = unsafe { std::mem::zeroed() };

pub static mut spr_dialog: Global<gfxSprite> = Global::uninit();
pub static mut menu_shade: Global<gfxSprite> = Global::uninit();
pub static mut spr_largedialog: Global<gfxSprite> = Global::uninit();

pub static mut spr_warps: Global<[gfxSprite; 3]> = Global::uninit();
pub static mut spr_path: Global<gfxSprite> = Global::uninit();

pub static mut spr_vehicleicons: Global<gfxSprite> = Global::uninit();

pub static mut set_tile: i32 = 0;
pub static mut fAutoPaint: bool = true;

pub static mut edit_mode: i32 = 0;

pub static mut draw_offset_col: i32 = 0; //col and row offset for drawing map to surface
pub static mut draw_offset_row: i32 = 0;
pub static mut draw_offset_x: i32 = 0; //x and y offset for drawing maps smaller than screensize
pub static mut draw_offset_y: i32 = 0;

pub static mut state: i32 = 0;
pub static mut selectedtiles: [[bool; MAPHEIGHT as usize]; MAPWIDTH as usize] = [[false; MAPHEIGHT as usize]; MAPWIDTH as usize];
pub static mut moveselectedtiles: [[bool; MAPHEIGHT as usize]; MAPWIDTH as usize] = [[false; MAPHEIGHT as usize]; MAPWIDTH as usize];

pub static mut mouse_x: i32 = 0;
pub static mut mouse_y: i32 = 0;

fn bound_to_window_w(x: i32) -> i32 {
    0.max(x.min(640 - 1))
}

fn bound_to_window_h(y: i32) -> i32 {
    0.max(y.min(480 - 1))
}

fn bound_mouse_motion_coords() {
    unsafe {
        mouse_x = bound_to_window_w(event.motion.x);
        mouse_y = bound_to_window_h(event.motion.y);
    }
}

//Vehicle structure that holds the current vehicle "stamp"
pub static mut g_wvVehicleStamp: Global<WorldVehicle> = Global::uninit();

fn check_key(keystate: *const u8, key: SDL_Keycode) -> bool {
    unsafe { *keystate.add(SDL_GetScancodeFromKey(key) as usize) != 0 }
}

pub static mut ignoreclick: bool = false;

pub static mut findstring: String = String::new();

pub static mut g_musiccategorydisplaytimer: i16 = 0;

pub static mut g_messagedisplaytimer: i16 = 0;
pub static mut g_szMessageTitle: String = String::new();
pub static mut g_szMessageLine: [String; 3] = [String::new(), String::new(), String::new()];

//Vehicle stuff
pub static mut vehiclelist: Vec<Ptr<WorldVehicle>> = Vec::new();

//Warp stuff
pub static mut warplist: Vec<Ptr<WorldWarp>> = Vec::new();

pub static mut g_fFullScreen: bool = false;
pub static mut g_fShowStagePreviews: bool = true;

//Stage Mode Menu
pub static mut mCurrentMenu: Ptr<UI_Menu> = Ptr::null();
pub static mut mStageSettingsMenu: Global<UI_Menu> = Global::uninit();
pub static mut mBonusItemPicker: Global<UI_Menu> = Global::uninit();

pub static mut miModeField: Ptr<MI_ImageSelectField> = Ptr::null();
pub static mut miGoalField: [Ptr<MI_SelectField<i16>>; GAMEMODE_LAST as usize] = [Ptr::null(); GAMEMODE_LAST as usize];
pub static mut miModeSettingsButton: Ptr<MI_Button> = Ptr::null();
pub static mut miBonusItemsButton: Ptr<MI_Button> = Ptr::null();
pub static mut miSpecialGoalField: [Ptr<MI_SelectField<i16>>; 3] = [Ptr::null(); 3];

pub static mut miFinalStageField: Ptr<MI_SelectField<bool>> = Ptr::null();
pub static mut miPointsField: Ptr<MI_SelectField<i16>> = Ptr::null();

pub static mut miNameField: Ptr<MI_TextField> = Ptr::null();
pub static mut miBonusType: Ptr<MI_SelectField<i16>> = Ptr::null();
pub static mut miBonusTextField: [Ptr<MI_TextField>; 5] = [Ptr::null(); 5];

pub static mut miMapField: Ptr<MI_MapField> = Ptr::null();

pub static mut miDeleteStageButton: Ptr<MI_Button> = Ptr::null();

pub static mut miDeleteStageDialogImage: Ptr<MI_Image> = Ptr::null();
pub static mut miDeleteStageDialogAreYouText: Ptr<MI_Text> = Ptr::null();
pub static mut miDeleteStageDialogSureText: Ptr<MI_Text> = Ptr::null();
pub static mut miDeleteStageDialogYesButton: Ptr<MI_Button> = Ptr::null();
pub static mut miDeleteStageDialogNoButton: Ptr<MI_Button> = Ptr::null();

//Vehicle Creation Menu
pub static mut mVehicleMenu: Global<UI_Menu> = Global::uninit();
pub static mut miVehicleSpriteField: Ptr<MI_ImageSelectField> = Ptr::null();
pub static mut miVehicleStageField: Ptr<MI_ImageSelectField> = Ptr::null();
pub static mut miVehicleMinMovesField: Ptr<MI_SelectField<i16>> = Ptr::null();
pub static mut miVehicleMaxMovesField: Ptr<MI_SelectField<i16>> = Ptr::null();
pub static mut miVehiclePacesField: Ptr<MI_SelectField<bool>> = Ptr::null();
pub static mut miVehicleDirectionField: Ptr<MI_SelectField<i16>> = Ptr::null();
pub static mut miVehicleBoundaryField: Ptr<MI_SelectField<i16>> = Ptr::null();
pub static mut miVehicleCreateButton: Ptr<MI_Button> = Ptr::null();
pub static mut miTitleText: Ptr<MI_Text> = Ptr::null();

pub static mut mModeOptionsMenu: Ptr<UI_ModeOptionsMenu> = Ptr::null();

pub static mut sMapThumbnail: *mut SDL_Surface = null_mut();
pub static mut iOldStageId: i16 = -1;

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

pub static mut stagemodes: Vec<StageMode> = Vec::new();

/// `strncpy(dst, src, n); dst[n - 1] = 0;` into a `char[n]`.
fn truncate_cstr(s: &str, n: usize) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len().min(n - 1);
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

pub fn set_stage_mode(iIndex: i16, szModeName: &str, szGoalName: &str, iIncrement: i16, iDefault: i16) {
    unsafe {
        if iIndex < 0 || iIndex as i32 >= GAMEMODE_LAST {
            return;
        }

        let sm = &mut stagemodes[iIndex as usize];

        sm.szName = truncate_cstr(szModeName, 128);

        sm.szGoal = truncate_cstr(szGoalName, 64);

        sm.iDefaultGoal = iDefault;

        for iModeOption in 0..(GAMEMODE_NUM_OPTIONS - 1) {
            sm.options[iModeOption].iValue = ((iModeOption as i32 + 1) * iIncrement as i32) as i16;
            sm.options[iModeOption].szName = format!("{}", sm.options[iModeOption].iValue);
        }
    }
}

fn r(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

fn key_sym() -> i32 {
    unsafe { event.key.keysym.sym }
}

fn event_type() -> u32 {
    unsafe { event.type_ }
}

const T_QUIT: u32 = SDL_EventType::SDL_QUIT as u32;
const T_KEYDOWN: u32 = SDL_EventType::SDL_KEYDOWN as u32;
const T_KEYUP: u32 = SDL_EventType::SDL_KEYUP as u32;
const T_MOUSEBUTTONDOWN: u32 = SDL_EventType::SDL_MOUSEBUTTONDOWN as u32;
const T_MOUSEBUTTONUP: u32 = SDL_EventType::SDL_MOUSEBUTTONUP as u32;
const T_MOUSEMOTION: u32 = SDL_EventType::SDL_MOUSEMOTION as u32;

const BUTTON_LEFT: u8 = SDL_BUTTON_LEFT as u8;
const BUTTON_RIGHT: u8 = SDL_BUTTON_RIGHT as u8;
const SDL_BUTTON_LMASK: u32 = 1 << (SDL_BUTTON_LEFT - 1);
const SDL_BUTTON_RMASK: u32 = 1 << (SDL_BUTTON_RIGHT - 1);

/// The end-of-frame wait every editor loop does; the reference harness `#define`s `SDL_Delay` to this.
fn frame_wait(framestart: i32) {
    unsafe {
        let mut delay: i32 = WAITTIME - (SDL_GetTicks() as i32 - framestart);
        if delay < 0 {
            delay = 0;
        } else if delay > WAITTIME {
            delay = WAITTIME;
        }

        editor_harness::frame_delay(delay as u32);
    }
}

fn tile(iCol: i16, iRow: i16) -> &'static mut WorldMapTile {
    unsafe { g_worldmap.tiles.at_mut(iCol as usize, iRow as usize) }
}

fn sprite(s: &mut gfxSprite) -> Ptr<gfxSprite> {
    Ptr::from_mut(s)
}

fn dump_editor_state(out: &mut dyn Write) {
    unsafe {
        let _ = writeln!(
            out,
            "E state={} edit_mode={} set_tile={} auto={} col={} row={} w={} h={} stages={} vehicles={} warps={} world={}",
            state,
            edit_mode,
            set_tile,
            fAutoPaint as i32,
            draw_offset_col,
            draw_offset_row,
            iWorldWidth,
            iWorldHeight,
            game_values.tourstops.len(),
            vehiclelist.len(),
            warplist.len(),
            worldlist.current_path().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        );
    }
}

fn load_sprite(path: &str, color_key: crate::common::gfx::color::RGB) -> gfxSprite {
    gfxSprite::from_file(Path::new(&convert_path(path)), Some(color_key), None, None)
}

fn load_sprite_alpha(path: &str, color_key: crate::common::gfx::color::RGB, alpha: u8) -> gfxSprite {
    gfxSprite::from_file(Path::new(&convert_path(path)), Some(color_key), Some(alpha), None)
}

//main main main
pub fn main() {
    crate::globals::init_globals();

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

    unsafe {
        if !cmd.data_root.is_empty() {
            RootDataDirectory = cmd.data_root.clone();
        }

        ensure_settings_dir();

        rm = Ptr::new_box(CResourceManager::new());

        g_map = Ptr::from_box(CMap::new());
        g_tilesetmanager = Ptr::new_box(CTilesetManager::new());
        filterslist = Ptr::new_box(FiltersList::new());
        maplist = Ptr::new_box(MapList::new(true));
        menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        gamegraphicspacklist = Ptr::new_box(GraphicsList::new());
        worldlist = Ptr::new_box(WorldList::new());

        game_values.sound = false;
        game_values.music = false;

        /* This must occur before any data files are loaded */
        initialize_paths();

        let mut done: bool;

        println!("-------------------------------------------------------------------------------");
        println!(" {} {}", TITLESTRING, MAPTITLESTRING);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        let mut saved_col: i32 = 0;
        let mut saved_row: i32 = 0;
        {
            let options_path = get_home_directory() + "worldeditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "rb");
            if editor_settings.is_open() {
                saved_col = editor_settings.read_i32();
                saved_row = editor_settings.read_i32();
                g_fFullScreen = editor_settings.read_bool();
                findstring = editor_settings.read_string_long(FILEBUFSIZE as usize);
            }
        }

        gfx_init(640, 480, g_fFullScreen);
        blitdest = screen;
        editor_harness::init();
        editor_harness::set_dumper(dump_editor_state);
        g_tilesetmanager.init(&convert_path("gfx/Classic/tilesets"));

        let title = format!("{} {}", TITLESTRING, MAPTITLESTRING);
        gfx_settitle(&title);

        game_values.toplayer = true;

        println!("\n---------------- loading graphics ----------------");

        spr_warps.init([
            load_sprite("gfx/leveleditor/leveleditor_warp.png", colors::MAGENTA),
            load_sprite("gfx/leveleditor/leveleditor_warp_preview.png", colors::MAGENTA),
            load_sprite("gfx/leveleditor/leveleditor_warp_thumbnail.png", colors::MAGENTA),
        ]);

        spr_path.init(load_sprite("gfx/leveleditor/leveleditor_world_path.png", colors::MAGENTA));

        rm.spr_selectedtile = load_sprite_alpha("gfx/leveleditor/leveleditor_selectedtile.png", colors::BLACK, 128);

        spr_dialog.init(load_sprite_alpha("gfx/leveleditor/leveleditor_dialog.png", colors::MAGENTA, 255));
        menu_shade.init(load_sprite_alpha("gfx/leveleditor/leveleditor_shade.png", colors::MAGENTA, 128));
        spr_largedialog.init(load_sprite_alpha("gfx/leveleditor/leveleditor_platform.png", colors::MAGENTA, 255));

        rm.menu_font_small = gfxFont::from_path(&convert_path("gfx/packs/Classic/fonts/font_small.png"));
        rm.menu_font_large = gfxFont::from_path(&convert_path("gfx/packs/Classic/fonts/font_large.png"));

        println!("\n---------------- load world ----------------");

        rm.spr_worldbackground[0] = load_sprite("gfx/packs/Classic/world/world_background.png", colors::MAGENTA);
        rm.spr_worldbackground[1] = load_sprite("gfx/packs/Classic/world/preview/world_background.png", colors::MAGENTA);
        rm.spr_worldbackground[2] = load_sprite("gfx/packs/Classic/world/thumbnail/world_background.png", colors::MAGENTA);

        rm.spr_worldforeground[0] = load_sprite("gfx/packs/Classic/world/world_foreground.png", colors::MAGENTA);
        rm.spr_worldforeground[1] = load_sprite("gfx/packs/Classic/world/preview/world_foreground.png", colors::MAGENTA);
        rm.spr_worldforeground[2] = load_sprite("gfx/packs/Classic/world/thumbnail/world_foreground.png", colors::MAGENTA);

        rm.spr_worldforegroundspecial[0] = load_sprite("gfx/packs/Classic/world/world_foreground_special.png", colors::MAGENTA);
        rm.spr_worldforegroundspecial[1] = load_sprite("gfx/packs/Classic/world/preview/world_foreground_special.png", colors::MAGENTA);
        rm.spr_worldforegroundspecial[2] = load_sprite("gfx/packs/Classic/world/thumbnail/world_foreground_special.png", colors::MAGENTA);

        rm.spr_worldpaths[0] = load_sprite("gfx/packs/Classic/world/world_paths.png", colors::MAGENTA);
        rm.spr_worldpaths[1] = load_sprite("gfx/packs/Classic/world/preview/world_paths.png", colors::MAGENTA);
        rm.spr_worldpaths[2] = load_sprite("gfx/packs/Classic/world/thumbnail/world_paths.png", colors::MAGENTA);

        rm.spr_worldvehicle[0] = load_sprite("gfx/packs/Classic/world/world_vehicles.png", colors::MAGENTA);
        rm.spr_worldvehicle[1] = load_sprite("gfx/packs/Classic/world/preview/world_vehicles.png", colors::MAGENTA);
        rm.spr_worldvehicle[2] = load_sprite("gfx/packs/Classic/world/thumbnail/world_vehicles.png", colors::MAGENTA);

        rm.spr_worlditems = load_sprite("gfx/packs/Classic/world/world_powerups.png", colors::MAGENTA);
        rm.spr_worlditempopup = load_sprite("gfx/packs/Classic/world/world_item_popup.png", colors::MAGENTA);

        rm.spr_storedpowerupsmall = load_sprite("gfx/packs/Classic/powerups/small.png", colors::MAGENTA);
        rm.spr_worlditemssmall = load_sprite("gfx/packs/Classic/world/world_powerupssmall.png", colors::MAGENTA);
        rm.spr_worlditemsplace = load_sprite("gfx/packs/Classic/world/world_bonusplace.png", colors::MAGENTA);

        rm.menu_dialog = load_sprite("gfx/packs/Classic/menu/menu_dialog.png", colors::MAGENTA);

        //Mode Options Menu Gfx
        rm.menu_egg = load_sprite("gfx/packs/Classic/modeobjects/menu_egg.png", colors::MAGENTA);
        rm.menu_stomp = load_sprite("gfx/packs/Classic/modeobjects/menu_stomp.png", colors::MAGENTA);
        rm.menu_survival = load_sprite("gfx/packs/Classic/modeobjects/menu_survival.png", colors::MAGENTA);
        rm.spr_phanto = load_sprite("gfx/packs/Classic/modeobjects/phanto.png", colors::MAGENTA);
        rm.menu_plain_field = load_sprite("gfx/leveleditor/menu_plain_field.png", colors::MAGENTA);
        rm.menu_slider_bar = load_sprite("gfx/packs/Classic/menu/menu_slider_bar.png", colors::MAGENTA);
        rm.spr_selectfield = load_sprite("gfx/leveleditor/menu_selectfield.png", colors::MAGENTA);
        rm.menu_verticalarrows = load_sprite("gfx/packs/Classic/menu/menu_vertical_arrows.png", colors::MAGENTA);
        rm.spr_storedpoweruplarge = load_sprite("gfx/packs/Classic/powerups/large.png", colors::MAGENTA);

        rm.menu_mode_small = load_sprite("gfx/packs/Classic/menu/menu_mode_small.png", colors::MAGENTA);
        rm.menu_mode_large = load_sprite("gfx/packs/Classic/menu/menu_mode_large.png", colors::MAGENTA);

        spr_vehicleicons.init(load_sprite("gfx/leveleditor/vehicle_icons.png", colors::MAGENTA));

        rm.spr_thumbnail_warps[0] = load_sprite("gfx/packs/Classic/menu/menu_warp_preview.png", colors::MAGENTA);
        rm.spr_thumbnail_warps[1] = load_sprite("gfx/packs/Classic/menu/menu_warp_thumbnail.png", colors::MAGENTA);

        rm.spr_thumbnail_mapitems[0] = load_sprite("gfx/packs/Classic/menu/menu_mapitems_preview.png", colors::MAGENTA);
        rm.spr_thumbnail_mapitems[1] = load_sprite("gfx/packs/Classic/menu/menu_mapitems_thumbnail.png", colors::MAGENTA);

        rm.spr_tileanimation[1] = load_sprite("gfx/packs/Classic/tilesets/tile_animation_preview.png", colors::MAGENTA);
        rm.spr_tileanimation[2] = load_sprite("gfx/packs/Classic/tilesets/tile_animation_thumbnail.png", colors::MAGENTA);

        rm.spr_blocks[1] = load_sprite("gfx/packs/Classic/tilesets/blocks_preview.png", colors::MAGENTA);
        rm.spr_blocks[2] = load_sprite("gfx/packs/Classic/tilesets/blocks_thumbnail.png", colors::MAGENTA);

        rm.spr_unknowntile[1] = load_sprite("gfx/packs/Classic/tilesets/unknown_tile_preview.png", colors::MAGENTA);
        rm.spr_unknowntile[2] = load_sprite("gfx/packs/Classic/tilesets/unknown_tile_thumbnail.png", colors::MAGENTA);

        rm.spr_hazard_fireball[1] = load_sprite("gfx/packs/Classic/hazards/fireball_preview.png", colors::MAGENTA);
        rm.spr_hazard_fireball[2] = load_sprite("gfx/packs/Classic/hazards/fireball_thumbnail.png", colors::MAGENTA);

        rm.spr_hazard_rotodisc[1] = load_sprite("gfx/packs/Classic/hazards/rotodisc_preview.png", colors::MAGENTA);
        rm.spr_hazard_rotodisc[2] = load_sprite("gfx/packs/Classic/hazards/rotodisc_thumbnail.png", colors::MAGENTA);

        rm.spr_hazard_bulletbill[1] = load_sprite("gfx/packs/Classic/hazards/bulletbill_preview.png", colors::MAGENTA);
        rm.spr_hazard_bulletbill[2] = load_sprite("gfx/packs/Classic/hazards/bulletbill_thumbnail.png", colors::MAGENTA);

        rm.spr_hazard_flame[1] = load_sprite("gfx/packs/Classic/hazards/flame_preview.png", colors::MAGENTA);
        rm.spr_hazard_flame[2] = load_sprite("gfx/packs/Classic/hazards/flame_thumbnail.png", colors::MAGENTA);

        rm.spr_hazard_pirhanaplant[1] = load_sprite("gfx/packs/Classic/hazards/pirhanaplant_preview.png", colors::MAGENTA);
        rm.spr_hazard_pirhanaplant[2] = load_sprite("gfx/packs/Classic/hazards/pirhanaplant_thumbnail.png", colors::MAGENTA);

        rm.load_menu_graphics();

        sMapSurface = SDL_CreateRGBSurface((*screen).flags, 768, 608, (*(*screen).format).BitsPerPixel as i32, 0, 0, 0, 0);

        let fs = findstring.clone();
        worldlist.find(&fs);
        game_values.worldindex = worldlist.current_index() as i16;
        loadcurrentworld();
        findstring.clear(); //clear out the find string so that pressing "f" will give you the find dialog

        if saved_row >= 0 && saved_row <= iWorldHeight as i32 - 15 && saved_col >= 0 && saved_col <= iWorldWidth as i32 - 20 {
            draw_offset_row = saved_row;
            draw_offset_col = saved_col;
            updateworldsurface();
        }

        //Setup input for menus
        game_values.inputConfiguration[0][0].iDevice = DEVICE_KEYBOARD;
        for iInputState in 0..2usize {
            //for game/menu
            for iKey in 0..NUM_KEYS as usize {
                game_values.inputConfiguration[0][0].inputGameControls[iInputState].keys[iKey] = controlkeys[0][iInputState][0][iKey];
            }
        }

        game_values.playerInput.inputControls[0] = Ptr::from_mut(&mut game_values.inputConfiguration[0][0]);

        stagemodes = vec![StageMode { options: vec![StageModeOption::default(); GAMEMODE_NUM_OPTIONS - 1], ..Default::default() }; GAMEMODE_LAST as usize];

        set_stage_mode(0, "Classic", "Lives", 5, 10);
        set_stage_mode(1, "Frag", "Kills", 5, 20);
        set_stage_mode(2, "Time Limit", "Time", 30, 60);
        set_stage_mode(3, "Jail", "Kills", 5, 20);
        set_stage_mode(4, "Coin Collection", "Coins", 5, 20);
        set_stage_mode(5, "Stomp", "Kills", 10, 10);
        set_stage_mode(6, "Yoshi's Eggs", "Eggs", 5, 20);
        set_stage_mode(7, "Capture The Flag", "Points", 5, 20);
        set_stage_mode(8, "Chicken", "Points", 50, 200);
        set_stage_mode(9, "Tag", "Points", 50, 200);
        set_stage_mode(10, "Star", "Lives", 1, 5);
        set_stage_mode(11, "Domination", "Points", 50, 200);
        set_stage_mode(12, "King of the Hill", "Points", 50, 200);
        set_stage_mode(13, "Race", "Laps", 2, 10);
        set_stage_mode(14, "Owned", "Points", 50, 200);
        set_stage_mode(15, "Frenzy", "Kills", 5, 20);
        set_stage_mode(16, "Survival", "Lives", 5, 10);
        set_stage_mode(17, "Greed", "Coins", 10, 40);
        set_stage_mode(18, "Health", "Lives", 1, 5);
        set_stage_mode(19, "Card Collection", "Points", 10, 30);
        set_stage_mode(20, "Phanto Chase", "Points", 50, 200);
        set_stage_mode(21, "Shy Guy Tag", "Points", 50, 200);

        mStageSettingsMenu.init(UI_Menu::new());
        mBonusItemPicker.init(UI_Menu::new());
        mVehicleMenu.init(UI_Menu::new());

        //Setup The Mode Menu
        mCurrentMenu = mStageSettingsMenu.as_ptr();

        //Name
        miNameField = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 20, "Name", 500, 120));
        miNameField.set_disallowed_chars(",");

        miModeField = Ptr::new_box(MI_ImageSelectField::new(sprite(&mut rm.spr_selectfield), sprite(&mut rm.menu_mode_small), 70, 60, "Mode", 500, 120, 16, 16));
        //miModeField->SetData(game_values.tourstops[0]->iMode, NULL, NULL);
        //miModeField->SetKey(0);
        miModeField.set_item_changed_code(MENU_CODE_MODE_CHANGED);

        for iGameMode in 0..GAMEMODE_LAST as usize {
            miModeField.add(stagemodes[iGameMode].szName.clone(), iGameMode as i16);

            miGoalField[iGameMode] = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 100, stagemodes[iGameMode].szGoal.clone(), 352, 120));
            miGoalField[iGameMode].set_visible(iGameMode == 0);

            for iGameModeOption in 0..(GAMEMODE_NUM_OPTIONS - 1) {
                let option = stagemodes[iGameMode].options[iGameModeOption].clone();
                miGoalField[iGameMode].add(option.szName, option.iValue);
            }

            //miGoalField[iGameMode]->SetData(&gamemodes[iGameMode]->goal, NULL, NULL);
            //miGoalField[iGameMode]->SetKey(gamemodes[iGameMode]->goal);
        }

        miModeField.add("Bonus House", 24);
        miModeField.add("Pipe Minigame", 25);
        miModeField.add("Boss Minigame", 26);
        miModeField.add("Boxes Minigame", 27);

        //Create goal field for pipe game
        miSpecialGoalField[0] = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 100, "Points", 352, 120));
        miSpecialGoalField[0].set_visible(false);

        for iGameModeOption in 0..(GAMEMODE_NUM_OPTIONS as i16 - 1) {
            let iValue: i16 = 10 + iGameModeOption * 10;
            miSpecialGoalField[0].add(format!("{}", iValue), iValue);
        }

        //Create goal field for boss game
        miSpecialGoalField[1] = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 100, "Lives", 352, 120));
        miSpecialGoalField[1].set_visible(false);

        for iGameLives in 1..=30i16 {
            miSpecialGoalField[1].add(format!("{}", iGameLives), iGameLives);
        }

        //Create goal field for boxes game
        miSpecialGoalField[2] = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 100, "Lives", 352, 120));
        miSpecialGoalField[2].set_visible(false);

        for iGameLives in 1..=30i16 {
            miSpecialGoalField[2].add(format!("{}", iGameLives), iGameLives);
        }

        //Mode Settings Button
        miModeSettingsButton = Ptr::new_box(MI_Button::new_left(sprite(&mut rm.spr_selectfield), 430, 100, "Settings", 140));
        miModeSettingsButton.set_code(MENU_CODE_TO_MODE_SETTINGS_MENU);

        //Points Field
        miPointsField = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 140, "Points", 245, 120));
        for iPoints in 0..=20i16 {
            miPointsField.add(format!("{}", iPoints), iPoints);
        }

        //Final Stage Field
        miFinalStageField = Ptr::new_box(MI_SelectField::<bool>::new(sprite(&mut rm.spr_selectfield), 325, 140, "End Stage", 245, 120));
        miFinalStageField.add("No", false);
        miFinalStageField.add("Yes", true);
        miFinalStageField.set_auto_advance(true);

        //Map Select Field
        miMapField = Ptr::new_box(MI_MapField::new(sprite(&mut rm.spr_selectfield), 70, 180, "Map", 500, 120, true));

        //Bonus Item Picker Menu Button
        miBonusItemsButton = Ptr::new_box(MI_Button::new_left(sprite(&mut rm.spr_selectfield), 430, 220, "Bonuses", 140));
        miBonusItemsButton.set_code(MENU_CODE_TO_BONUS_PICKER_MENU);

        //Bonus Type
        miBonusType = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 100, "Type", 500, 120));
        miBonusType.add("Fixed", false as i16);
        miBonusType.add("Random", true as i16);
        miBonusType.set_auto_advance(true);

        //Bonus House Text * 5
        miBonusTextField[0] = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 140, "Text", 500, 120));
        miBonusTextField[1] = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 180, "Text", 500, 120));
        miBonusTextField[2] = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 220, "Text", 500, 120));
        miBonusTextField[3] = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 260, "Text", 500, 120));
        miBonusTextField[4] = Ptr::new_box(MI_TextField::new(sprite(&mut rm.spr_selectfield), 70, 300, "Text", 500, 120));

        miBonusTextField[0].set_disallowed_chars(",|");
        miBonusTextField[1].set_disallowed_chars(",|");
        miBonusTextField[2].set_disallowed_chars(",|");
        miBonusTextField[3].set_disallowed_chars(",|");
        miBonusTextField[4].set_disallowed_chars(",|");

        //Delete Stage Button
        miDeleteStageButton = Ptr::new_box(MI_Button::new_left(sprite(&mut rm.spr_selectfield), 430, 440, "Delete", 140));
        miDeleteStageButton.set_code(MENU_CODE_DELETE_STAGE_BUTTON);

        //Are You Sure Dialog for Delete Stage
        miDeleteStageDialogImage = Ptr::new_box(MI_Image::new(spr_dialog.as_ptr(), 224, 176, 0, 0, 192, 128, 1, 1, 0));
        miDeleteStageDialogAreYouText = Ptr::new_box(MI_HeaderText::new("Are You", 320, 195));
        miDeleteStageDialogSureText = Ptr::new_box(MI_HeaderText::new("Sure?", 320, 220));
        miDeleteStageDialogYesButton = Ptr::new_box(MI_Button::new(sprite(&mut rm.spr_selectfield), 235, 250, "Yes", 80, TextAlign::CENTER));
        miDeleteStageDialogNoButton = Ptr::new_box(MI_Button::new(sprite(&mut rm.spr_selectfield), 325, 250, "No", 80, TextAlign::CENTER));

        miDeleteStageDialogYesButton.set_code(MENU_CODE_DELETE_STAGE_YES);
        miDeleteStageDialogNoButton.set_code(MENU_CODE_DELETE_STAGE_NO);

        miDeleteStageDialogImage.set_visible(false);
        miDeleteStageDialogAreYouText.set_visible(false);
        miDeleteStageDialogSureText.set_visible(false);
        miDeleteStageDialogYesButton.set_visible(false);
        miDeleteStageDialogNoButton.set_visible(false);

        let null = Ptr::null();

        //Add Name Field
        mStageSettingsMenu.add_control(ctl_ptr(miNameField), ctl_ptr(miDeleteStageButton), ctl_ptr(miModeField), null, null);

        //Add Mode Field
        mStageSettingsMenu.add_control(ctl_ptr(miModeField), ctl_ptr(miNameField), ctl_ptr(miGoalField[0]), null, null);

        //Add Mode Goal Fields
        mStageSettingsMenu.add_control(ctl_ptr(miGoalField[0]), ctl_ptr(miModeField), ctl_ptr(miGoalField[1]), null, ctl_ptr(miModeSettingsButton));

        for iGoalField in 1..(GAMEMODE_LAST as usize - 1) {
            mStageSettingsMenu.add_control(
                ctl_ptr(miGoalField[iGoalField]),
                ctl_ptr(miGoalField[iGoalField - 1]),
                ctl_ptr(miGoalField[iGoalField + 1]),
                ctl_ptr(miGoalField[iGoalField - 1]),
                ctl_ptr(miModeSettingsButton),
            );
        }

        let last = GAMEMODE_LAST as usize - 1;
        mStageSettingsMenu.add_control(ctl_ptr(miGoalField[last]), ctl_ptr(miGoalField[last - 1]), ctl_ptr(miSpecialGoalField[0]), ctl_ptr(miGoalField[last - 1]), ctl_ptr(miModeSettingsButton));

        mStageSettingsMenu.add_control(ctl_ptr(miSpecialGoalField[0]), ctl_ptr(miGoalField[last]), ctl_ptr(miSpecialGoalField[1]), ctl_ptr(miGoalField[last]), ctl_ptr(miSpecialGoalField[1]));
        mStageSettingsMenu.add_control(ctl_ptr(miSpecialGoalField[1]), ctl_ptr(miSpecialGoalField[0]), ctl_ptr(miSpecialGoalField[2]), ctl_ptr(miSpecialGoalField[0]), ctl_ptr(miSpecialGoalField[2]));
        mStageSettingsMenu.add_control(ctl_ptr(miSpecialGoalField[2]), ctl_ptr(miSpecialGoalField[1]), ctl_ptr(miPointsField), ctl_ptr(miSpecialGoalField[1]), ctl_ptr(miModeSettingsButton));

        //Add Mode Settings Button
        mStageSettingsMenu.add_control(ctl_ptr(miModeSettingsButton), ctl_ptr(miModeField), ctl_ptr(miFinalStageField), ctl_ptr(miSpecialGoalField[2]), null);

        //Add Points Field
        mStageSettingsMenu.add_control(ctl_ptr(miPointsField), ctl_ptr(miSpecialGoalField[2]), ctl_ptr(miMapField), null, ctl_ptr(miFinalStageField));

        //Add Final Stage Field
        mStageSettingsMenu.add_control(ctl_ptr(miFinalStageField), ctl_ptr(miSpecialGoalField[2]), ctl_ptr(miMapField), ctl_ptr(miPointsField), null);

        //Add Map Field
        mStageSettingsMenu.add_control(ctl_ptr(miMapField), ctl_ptr(miFinalStageField), ctl_ptr(miBonusType), null, ctl_ptr(miBonusItemsButton));

        //Add Bonus House Fields
        mStageSettingsMenu.add_control(ctl_ptr(miBonusType), ctl_ptr(miMapField), ctl_ptr(miBonusTextField[0]), null, null);
        mStageSettingsMenu.add_control(ctl_ptr(miBonusTextField[0]), ctl_ptr(miBonusType), ctl_ptr(miBonusTextField[1]), null, null);
        mStageSettingsMenu.add_control(ctl_ptr(miBonusTextField[1]), ctl_ptr(miBonusTextField[0]), ctl_ptr(miBonusTextField[2]), null, null);
        mStageSettingsMenu.add_control(ctl_ptr(miBonusTextField[2]), ctl_ptr(miBonusTextField[1]), ctl_ptr(miBonusTextField[3]), null, null);
        mStageSettingsMenu.add_control(ctl_ptr(miBonusTextField[3]), ctl_ptr(miBonusTextField[2]), ctl_ptr(miBonusTextField[4]), null, null);
        mStageSettingsMenu.add_control(ctl_ptr(miBonusTextField[4]), ctl_ptr(miBonusTextField[3]), ctl_ptr(miBonusItemsButton), null, null);

        //Add Bonus Button
        mStageSettingsMenu.add_control(ctl_ptr(miBonusItemsButton), ctl_ptr(miBonusTextField[4]), ctl_ptr(miDeleteStageButton), ctl_ptr(miMapField), null);

        //Add Delete Stage Button
        mStageSettingsMenu.add_control(ctl_ptr(miDeleteStageButton), ctl_ptr(miBonusItemsButton), ctl_ptr(miNameField), ctl_ptr(miMapField), null);

        //Add Are You Sure Dialog
        mStageSettingsMenu.add_non_control(ctl_ptr(miDeleteStageDialogImage));
        mStageSettingsMenu.add_non_control(ctl_ptr(miDeleteStageDialogAreYouText));
        mStageSettingsMenu.add_non_control(ctl_ptr(miDeleteStageDialogSureText));

        mStageSettingsMenu.add_control(ctl_ptr(miDeleteStageDialogYesButton), null, null, null, ctl_ptr(miDeleteStageDialogNoButton));
        mStageSettingsMenu.add_control(ctl_ptr(miDeleteStageDialogNoButton), null, null, ctl_ptr(miDeleteStageDialogYesButton), null);

        mStageSettingsMenu.set_initial_focus(ctl_ptr(miNameField));
        mStageSettingsMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        mBonusItemPicker.set_cancel_code(MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS);

        mModeOptionsMenu = Ptr::from_box(UI_ModeOptionsMenu::new());

        g_wvVehicleStamp.init(WorldVehicle::new());
        g_wvVehicleStamp.iDrawSprite = 0;
        g_wvVehicleStamp.iActionId = 0;
        g_wvVehicleStamp.currentTile.x = 0;
        g_wvVehicleStamp.currentTile.y = 0;
        g_wvVehicleStamp.iMinMoves = 5;
        g_wvVehicleStamp.iMaxMoves = 8;
        g_wvVehicleStamp.fSpritePaces = true;
        g_wvVehicleStamp.iDrawDirection = 0;
        g_wvVehicleStamp.iBoundary = 0;

        //Create Vehicle Menu
        miVehicleSpriteField = Ptr::new_box(MI_ImageSelectField::new(sprite(&mut rm.spr_selectfield), spr_vehicleicons.as_ptr(), 70, 80, "Sprite", 500, 150, 16, 16));
        miVehicleSpriteField.add("Hammer Brother", 0);
        miVehicleSpriteField.add("Boomerang Brother", 1);
        miVehicleSpriteField.add("Fire Brother", 2);
        miVehicleSpriteField.add("Tank 1", 3);
        miVehicleSpriteField.add("Boat 1", 4);
        miVehicleSpriteField.add("Boat 2", 5);
        miVehicleSpriteField.add("Airship 1", 6);
        miVehicleSpriteField.add("Airship 2", 7);
        miVehicleSpriteField.add("Tank 2", 8);
        miVehicleSpriteField.set_output_ptr(&mut g_wvVehicleStamp.iDrawSprite as *mut i16);
        miVehicleSpriteField.set_current_value(g_wvVehicleStamp.iDrawSprite);

        miVehicleStageField = Ptr::new_box(MI_ImageSelectField::new(sprite(&mut rm.spr_selectfield), sprite(&mut rm.menu_mode_small), 70, 120, "Stage", 500, 150, 16, 16));
        miVehicleStageField.set_output_ptr(&mut g_wvVehicleStamp.iActionId as *mut i16);

        miVehicleMinMovesField = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 160, "Min Moves", 500, 150));

        for iMinMoves in 0..=100i16 {
            miVehicleMinMovesField.add(format!("{}", iMinMoves), iMinMoves);
        }

        miVehicleMinMovesField.set_output_ptr(&mut g_wvVehicleStamp.iMinMoves as *mut i16);
        miVehicleMinMovesField.set_current_value(g_wvVehicleStamp.iMinMoves);
        miVehicleMinMovesField.set_item_changed_code(MENU_CODE_VEHICLE_MIN_MOVES_CHANGED);
        miVehicleMinMovesField.allow_wrap(false);
        miVehicleMinMovesField.allow_fast_scroll(true);

        miVehicleMaxMovesField = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 200, "Max Moves", 500, 150));

        for iMaxMoves in 0..=100i16 {
            miVehicleMaxMovesField.add(format!("{}", iMaxMoves), iMaxMoves);
        }

        miVehicleMaxMovesField.set_output_ptr(&mut g_wvVehicleStamp.iMaxMoves as *mut i16);
        miVehicleMaxMovesField.set_current_value(g_wvVehicleStamp.iMaxMoves);
        miVehicleMaxMovesField.set_item_changed_code(MENU_CODE_VEHICLE_MAX_MOVES_CHANGED);
        miVehicleMaxMovesField.allow_wrap(false);
        miVehicleMaxMovesField.allow_fast_scroll(true);

        miVehiclePacesField = Ptr::new_box(MI_SelectField::<bool>::new(sprite(&mut rm.spr_selectfield), 70, 240, "Paces", 500, 150));
        miVehiclePacesField.add("No", false);
        miVehiclePacesField.add("Yes", true);
        miVehiclePacesField.set_output_ptr(&mut g_wvVehicleStamp.fSpritePaces as *mut bool);
        miVehiclePacesField.set_current_value(if g_wvVehicleStamp.fSpritePaces { 1 } else { 0 } != 0);
        miVehiclePacesField.set_auto_advance(true);

        miVehicleDirectionField = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 280, "Direction", 500, 150));
        miVehicleDirectionField.add("Left", false as i16);
        miVehicleDirectionField.add("Right", true as i16);
        miVehicleDirectionField.set_output_ptr(&mut g_wvVehicleStamp.iDrawDirection as *mut i16);
        miVehicleDirectionField.set_current_value(g_wvVehicleStamp.iDrawDirection);
        miVehicleDirectionField.set_auto_advance(true);

        miVehicleBoundaryField = Ptr::new_box(MI_SelectField::<i16>::new(sprite(&mut rm.spr_selectfield), 70, 320, "Boundary", 500, 150));
        miVehicleBoundaryField.add("No Boundary", 0);

        for iBoundary in 1..=100i16 {
            miVehicleBoundaryField.add(format!("{}", iBoundary), iBoundary);
        }

        miVehicleBoundaryField.set_output_ptr(&mut g_wvVehicleStamp.iBoundary as *mut i16);
        miVehicleBoundaryField.set_current_value(g_wvVehicleStamp.iBoundary);
        miVehicleBoundaryField.allow_fast_scroll(true);

        miVehicleCreateButton = Ptr::new_box(MI_Button::new(sprite(&mut rm.spr_selectfield), 430, 360, "OK", 140, TextAlign::CENTER));
        miVehicleCreateButton.set_code(MENU_CODE_CREATE_VEHICLE);

        miTitleText = Ptr::new_box(MI_Text::new("Clicking on the map will add the vehicle configured below", 320, 50, 640, true, TextAlign::CENTER));

        mVehicleMenu.add_non_control(ctl_ptr(miTitleText));

        mVehicleMenu.add_control(ctl_ptr(miVehicleSpriteField), ctl_ptr(miVehicleCreateButton), ctl_ptr(miVehicleStageField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleStageField), ctl_ptr(miVehicleSpriteField), ctl_ptr(miVehicleMinMovesField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleMinMovesField), ctl_ptr(miVehicleStageField), ctl_ptr(miVehicleMaxMovesField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleMaxMovesField), ctl_ptr(miVehicleMinMovesField), ctl_ptr(miVehiclePacesField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehiclePacesField), ctl_ptr(miVehicleMaxMovesField), ctl_ptr(miVehicleDirectionField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleDirectionField), ctl_ptr(miVehiclePacesField), ctl_ptr(miVehicleBoundaryField), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleBoundaryField), ctl_ptr(miVehicleDirectionField), ctl_ptr(miVehicleCreateButton), null, null);
        mVehicleMenu.add_control(ctl_ptr(miVehicleCreateButton), ctl_ptr(miVehicleBoundaryField), ctl_ptr(miVehicleSpriteField), null, null);

        mVehicleMenu.set_initial_focus(ctl_ptr(miVehicleSpriteField));
        mVehicleMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);

        println!("\n---------------- ready, steady, go! ----------------");

        println!("entering world editor loop...");
        done = false;
        while !done {
            match state {
                EDITOR_EDIT => state = editor_edit(),
                EDITOR_WATER => state = editor_water(),
                EDITOR_BACKGROUND => state = editor_background(),
                EDITOR_STAGEFOREGROUND => state = editor_stageforeground(),
                EDITOR_STRUCTUREFOREGROUND => state = editor_structureforeground(),
                EDITOR_BRIDGES => state = editor_bridges(),
                EDITOR_VEHICLES => state = editor_vehicles(),
                EDITOR_PATH => state = editor_path(),
                EDITOR_PATHSPRITE => state = editor_pathsprite(),
                EDITOR_WARP => state = editor_warp(),
                EDITOR_START_ITEMS => state = editor_start_items(),
                EDITOR_BOUNDARY => state = editor_boundary(),
                EDITOR_TYPE => state = editor_type(),
                EDITOR_STAGE => state = editor_stage(),
                EDITOR_QUIT => done = true,
                DISPLAY_HELP => state = display_help(),
                SAVE_AS => state = save_as(),
                FIND => state = find(),
                CLEAR_WORLD => state = clear_world(),
                NEW_WORLD => state = new_world(),
                RESIZE_WORLD => state = resize_world(),
                SAVE => state = savecurrentworld(),
                _ => println!(" PANIC: WEIRD GAMESTATE: {}", state),
            }
        }

        SDL_FreeSurface(sMapSurface);

        println!("\n---------------- save world ----------------");

        WriteVehiclesIntoWorld();
        WriteWarpsIntoWorld();
        g_worldmap.save(&convert_path("worlds/ZZworldeditor.txt"));

        {
            let options_path = get_home_directory() + "worldeditor.bin";
            let mut editor_settings = BinaryFile::new(&options_path, "wb");
            if editor_settings.is_open() {
                editor_settings.write_i32(draw_offset_col);
                editor_settings.write_i32(draw_offset_row);
                editor_settings.write_bool(g_fFullScreen);
                editor_settings.write_string_long(&worldlist.current_path().to_string_lossy());
            }
        }

        println!("\n---------------- shutdown ----------------");
    }
}

/// Painting with the left button (`SDL_MOUSEBUTTONDOWN` and `SDL_MOUSEMOTION` repeat the same block in C++).
fn paint_left(iCol: i16, iRow: i16, fromMotion: bool) {
    unsafe {
        if edit_mode == 0 {
            //selected background
            if tile(iCol, iRow).iBackgroundSprite as i32 != set_tile || fAutoPaint {
                let mut fNeedUpdate = false;
                if tile(iCol, iRow).iBackgroundSprite as i32 != set_tile {
                    tile(iCol, iRow).iBackgroundSprite = set_tile as i16;
                    fNeedUpdate = true;
                }

                fNeedUpdate |= UpdateForeground(iCol, iRow);

                if (set_tile % WORLD_BACKGROUND_SPRITE_SET_SIZE as i32) < 2 && fAutoPaint {
                    fNeedUpdate |= UpdateCoastline(iCol, iRow);
                }

                if fNeedUpdate {
                    updateworldsurface();
                }
            }
        } else if edit_mode == 1 {
            //selected foreground
            if tile(iCol, iRow).iForegroundSprite as i32 != set_tile {
                tile(iCol, iRow).iForegroundSprite = set_tile as i16;
                updateworldsurface();

                if !fromMotion && set_tile >= WORLD_BRIDGE_SPRITE_OFFSET as i32 && set_tile <= WORLD_BRIDGE_SPRITE_OFFSET as i32 + 3 {
                    tile(iCol, iRow).iConnectionType = (set_tile - WORLD_BRIDGE_SPRITE_OFFSET as i32 + 12) as i16;
                }
            }
        } else if edit_mode == 2 {
            //selected connection
            tile(iCol, iRow).iConnectionType = set_tile as i16;

            if fAutoPaint {
                UpdatePath(iCol, iRow);
            }
        } else if edit_mode == 3 {
            //selected type
            //start tiles
            if set_tile <= 1 {
                if tile(iCol, iRow).iForegroundSprite as i32 != set_tile + WORLD_START_SPRITE_OFFSET as i32 {
                    tile(iCol, iRow).iType = 1;
                    tile(iCol, iRow).iForegroundSprite = (set_tile + WORLD_START_SPRITE_OFFSET as i32) as i16;
                    updateworldsurface();
                }
            } else if set_tile <= 5 {
                //doors
                if tile(iCol, iRow).iType as i32 != set_tile {
                    //if the door was placed on a start tile
                    if tile(iCol, iRow).iType == 1 {
                        tile(iCol, iRow).iForegroundSprite = 0;
                    }

                    tile(iCol, iRow).iType = set_tile as i16;
                    updateworldsurface();
                }
            }
        } else if edit_mode == 4 {
            //selected path sprite
            let iAdjustedTile = AdjustForeground(set_tile as i16, iCol, iRow);
            let mut fNeedUpdate = false;

            if !fAutoPaint && tile(iCol, iRow).iForegroundSprite != iAdjustedTile {
                tile(iCol, iRow).iForegroundSprite = iAdjustedTile;
                fNeedUpdate = true;
            }

            //Detect if there was a change so we can repaint the screen
            if fAutoPaint {
                let mut iOldTiles = [0i16; 9];
                GetForegroundTileValues(iCol, iRow, &mut iOldTiles);
                tile(iCol, iRow).iForegroundSprite = iAdjustedTile;
                UpdatePathSprite(iCol, iRow);

                if ForegroundTileValuesChanged(iCol, iRow, &iOldTiles) {
                    fNeedUpdate = true;
                }
            }

            if fNeedUpdate {
                updateworldsurface();
            }
        } else if edit_mode == 5 {
            //selected vehicle
            AddVehicleToTile(iCol, iRow, set_tile as i16);
        } else if edit_mode == 6 {
            //selected warp
            AddWarpToTile(iCol, iRow, set_tile as i16);
        } else if edit_mode == 7 {
            //water
            if tile(iCol, iRow).iBackgroundWater as i32 != set_tile {
                tile(iCol, iRow).iBackgroundWater = set_tile as i16;
                updateworldsurface();
            }
        } else if edit_mode == 8 {
            //boundary
            if !fromMotion {
                tile(iCol, iRow).iVehicleBoundary = set_tile as i16;
            }
        } else if edit_mode == 9 {
            //if the stage was placed on a start tile
            if tile(iCol, iRow).iType == 1 {
                tile(iCol, iRow).iForegroundSprite = 0;
                updateworldsurface();
            }

            tile(iCol, iRow).iType = set_tile as i16;
        }
    }
}

/// Erasing with the right button; returns the new `iStageDisplay` when C++ resets it.
fn paint_right(iCol: i16, iRow: i16, fromMotion: bool, iStageDisplay: &mut i16) {
    unsafe {
        if edit_mode == 0 {
            if tile(iCol, iRow).iBackgroundSprite != 0 || fAutoPaint {
                let mut fNeedUpdate = false;

                if tile(iCol, iRow).iBackgroundSprite != 0 {
                    tile(iCol, iRow).iBackgroundSprite = 0;
                    fNeedUpdate = true;
                }

                fNeedUpdate |= UpdateForeground(iCol, iRow);

                if fAutoPaint {
                    fNeedUpdate |= UpdateCoastline(iCol, iRow);
                }

                if fNeedUpdate {
                    updateworldsurface();
                }
            }
        } else if edit_mode == 1 {
            if tile(iCol, iRow).iForegroundSprite != 0 {
                tile(iCol, iRow).iForegroundSprite = 0;
                updateworldsurface();
            }
        } else if edit_mode == 2 {
            tile(iCol, iRow).iConnectionType = 0;

            if fAutoPaint {
                UpdatePath(iCol, iRow);
            }
        } else if edit_mode == 3 {
            //selected start/door
            if tile(iCol, iRow).iType == 1 {
                tile(iCol, iRow).iForegroundSprite = 0;
                updateworldsurface();
            } else if tile(iCol, iRow).iType <= 5 {
                tile(iCol, iRow).iType = 0;
                updateworldsurface();
            }

            tile(iCol, iRow).iType = 0;
        } else if edit_mode == 4 {
            let mut fNeedUpdate = false;

            if !fAutoPaint && tile(iCol, iRow).iForegroundSprite != 0 {
                tile(iCol, iRow).iForegroundSprite = 0;
                fNeedUpdate = true;
            }

            //Detect if there was a change so we can repaint the screen
            if fAutoPaint {
                let mut iOldTiles = [0i16; 9];
                GetForegroundTileValues(iCol, iRow, &mut iOldTiles);
                tile(iCol, iRow).iForegroundSprite = 0;
                UpdatePathSprite(iCol, iRow);

                if ForegroundTileValuesChanged(iCol, iRow, &iOldTiles) {
                    fNeedUpdate = true;
                }
            }

            if fNeedUpdate {
                updateworldsurface();
            }
        } else if edit_mode == 5 {
            //vehicles
            RemoveVehicleFromTile(iCol, iRow);
            *iStageDisplay = -1;
        } else if edit_mode == 6 {
            //Warps
            RemoveWarpFromTile(iCol, iRow);
        } else if edit_mode == 7 {
            //water
            if tile(iCol, iRow).iBackgroundWater != 0 {
                tile(iCol, iRow).iBackgroundWater = 0;
                updateworldsurface();
            }
        } else if edit_mode == 8 {
            //boundary
            if !fromMotion {
                tile(iCol, iRow).iVehicleBoundary = 0;
            }
        } else if edit_mode == 9 {
            //stage
            tile(iCol, iRow).iType = 0;
            *iStageDisplay = -1;
        }
    }
}

fn copy_vehicle_into_stamp(vehicle: &WorldVehicle) {
    unsafe {
        g_wvVehicleStamp.iDrawSprite = vehicle.iDrawSprite;
        g_wvVehicleStamp.iActionId = vehicle.iActionId;
        g_wvVehicleStamp.iMinMoves = vehicle.iMinMoves;
        g_wvVehicleStamp.iMaxMoves = vehicle.iMaxMoves;
        g_wvVehicleStamp.fSpritePaces = vehicle.fSpritePaces;
        g_wvVehicleStamp.iDrawDirection = vehicle.iDrawDirection;
        g_wvVehicleStamp.iBoundary = vehicle.iBoundary;

        miVehicleSpriteField.set_current_value(g_wvVehicleStamp.iDrawSprite);
        miVehicleStageField.set_current_value(g_wvVehicleStamp.iActionId);
        miVehicleMinMovesField.set_current_value(g_wvVehicleStamp.iMinMoves);
        miVehicleMaxMovesField.set_current_value(g_wvVehicleStamp.iMaxMoves);
        miVehiclePacesField.set_current_value((if g_wvVehicleStamp.fSpritePaces { 1 } else { 0 }) != 0);
        miVehicleDirectionField.set_current_value(g_wvVehicleStamp.iDrawDirection);
        miVehicleBoundaryField.set_current_value(g_wvVehicleStamp.iBoundary);
    }
}

pub fn editor_edit() -> i32 {
    unsafe {
        let mut done = false;
        g_musiccategorydisplaytimer = 0;

        let mut view_repeat_direction: i16 = -1;
        let mut view_repeat_timer: i16 = 0;

        let mut fExiting = false;
        let mut fSelectedYes = false;

        let mut iStageDisplay: i16 = -1;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            if fExiting {
                //handle messages
                while SDL_PollEvent(&mut event) != 0 {
                    if event_type() == T_KEYDOWN {
                        let key = key_sym();

                        if key == SDLK_LEFT as i32 {
                            fSelectedYes = true;
                        } else if key == SDLK_RIGHT as i32 {
                            fSelectedYes = false;
                        } else if key_sym() == SDLK_KP_ENTER as i32 || key_sym() == SDLK_RETURN as i32 {
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
                    match event_type() {
                        T_QUIT => {
                            done = true;
                        }

                        T_KEYDOWN => {
                            let keystate = SDL_GetKeyboardState(null_mut());
                            if key_sym() == SDLK_ESCAPE as i32 {
                                if g_musiccategorydisplaytimer > 0 {
                                    g_musiccategorydisplaytimer = 0;
                                } else if edit_mode != 0 {
                                    edit_mode = 0;
                                    set_tile = 0;
                                } else {
                                    fSelectedYes = false;
                                    fExiting = true;
                                }
                            }

                            if event.key.keysym.mod_ & (SDL_Keymod::KMOD_LALT as u16 | SDL_Keymod::KMOD_RALT as u16) != 0 {
                                if key_sym() == SDLK_RETURN as i32 {
                                    g_fFullScreen = !g_fFullScreen;
                                    gfx_changefullscreen(g_fFullScreen);
                                    blitdest = screen;
                                }
                            }

                            if key_sym() == SDLK_INSERT as i32 {
                                takescreenshot();
                            }

                            if key_sym() == SDLK_1 as i32 {
                                return EDITOR_WATER;
                            }

                            if key_sym() == SDLK_2 as i32 {
                                return EDITOR_BACKGROUND;
                            }

                            if key_sym() == SDLK_3 as i32 {
                                return EDITOR_STAGEFOREGROUND;
                            }

                            if key_sym() == SDLK_4 as i32 {
                                return EDITOR_PATHSPRITE;
                            }

                            if key_sym() == SDLK_5 as i32 {
                                return EDITOR_STRUCTUREFOREGROUND;
                            }

                            if key_sym() == SDLK_6 as i32 {
                                return EDITOR_BRIDGES;
                            }

                            if key_sym() == SDLK_t as i32 {
                                return EDITOR_TYPE;
                            }

                            if key_sym() == SDLK_e as i32 {
                                return EDITOR_STAGE;
                            }

                            if key_sym() == SDLK_p as i32 {
                                return EDITOR_PATH;
                            }

                            if key_sym() == SDLK_v as i32 {
                                return EDITOR_VEHICLES;
                            }

                            if key_sym() == SDLK_SPACE as i32 {
                                g_fShowStagePreviews = !g_fShowStagePreviews;
                                if g_fShowStagePreviews {
                                    SetDisplayMessage(60, "Stage Previews", "Preview popups", "have been", "enabled.");
                                } else {
                                    SetDisplayMessage(60, "Stage Previews", "Preview popups", "have been", "disabled.");
                                }
                            }

                            if edit_mode == 5 && key_sym() == SDLK_c as i32 {
                                let iButtonX: i16 = (mouse_x - draw_offset_x) as i16;
                                let iButtonY: i16 = (mouse_y - draw_offset_y) as i16;
                                let iCol: i16 = (iButtonX as i32 / TILESIZE + draw_offset_col) as i16;
                                let iRow: i16 = (iButtonY as i32 / TILESIZE + draw_offset_row) as i16;

                                for vehicle in vehiclelist.iter() {
                                    if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                                        copy_vehicle_into_stamp(vehicle);
                                        return EDITOR_VEHICLES;
                                    }
                                }
                            }

                            if key_sym() == SDLK_w as i32 {
                                return EDITOR_WARP;
                            }

                            if key_sym() == SDLK_i as i32 {
                                return EDITOR_START_ITEMS;
                            }

                            if key_sym() == SDLK_b as i32 {
                                return EDITOR_BOUNDARY;
                            }

                            if key_sym() == SDLK_a as i32 {
                                fAutoPaint = !fAutoPaint;
                            }

                            if key_sym() == SDLK_r as i32 {
                                if g_musiccategorydisplaytimer > 0 {
                                    g_worldmap.iMusicCategory.post_increment();
                                }

                                g_musiccategorydisplaytimer = 90;
                            }

                            if key_sym() == SDLK_s as i32 {
                                if check_key(keystate, SDLK_LSHIFT as i32) || check_key(keystate, SDLK_RSHIFT as i32) {
                                    return SAVE_AS;
                                }

                                return SAVE;
                            }

                            if key_sym() == SDLK_f as i32 {
                                if check_key(keystate, SDLK_LSHIFT as i32) || check_key(keystate, SDLK_RSHIFT as i32) || findstring.is_empty() {
                                    return FIND;
                                }

                                findcurrentstring();
                            }

                            if key_sym() == SDLK_DELETE as i32 && (check_key(keystate, SDLK_LCTRL as i32) || check_key(keystate, SDLK_RCTRL as i32)) {
                                return CLEAR_WORLD;
                            }

                            if key_sym() == SDLK_n as i32 {
                                return NEW_WORLD;
                            }

                            if key_sym() == SDLK_k as i32 {
                                return RESIZE_WORLD;
                            }

                            if key_sym() == SDLK_h as i32 || key_sym() == SDLK_F1 as i32 {
                                return DISPLAY_HELP;
                            }

                            if key_sym() == SDLK_UP as i32 {
                                if draw_offset_row > 0 {
                                    draw_offset_row -= 1;
                                    updateworldsurface();

                                    view_repeat_direction = 0;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym() == SDLK_DOWN as i32 {
                                if draw_offset_row < iWorldHeight as i32 - 15 {
                                    draw_offset_row += 1;
                                    updateworldsurface();

                                    view_repeat_direction = 1;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym() == SDLK_LEFT as i32 {
                                if draw_offset_col > 0 {
                                    draw_offset_col -= 1;
                                    updateworldsurface();

                                    view_repeat_direction = 2;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            } else if key_sym() == SDLK_RIGHT as i32 {
                                if draw_offset_col < iWorldWidth as i32 - 20 {
                                    draw_offset_col += 1;
                                    updateworldsurface();

                                    view_repeat_direction = 3;
                                    view_repeat_timer = 30;

                                    iStageDisplay = -1;
                                }
                            }

                            if key_sym() == SDLK_PAGEUP as i32 {
                                game_values.worldindex -= 1;
                                if game_values.worldindex < 0 {
                                    game_values.worldindex = worldlist.count() as i16 - 1;
                                }

                                worldlist.prev();

                                loadcurrentworld();

                                iOldStageId = -1;
                            }

                            if key_sym() == SDLK_PAGEDOWN as i32 {
                                game_values.worldindex += 1;
                                if game_values.worldindex as usize >= worldlist.count() {
                                    game_values.worldindex = 0;
                                }

                                worldlist.next();

                                loadcurrentworld();

                                iOldStageId = -1;
                            }
                        }

                        T_KEYUP => {
                            if key_sym() == SDLK_UP as i32 || key_sym() == SDLK_DOWN as i32 || key_sym() == SDLK_LEFT as i32 || key_sym() == SDLK_RIGHT as i32 {
                                view_repeat_direction = -1;
                                view_repeat_timer = 0;
                            }
                        }

                        T_MOUSEBUTTONDOWN => {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) - draw_offset_x) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) - draw_offset_y) as i16;
                            let iCol: i16 = (iButtonX as i32 / TILESIZE + draw_offset_col) as i16;
                            let iRow: i16 = (iButtonY as i32 / TILESIZE + draw_offset_row) as i16;

                            if iButtonX >= 0 && iButtonY >= 0 && (iButtonX as i32) < iWorldWidth as i32 * TILESIZE && (iButtonY as i32) < iWorldHeight as i32 * TILESIZE {
                                if event.button.button == BUTTON_LEFT && !ignoreclick {
                                    paint_left(iCol, iRow, false);
                                } else if event.button.button == BUTTON_RIGHT {
                                    paint_right(iCol, iRow, false, &mut iStageDisplay);
                                }
                            }
                        }
                        //Painting tiles with mouse movement
                        T_MOUSEMOTION => {
                            bound_mouse_motion_coords();
                            let iButtonX: i16 = (bound_to_window_w(event.motion.x) - draw_offset_x) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.motion.y) - draw_offset_y) as i16;
                            let iCol: i16 = ((iButtonX as i32 >> 5) + draw_offset_col) as i16;
                            let iRow: i16 = ((iButtonY as i32 >> 5) + draw_offset_row) as i16;

                            if iButtonX >= 0 && iButtonY >= 0 && (iButtonX as i32) < iWorldWidth as i32 * TILESIZE && (iButtonY as i32) < iWorldHeight as i32 * TILESIZE {
                                if event.motion.state == SDL_BUTTON_LMASK && !ignoreclick {
                                    paint_left(iCol, iRow, true);
                                } else if event.motion.state == SDL_BUTTON_RMASK {
                                    paint_right(iCol, iRow, true, &mut iStageDisplay);
                                }
                            }

                            //Scan to see if we are mousing over a stage
                            if edit_mode == 5 {
                                iStageDisplay = -1;

                                for vehicle in vehiclelist.iter() {
                                    if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                                        iStageDisplay = vehicle.iActionId;
                                        break;
                                    }
                                }
                            } else if edit_mode == 9 {
                                iStageDisplay = -1;
                                if iCol >= 0 && iRow >= 0 && iCol < iWorldWidth && iRow < iWorldHeight {
                                    let iType: i16 = tile(iCol, iRow).iType - 6;
                                    if iType >= 0 {
                                        iStageDisplay = iType;
                                    }
                                }
                            }
                        }

                        T_MOUSEBUTTONUP => {
                            if event.button.button == BUTTON_LEFT {
                                ignoreclick = false;
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

                        if view_repeat_direction == 0 && draw_offset_row > 0 {
                            draw_offset_row -= 1;
                            updateworldsurface();
                        } else if view_repeat_direction == 1 && draw_offset_row < iWorldHeight as i32 - 15 {
                            draw_offset_row += 1;
                            updateworldsurface();
                        } else if view_repeat_direction == 2 && draw_offset_col > 0 {
                            draw_offset_col -= 1;
                            updateworldsurface();
                        } else if view_repeat_direction == 3 && draw_offset_col < iWorldWidth as i32 - 20 {
                            draw_offset_col += 1;
                            updateworldsurface();
                        }
                    }
                }
            }

            drawmap(false, TILESIZE as i16);

            //Ask if you are sure you want to exit
            if fExiting {
                spr_dialog.draw_src(224, 176, &r(0, 0, 192, 128));
                rm.menu_font_large.draw_centered(320, 195, "Exit");
                rm.menu_font_large.draw_centered(320, 220, "Are You Sure?");
                rm.menu_font_large.draw_centered(282, 254, "Yes");
                rm.menu_font_large.draw_centered(356, 254, "No");

                spr_dialog.draw_src(if fSelectedYes { 250 } else { 326 }, 250, &r(192, 0, 64, 32));
            } else {
                //Draw Paths
                if edit_mode == 2 {
                    let mut iRow = draw_offset_row as i16;
                    while (iRow as i32) < draw_offset_row + 15 && iRow < iWorldHeight {
                        let mut iCol = draw_offset_col as i16;
                        while iCol as i32 <= draw_offset_col + 20 && iCol < iWorldWidth {
                            let iConnection: i16 = tile(iCol, iRow).iConnectionType;

                            if iConnection > 0 {
                                spr_path.draw_src(
                                    (iCol as i32 - draw_offset_col) * TILESIZE + draw_offset_x,
                                    (iRow as i32 - draw_offset_row) * TILESIZE + draw_offset_y,
                                    &r((iConnection as i32 - 1) << 5, 0, TILESIZE, TILESIZE),
                                );
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }
                } else if edit_mode == 6 {
                    //draw warps
                    for warp in warplist.iter() {
                        let mut ix: i16;
                        let mut iy: i16;

                        if warp.posA.x >= 0 {
                            ix = ((warp.posA.x as i32 - draw_offset_col) * TILESIZE + draw_offset_x) as i16;
                            iy = ((warp.posA.y as i32 - draw_offset_row) * TILESIZE + draw_offset_y) as i16;

                            spr_warps[0].draw_src(ix as i32, iy as i32, &r((warp.id as i32) << 5, 0, 32, 32));
                        }

                        if warp.posB.x >= 0 {
                            ix = ((warp.posB.x as i32 - draw_offset_col) * TILESIZE + draw_offset_x) as i16;
                            iy = ((warp.posB.y as i32 - draw_offset_row) * TILESIZE + draw_offset_y) as i16;

                            spr_warps[0].draw_src(ix as i32, iy as i32, &r((warp.id as i32) << 5, 0, 32, 32));
                        }
                    }
                } else if edit_mode == 8 {
                    //draw boundaries
                    let color = SDL_MapRGB((*blitdest).format, 255, 0, 255);
                    let mut iRow = draw_offset_row as i16;
                    while (iRow as i32) < draw_offset_row + 15 && iRow < iWorldHeight {
                        let mut iCol = draw_offset_col as i16;
                        while iCol as i32 <= draw_offset_col + 20 && iCol < iWorldWidth {
                            let iBoundary: i16 = tile(iCol, iRow).iVehicleBoundary - 1;

                            if iBoundary >= 0 {
                                let ix: i16 = ((iCol as i32 - draw_offset_col) * TILESIZE + draw_offset_x) as i16;
                                let iy: i16 = ((iRow as i32 - draw_offset_row) * TILESIZE + draw_offset_y) as i16;
                                let mut rr = r(ix as i32, iy as i32, 32, 32);
                                SDL_FillRect(blitdest, &mut rr, color);

                                rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(((iBoundary % 10) as i32) << 5, ((iBoundary / 10) as i32) << 5, 32, 32));
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }
                } else if edit_mode == 9 {
                    //draw stages
                    let color = SDL_MapRGB((*blitdest).format, 0, 0, 255);
                    let mut iRow = draw_offset_row as i16;
                    while (iRow as i32) < draw_offset_row + 15 && iRow < iWorldHeight {
                        let mut iCol = draw_offset_col as i16;
                        while iCol as i32 <= draw_offset_col + 20 && iCol < iWorldWidth {
                            let iType: i16 = tile(iCol, iRow).iType - 6;

                            if iType >= 0 {
                                let ix: i16 = ((iCol as i32 - draw_offset_col) * TILESIZE + draw_offset_x) as i16;
                                let iy: i16 = ((iRow as i32 - draw_offset_row) * TILESIZE + draw_offset_y) as i16;
                                let mut rr = r(ix as i32, iy as i32, 32, 32);
                                SDL_FillRect(blitdest, &mut rr, color);

                                rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(((iType % 10) as i32) << 5, ((iType / 10) as i32) << 5, 32, 32));
                            }
                            iCol += 1;
                        }
                        iRow += 1;
                    }

                    if iStageDisplay >= 0 && g_fShowStagePreviews {
                        DisplayStageDetails(false, iStageDisplay, mouse_x as i16, mouse_y as i16);
                    }
                }

                if edit_mode == 5 || edit_mode == 8 {
                    //draw vehicles
                    let color = SDL_MapRGB((*blitdest).format, 0, 0, 128);
                    for vehicle in vehiclelist.iter() {
                        let ix: i16 = ((vehicle.currentTile.x as i32 - draw_offset_col) * TILESIZE + draw_offset_x) as i16;
                        let iy: i16 = ((vehicle.currentTile.y as i32 - draw_offset_row) * TILESIZE + draw_offset_y) as i16;

                        let mut rr = r(ix as i32, iy as i32, 32, 32);
                        SDL_FillRect(blitdest, &mut rr, color);

                        rm.spr_worldvehicle[0].draw_src(ix as i32, iy as i32, &r((vehicle.iDrawDirection as i32) << 5, (vehicle.iDrawSprite as i32) << 5, 32, 32));

                        if edit_mode == 5 {
                            rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(((vehicle.iActionId % 10) as i32) << 5, ((vehicle.iActionId / 10) as i32) << 5, 32, 32));
                        }

                        if edit_mode == 8 {
                            let iBoundary: i16 = vehicle.iBoundary - 1;
                            if iBoundary == -1 {
                                rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(288, 288, 32, 32));
                            } else {
                                rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(((iBoundary % 10) as i32) << 5, ((iBoundary / 10) as i32) << 5, 32, 32));
                            }
                        }
                    }

                    if edit_mode == 5 {
                        if iStageDisplay >= 0 {
                            DisplayStageDetails(false, iStageDisplay, mouse_x as i16, mouse_y as i16);
                        }
                    }
                }

                rm.menu_font_small.draw(0, 0, szEditModes[edit_mode as usize]);

                if fAutoPaint {
                    rm.menu_font_small.draw(0, 16, "Auto Paint");
                }

                rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

                g_musiccategorydisplaytimer -= 1;
                if g_musiccategorydisplaytimer > 0 {
                    spr_dialog.draw_src(224, 176, &r(0, 0, 192, 128));
                    rm.menu_font_small.draw_centered(320, 195, "Music Category");
                    rm.menu_font_large.draw_centered(320, 220, &crate::common::file_list::world_music_category_to_string(g_worldmap.iMusicCategory));

                    rm.menu_font_small.draw_centered(320, 255, "Press 'R' Again");
                    rm.menu_font_small.draw_centered(320, 270, "To Change");
                }

                DrawMessage();
            }

            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn DrawMessage() {
    unsafe {
        if g_messagedisplaytimer > 0 {
            g_messagedisplaytimer -= 1;

            spr_dialog.draw_src(224, 176, &r(0, 0, 192, 128));
            rm.menu_font_large.draw_centered(320, 195, &g_szMessageTitle);
            rm.menu_font_large.draw_centered(320, 220, &g_szMessageLine[0]);
            rm.menu_font_large.draw_centered(320, 240, &g_szMessageLine[1]);
            rm.menu_font_large.draw_centered(320, 260, &g_szMessageLine[2]);
        }
    }
}

pub fn GetForegroundTileValues(iCol: i16, iRow: i16, iOldTiles: &mut [i16; 9]) {
    unsafe {
        let mut iIndex: i16 = 0;
        for iAutoRow in (iRow - 1)..=(iRow + 1) {
            for iAutoCol in (iCol - 1)..=(iCol + 1) {
                if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                    iOldTiles[iIndex as usize] = tile(iAutoCol, iAutoRow).iForegroundSprite;
                    iIndex += 1;
                }
            }
        }

        for i in iIndex..9 {
            iOldTiles[i as usize] = 0;
        }
    }
}

pub fn ForegroundTileValuesChanged(iCol: i16, iRow: i16, iOldTiles: &[i16; 9]) -> bool {
    unsafe {
        let mut iIndex: usize = 0;
        for iAutoRow in (iRow - 1)..=(iRow + 1) {
            for iAutoCol in (iCol - 1)..=(iCol + 1) {
                if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                    let old = iOldTiles[iIndex];
                    iIndex += 1;
                    if tile(iAutoCol, iAutoRow).iForegroundSprite != old {
                        return true;
                    }
                }
            }
        }

        false
    }
}

pub fn ReadVehiclesIntoEditor() {
    unsafe {
        for v in vehiclelist.drain(..) {
            v.delete();
        }

        for vehicle in g_worldmap.vehicles.iter() {
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

            vehiclelist.push(Ptr::new_box(vehiclecopy));
        }
    }
}

pub fn WriteVehiclesIntoWorld() {
    unsafe {
        //Cleanup old vehicles
        g_worldmap.vehicles.clear();

        //Insert new vehicles
        g_worldmap.vehicles.reserve(vehiclelist.len());

        for vehicle in vehiclelist.iter() {
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
            g_worldmap.vehicles.push(vehiclecopy);
        }
    }
}

pub fn AddVehicleToTile(iCol: i16, iRow: i16, iType: i16) {
    unsafe {
        let mut newvehicle: Ptr<WorldVehicle> = Ptr::null();
        for vehicle in vehiclelist.iter() {
            if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                newvehicle = *vehicle;
                break;
            }
        }

        if newvehicle.is_null() {
            newvehicle = Ptr::new_box(WorldVehicle::new());
            newvehicle.currentTile.x = iCol;
            newvehicle.currentTile.y = iRow;
            vehiclelist.push(newvehicle);
        }

        newvehicle.iDrawSprite = g_wvVehicleStamp.iDrawSprite;
        newvehicle.iActionId = g_wvVehicleStamp.iActionId;
        newvehicle.iMinMoves = g_wvVehicleStamp.iMinMoves;
        newvehicle.iMaxMoves = g_wvVehicleStamp.iMaxMoves;
        newvehicle.fSpritePaces = g_wvVehicleStamp.fSpritePaces;
        newvehicle.iDrawDirection = g_wvVehicleStamp.iDrawDirection;
        newvehicle.iBoundary = g_wvVehicleStamp.iBoundary;
    }
}

pub fn RemoveVehicleFromTile(iCol: i16, iRow: i16) {
    unsafe {
        for i in 0..vehiclelist.len() {
            let vehicle = vehiclelist[i];
            if vehicle.currentTile.x == iCol && vehicle.currentTile.y == iRow {
                vehicle.delete();

                vehiclelist.remove(i);

                return;
            }
        }
    }
}

pub fn ReadWarpsIntoEditor() {
    unsafe {
        for w in warplist.drain(..) {
            w.delete();
        }

        for warp in g_worldmap.warps.iter() {
            warplist.push(Ptr::new_box(warp.clone()));
        }
    }
}

pub fn WriteWarpsIntoWorld() {
    unsafe {
        //Cleanup old vehicles
        g_worldmap.warps.clear();

        //Insert new vehicles
        g_worldmap.warps.reserve(warplist.len());

        for warp in warplist.iter() {
            g_worldmap.warps.push((**warp).clone());
        }
    }
}

pub fn AddWarpToTile(iCol: i16, iRow: i16, iType: i16) {
    unsafe {
        let mut newwarp: Ptr<WorldWarp> = Ptr::null();
        for warp in warplist.iter() {
            if warp.id == iType {
                newwarp = *warp;
                break;
            }
        }

        if newwarp.is_null() {
            newwarp = Ptr::new_box(WorldWarp::new(iType, Vec2s::new(iCol, iRow), Vec2s::new(-1, -1)));
            warplist.push(newwarp);
        } else if newwarp.posA.x == -1 {
            newwarp.posA = Vec2s::new(iCol, iRow);
        } else if newwarp.posA.x != iCol || newwarp.posA.y != iRow {
            newwarp.posB = Vec2s::new(iCol, iRow);
        }
    }
}

pub fn RemoveWarpFromTile(iCol: i16, iRow: i16) {
    unsafe {
        let mut i = 0;
        while i < warplist.len() {
            let mut warp = warplist[i];
            if warp.posA.x == iCol && warp.posA.y == iRow {
                if warp.posB.x == -1 && warp.posB.y == -1 {
                    warp.delete();

                    warplist.remove(i);

                    return;
                } else {
                    warp.posA = Vec2s::new(-1, -1);
                }
            } else if warp.posB.x == iCol && warp.posB.y == iRow {
                if warp.posA.x == -1 && warp.posA.y == -1 {
                    warp.delete();

                    warplist.remove(i);

                    return;
                } else {
                    warp.posB = Vec2s::new(-1, -1);
                }
            }

            i += 1;
        }
    }
}

pub fn UpdatePathSprite(iCol: i16, iRow: i16) {
    unsafe {
        for iAutoRow in (iRow - 1)..=(iRow + 1) {
            for iAutoCol in (iCol - 1)..=(iCol + 1) {
                if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                    AutoSetPathSprite(iAutoCol, iAutoRow);
                }
            }
        }
    }
}

pub fn AutoSetPathSprite(iCol: i16, iRow: i16) {
    unsafe {
        let iPathTypes: [i16; 16] = [6, 4, 3, 5, 6, 4, 1, 5, 6, 2, 3, 5, 6, 4, 3, 5];

        let mut iPath: i16 = 0;
        let mut iNeighborIndex: i16 = 0;

        let mut iForegroundSprite: i16 = tile(iCol, iRow).iForegroundSprite;
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
                    if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                        iForegroundSprite = tile(iAutoCol, iAutoRow).iForegroundSprite;

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
        tile(iCol, iRow).iForegroundSprite = AdjustForeground(iPathTypes[iPath as usize] + iForegroundStyle * WORLD_PATH_SPRITE_SET_SIZE, iCol, iRow);
    }
}

//Convert foreground sprite to match the background sprite
pub fn AdjustForeground(mut iSprite: i16, iCol: i16, iRow: i16) -> i16 {
    if iSprite >= WORLD_FOREGROUND_STAGE_OFFSET {
        return iSprite;
    }

    let iBackgroundSprite: i16 = tile(iCol, iRow).iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE;

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

pub fn UpdatePath(iCol: i16, iRow: i16) {
    unsafe {
        for iAutoRow in (iRow - 1)..=(iRow + 1) {
            for iAutoCol in (iCol - 1)..=(iCol + 1) {
                if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                    AutoSetPath(iAutoCol, iAutoRow);
                }
            }
        }
    }
}

pub fn AutoSetPath(iCol: i16, iRow: i16) {
    unsafe {
        let iPathTypes: [i16; 16] = [11, 1, 2, 3, 2, 4, 2, 8, 1, 1, 6, 7, 5, 9, 10, 11];

        let mut iPath: i16 = 0;
        let mut iNeighborIndex: i16 = 0;

        if tile(iCol, iRow).iConnectionType == 0 {
            return;
        }

        for iAutoRow in (iRow - 1)..=(iRow + 1) {
            for iAutoCol in (iCol - 1)..=(iCol + 1) {
                if iAutoCol == iCol && iAutoRow == iRow {
                    continue;
                }

                if (iAutoCol == iCol && iAutoRow != iRow) || (iAutoCol != iCol && iAutoRow == iRow) {
                    if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                        if tile(iAutoCol, iAutoRow).iConnectionType > 0 {
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
        let iForegroundSprite: i16 = tile(iCol, iRow).iForegroundSprite;

        if iPathType == 2 && iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 1 {
            iPathType = iForegroundSprite - WORLD_BRIDGE_SPRITE_OFFSET + 12;
        } else if iPathType == 1 && iForegroundSprite >= WORLD_BRIDGE_SPRITE_OFFSET + 2 && iForegroundSprite <= WORLD_BRIDGE_SPRITE_OFFSET + 3 {
            iPathType = iForegroundSprite - WORLD_BRIDGE_SPRITE_OFFSET + 12;
        }

        tile(iCol, iRow).iConnectionType = iPathType;
    }
}

pub fn UpdateForeground(iCol: i16, iRow: i16) -> bool {
    let iNewForeground: i16 = AdjustForeground(tile(iCol, iRow).iForegroundSprite, iCol, iRow);

    if tile(iCol, iRow).iForegroundSprite != iNewForeground {
        tile(iCol, iRow).iForegroundSprite = iNewForeground;
        return true;
    }

    false
}

pub fn UpdateCoastline(iCol: i16, iRow: i16) -> bool {
    unsafe {
        let iStartCol: i16 = if iCol == 0 { 0 } else { iCol - 1 };
        let iEndCol: i16 = if iCol == iWorldWidth - 1 { iWorldWidth - 1 } else { iCol + 1 };

        let iStartRow: i16 = if iRow == 0 { 0 } else { iRow - 1 };
        let iEndRow: i16 = if iRow == iWorldHeight - 1 { iWorldHeight - 1 } else { iRow + 1 };

        let mut fRet = false;
        for iAutoRow in iStartRow..=iEndRow {
            for iAutoCol in iStartCol..=iEndCol {
                fRet |= AutoSetTile(iAutoCol, iAutoRow);
                fRet |= UpdateForeground(iAutoCol, iAutoRow);
            }
        }

        fRet
    }
}

pub fn AutoSetTile(iCol: i16, iRow: i16) -> bool {
    unsafe {
        //Don't need to do anything if this tile is solid
        if tile(iCol, iRow).iBackgroundSprite % WORLD_BACKGROUND_SPRITE_SET_SIZE == 1 {
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

                if iAutoRow >= 0 && iAutoRow < iWorldHeight && iAutoCol >= 0 && iAutoCol < iWorldWidth {
                    let iBackgroundSprite: i16 = tile(iAutoCol, iAutoRow).iBackgroundSprite;

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

        if tile(iCol, iRow).iBackgroundSprite != iNewTile {
            tile(iCol, iRow).iBackgroundSprite = iNewTile;
            return true;
        }

        false
    }
}

pub fn updateworldsurface() {
    unsafe {
        g_worldmap.draw_map_to_surface(-1, true, sMapSurface, draw_offset_col as i16, draw_offset_row as i16, 0);
    }
}

pub fn drawmap(_fScreenshot: bool, _iBlockSize: i16) {
    unsafe {
        if fNeedBlackBackground {
            SDL_FillRect(screen, null(), 0x0);
        }

        SDL_UpperBlit(sMapSurface, &rectSrcSurface, blitdest, &mut rectDstSurface);
    }
}

pub fn editor_warp() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 6; //change to edit mode using warps
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                        let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                        if event.button.button == BUTTON_LEFT {
                            if iButtonX >= 0 && iButtonX <= 9 && iButtonY == 0 {
                                set_tile = iButtonX as i32;

                                edit_mode = 6; //change to edit mode using warps

                                //The user must release the mouse button before trying to add a tile
                                ignoreclick = true;

                                return EDITOR_EDIT;
                            }
                        }
                    }

                    _ => {}
                }
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            spr_warps[0].draw_src(0, 0, &r(0, 0, 320, 32));

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

fn point_in(x: i16, y: i16, rect: &SDL_Rect) -> bool {
    x as i32 >= rect.x && (x as i32) < rect.w + rect.x && y as i32 >= rect.y && (y as i32) < rect.h + rect.y
}

pub fn editor_start_items() -> i32 {
    unsafe {
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
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iButtonX: i16 = bound_to_window_w(event.button.x) as i16;
                        let iButtonY: i16 = bound_to_window_h(event.button.y) as i16;

                        if event.button.button == BUTTON_LEFT || event.button.button == BUTTON_RIGHT {
                            if g_worldmap.iNumInitialBonuses < 32 {
                                for iItem in 0..(NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize {
                                    if point_in(iButtonX, iButtonY, &rStartItemDst[iItem]) {
                                        let n = g_worldmap.iNumInitialBonuses as usize;
                                        g_worldmap.iInitialBonuses[n] = iItem as i16;
                                        g_worldmap.iNumInitialBonuses += 1;
                                        break;
                                    }
                                }
                            }

                            let mut iRemoveItem: i16 = 0;
                            while iRemoveItem < g_worldmap.iNumInitialBonuses {
                                if point_in(iButtonX, iButtonY, &rPickedItemDst[iRemoveItem as usize]) {
                                    let mut iAdjust = iRemoveItem;
                                    while iAdjust < g_worldmap.iNumInitialBonuses - 1 {
                                        g_worldmap.iInitialBonuses[iAdjust as usize] = g_worldmap.iInitialBonuses[iAdjust as usize + 1];
                                        iAdjust += 1;
                                    }

                                    g_worldmap.iNumInitialBonuses -= 1;

                                    break;
                                }
                                iRemoveItem += 1;
                            }
                        }
                    }

                    _ => {}
                }
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            for iItem in 0..NUM_POWERUPS as usize {
                rm.spr_storedpoweruplarge.draw_src(rStartItemDst[iItem].x, rStartItemDst[iItem].y, &r((iItem as i32) << 5, 0, 32, 32));
            }

            for iWorldItem in 0..NUM_WORLD_POWERUPS as usize {
                rm.spr_worlditems.draw_src(rStartItemDst[iWorldItem + NUM_POWERUPS as usize].x, rStartItemDst[iWorldItem + NUM_POWERUPS as usize].y, &r((iWorldItem as i32) << 5, 0, 32, 32));
            }

            for iPopup in 0..4i32 {
                rm.spr_worlditempopup.draw_src(0, 416 - (iPopup << 6), &r(0, 0, 320, 64));
                rm.spr_worlditempopup.draw_src(320, 416 - (iPopup << 6), &r(192, 0, 320, 64));
            }

            for iPickedItem in 0..g_worldmap.iNumInitialBonuses as usize {
                let iPowerup: i16 = g_worldmap.iInitialBonuses[iPickedItem];
                if iPowerup as i32 >= NUM_POWERUPS {
                    rm.spr_worlditems.draw_src(rPickedItemDst[iPickedItem].x, rPickedItemDst[iPickedItem].y, &r((iPowerup as i32 - NUM_POWERUPS) << 5, 0, 32, 32));
                } else {
                    rm.spr_storedpoweruplarge.draw_src(rPickedItemDst[iPickedItem].x, rPickedItemDst[iPickedItem].y, &r((iPowerup as i32) << 5, 0, 32, 32));
                }
            }

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_boundary() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 8; //change to edit mode using warps
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                        let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                        if event.button.button == BUTTON_LEFT {
                            if iButtonX >= 0 && iButtonX <= 9 && iButtonY >= 0 && iButtonY <= 9 {
                                set_tile = iButtonX as i32 + 10 * iButtonY as i32 + 1;

                                edit_mode = 8; //change to edit mode using warps

                                //The user must release the mouse button before trying to add a tile
                                ignoreclick = true;

                                return EDITOR_EDIT;
                            }
                        }
                    }

                    _ => {}
                }
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            let color = SDL_MapRGB((*blitdest).format, 255, 0, 255);
            let mut rr = r(0, 0, 320, 320);
            SDL_FillRect(blitdest, &mut rr, color);

            rm.spr_worldforegroundspecial[0].draw_src(0, 0, &r(0, 0, 320, 320));

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_type() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 3; //change to edit mode using doors/start
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            //Start and doors
                            if iButtonX >= 0 && iButtonX <= 5 && iButtonY == 0 {
                                set_tile = iButtonX as i32;
                            }

                            edit_mode = 3; //change to edit mode using warps

                            //The user must release the mouse button before trying to add a tile
                            ignoreclick = true;

                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            rm.spr_worldforegroundspecial[0].draw_src(0, 0, &r(320, 128, 64, 32));
            rm.spr_worldforegroundspecial[0].draw_src(64, 0, &r(320, 192, 128, 32));

            rm.spr_worldforegroundspecial[0].draw_src(64, 0, &r(448, 64, 128, 32));

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_water() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 7;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonY == 0 {
                                if iButtonX >= 0 && iButtonX <= 2 {
                                    set_tile = iButtonX as i32 + 4;
                                }
                            }

                            ignoreclick = true;
                            edit_mode = 7;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            for iWater in 0..3i32 {
                rm.spr_worldbackground[0].draw_src(iWater << 5, 0, &r(512 + (iWater << 7), 0, 32, 32));
            }

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_background() -> i32 {
    unsafe {
        let mut done = false;
        let mut iPage: i16 = 0;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        let key = key_sym();
                        if key >= SDLK_1 as i32 && key <= SDLK_2 as i32 {
                            iPage = (key - SDLK_1 as i32) as i16;
                        } else {
                            edit_mode = 0;
                            return EDITOR_EDIT;
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let mut iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            let iTileStyleOffset: i16 = ((iButtonX / 4) + (iPage * 5)) * WORLD_BACKGROUND_SPRITE_SET_SIZE;

                            iButtonX %= 4;

                            if iButtonX == 0 {
                                if iButtonY == 0 {
                                    set_tile = 0;
                                } else if iButtonY >= 1 && iButtonY < 15 {
                                    set_tile = iButtonY as i32 + 1;
                                }
                            } else if iButtonX == 1 {
                                if iButtonY == 0 {
                                    set_tile = 1;
                                } else if iButtonY >= 1 && iButtonY < 15 {
                                    set_tile = iButtonY as i32 + 15;
                                }
                            } else if iButtonX == 2 {
                                if iButtonY >= 0 && iButtonY < 15 {
                                    set_tile = iButtonY as i32 + 30;
                                }
                            } else if iButtonX == 3 {
                                if iButtonY >= 0 && iButtonY < 15 {
                                    set_tile = iButtonY as i32 + 45;
                                }
                            }

                            set_tile += iTileStyleOffset as i32;

                            ignoreclick = true;
                            edit_mode = 0;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            rm.spr_worldbackground[0].draw_src(0, 0, &r(iPage as i32 * 640, 32, 640, 480));

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_stageforeground() -> i32 {
    unsafe {
        let mut done = false;
        let mut iForegroundScreen: i16 = 0;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        let key = key_sym();

                        if key >= SDLK_1 as i32 && key <= SDLK_4 as i32 {
                            iForegroundScreen = (key - SDLK_1 as i32) as i16;
                        } else {
                            edit_mode = 1;
                            return EDITOR_EDIT;
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 10 {
                                if iButtonY >= 0 && iButtonY < 10 {
                                    set_tile = WORLD_FOREGROUND_STAGE_OFFSET as i32 + iButtonY as i32 * 10 + iButtonX as i32 + iForegroundScreen as i32 * 100;

                                    ignoreclick = true;
                                    edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            for iRow in 0..10i32 {
                for iCol in 0..10i32 {
                    rm.spr_worldforegroundspecial[0].draw_src(iCol << 5, iRow << 5, &r(384, (iForegroundScreen as i32) << 5, 32, 32));
                }
            }

            rm.spr_worldforegroundspecial[0].draw_src(0, 0, &r(0, 0, 320, 320));

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_bridges() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 1;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 4 {
                                if iButtonY >= 0 && iButtonY < 1 {
                                    set_tile = WORLD_BRIDGE_SPRITE_OFFSET as i32 + iButtonX as i32;

                                    ignoreclick = true;
                                    edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            rm.spr_worldforegroundspecial[0].draw_src(0, 0, &r(320, 224, 128, 32));

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_structureforeground() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 1;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonY >= 0 && iButtonY < 15 {
                                if iButtonX >= 0 && iButtonX < 12 {
                                    set_tile = WORLD_FOREGROUND_SPRITE_OFFSET as i32 + iButtonX as i32 + iButtonY as i32 * 12;

                                    ignoreclick = true;
                                    edit_mode = 1;
                                    return EDITOR_EDIT;
                                } else if iButtonX >= 12 && iButtonX < 14 {
                                    set_tile = WORLD_FOREGROUND_SPRITE_ANIMATED_OFFSET as i32 + iButtonY as i32 + (iButtonX as i32 - 12) * 15;

                                    ignoreclick = true;
                                    edit_mode = 1;
                                    return EDITOR_EDIT;
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            rm.spr_worldforeground[0].draw_src(0, 0, &r(0, 0, 416, 480));
            rm.spr_worldforeground[0].draw_src(416, 0, &r(512, 0, 32, 480));

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_pathsprite() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 4;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX < 8 {
                                if iButtonY >= 0 && iButtonY < 6 {
                                    set_tile = iButtonY as i32 + 1 + iButtonX as i32 * WORLD_PATH_SPRITE_SET_SIZE as i32;
                                }
                            }

                            ignoreclick = true;
                            edit_mode = 4;
                            return EDITOR_EDIT;
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);

            for iPath in 0..8i32 {
                rm.spr_worldpaths[0].draw_src(iPath << 5, 0, &r((iPath % 4) * 160, (iPath / 4) * 320, 32, 192));
            }

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

//Display stages over vehicles
//allow setting of stages on vehicles

pub fn editor_vehicles() -> i32 {
    unsafe {
        if g_worldmap.iNumStages <= 0 {
            SetDisplayMessage(120, "No Stages", "You need to create", "stages before you", "can create vehicles");
            return EDITOR_EDIT;
        }

        mCurrentMenu = mVehicleMenu.as_ptr();
        mCurrentMenu.reset_menu();

        let done = false;

        miVehicleStageField.clear();

        for iStage in 0..g_worldmap.iNumStages {
            let ts = game_values.tourstops[iStage as usize];
            let szStageName = format!("({}) {}", iStage + 1, ts.szName);

            let item = miVehicleStageField.add(szStageName, iStage);
            item.iconOverride = if ts.iStageType == 1 { 24 } else if ts.iMode >= 1000 { ts.iMode - 975 } else { ts.iMode };
        }

        miVehicleStageField.set_current_value(g_wvVehicleStamp.iActionId);

        while !done {
            let framestart = SDL_GetTicks() as i32;

            game_values.playerInput.clear_pressed_keys(1);

            let mut code: MenuCodeEnum = MENU_CODE_NONE;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        edit_mode = 5;
                        game_values.playerInput.reset_keys();
                        return EDITOR_EDIT;
                    }

                    T_KEYDOWN => {
                        if key_sym() == SDLK_v as i32 {
                            if !mCurrentMenu.is_modifying() {
                                edit_mode = 5;
                                game_values.playerInput.reset_keys();
                                return EDITOR_EDIT;
                            }
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iButtonX: i16 = bound_to_window_w(event.button.x) as i16;
                        let iButtonY: i16 = bound_to_window_h(event.button.y) as i16;

                        if event.button.button == BUTTON_LEFT {
                            code = mCurrentMenu.mouse_click(iButtonX, iButtonY);
                        }
                    }

                    _ => {}
                }

                game_values.playerInput.update(event, 1);
            }

            if MENU_CODE_NONE == code {
                code = mCurrentMenu.send_input(Ptr::from_mut(&mut game_values.playerInput));
            }

            if MENU_CODE_EXIT_APPLICATION == code {
                edit_mode = 5;
                game_values.playerInput.reset_keys();
                return EDITOR_EDIT;
            } else if MENU_CODE_VEHICLE_MIN_MOVES_CHANGED == code {
                let iMaxMoves: i16 = miVehicleMaxMovesField.current_value();
                if miVehicleMinMovesField.current_value() > iMaxMoves {
                    miVehicleMinMovesField.set_current_value(iMaxMoves);
                }
            } else if MENU_CODE_VEHICLE_MAX_MOVES_CHANGED == code {
                let iMinMoves: i16 = miVehicleMinMovesField.current_value();
                if miVehicleMaxMovesField.current_value() < iMinMoves {
                    miVehicleMaxMovesField.set_current_value(iMinMoves);
                }
            } else if MENU_CODE_CREATE_VEHICLE == code {
                edit_mode = 5;
                ignoreclick = true;
                game_values.playerInput.reset_keys();
                return EDITOR_EDIT;
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            mCurrentMenu.update();
            mCurrentMenu.draw();

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn editor_path() -> i32 {
    unsafe {
        let mut done = false;

        while !done {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        edit_mode = 2;
                        return EDITOR_EDIT;
                    }

                    T_MOUSEBUTTONDOWN => {
                        if event.button.button == BUTTON_LEFT {
                            let iButtonX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iButtonY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iButtonX >= 0 && iButtonX <= 15 && iButtonY == 0 {
                                set_tile = iButtonX as i32 + 1;

                                ignoreclick = true;
                                edit_mode = 2;
                                return EDITOR_EDIT;
                            }
                        }
                    }

                    _ => {}
                }
            }

            SDL_FillRect(screen, null(), 0x0);
            spr_path.draw_src(0, 0, &r(0, 0, 480, 32));

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn DisplayStageDetails(fForce: bool, iStageId: i16, mut iMouseX: i16, mut iMouseY: i16) {
    unsafe {
        let ts = game_values.tourstops[iStageId as usize];

        //If we're pointing to a new stage or no stage at all
        if iStageId != iOldStageId || fForce {
            if ts.iStageType == 1 {
                if !sMapThumbnail.is_null() {
                    SDL_FreeSurface(sMapThumbnail);
                    sMapThumbnail = null_mut();
                }
            } else if !ts.pszMapFile.is_empty() {
                if !sMapThumbnail.is_null() {
                    SDL_FreeSurface(sMapThumbnail);
                    sMapThumbnail = null_mut();
                }

                if maplist.findexact(&ts.pszMapFile, false) {
                    let file = maplist.current_filename().to_string();
                    g_map.load_map(&file, read_type_preview);
                    sMapThumbnail = g_map.create_thumbnail_surface(true);
                } else {
                    //otherwise show a unknown map icon
                    let path = CString::new(convert_path("gfx/leveleditor/leveleditor_mapnotfound.png")).unwrap();
                    sMapThumbnail = IMG_Load(path.as_ptr());
                }
            }
        }

        iOldStageId = iStageId;

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
            spr_largedialog.draw_src(x, y, &r(0, 0, 116, 116));
            spr_largedialog.draw_src(x + 116, y, &r(140, 0, 116, 116));
            spr_largedialog.draw_src(x, y + 116, &r(0, 108, 116, 116));
            spr_largedialog.draw_src(x + 116, y + 116, &r(140, 108, 116, 116));
        } else if iMode == 24 {
            //Make sure we're displaying it on the screen
            if iMouseY > 392 {
                iMouseY = 392;
            }

            let (x, y) = (iMouseX as i32, iMouseY as i32);
            spr_largedialog.draw_src(x, y, &r(0, 0, 116, 44));
            spr_largedialog.draw_src(x + 116, y, &r(140, 0, 116, 44));
            spr_largedialog.draw_src(x, y + 44, &r(0, 180, 116, 44));
            spr_largedialog.draw_src(x + 116, y + 44, &r(140, 180, 116, 44));
        }

        let (x, y) = (iMouseX as i32, iMouseY as i32);

        rm.menu_mode_large.draw_src(x + 16, y + 16, &r((iMode as i32) << 5, 0, 32, 32));

        rm.menu_font_small.draw_chop_right(x + 52, y + 16, 164, &ts.szName);

        if iMode != 24 {
            let szPrint = format!("Goal: {}", ts.iGoal);
            rm.menu_font_small.draw_chop_right(x + 52, y + 34, 164, &szPrint);

            let szPrint = format!("Points: {}", ts.iPoints);
            rm.menu_font_small.draw_chop_right(x + 16, y + 176, 100, &szPrint);

            let szPrint = format!("End: {}", if ts.fEndStage { "Yes" } else { "No" });
            rm.menu_font_small.draw_chop_right(x + 126, y + 176, 80, &szPrint);

            for iBonus in 0..ts.iNumBonuses as i32 {
                let wsb = &ts.wsbBonuses[iBonus as usize];
                rm.spr_worlditemsplace.draw_src(x + iBonus * 20 + 16, y + 194, &r(wsb.iWinnerPlace as i32 * 20, 0, 20, 20));

                let iBonusIcon: i32 = wsb.iBonus as i32;
                let spr_icon: &gfxSprite = if iBonusIcon < NUM_POWERUPS { &rm.spr_storedpowerupsmall } else { &rm.spr_worlditemssmall };
                let src = (if iBonusIcon < NUM_POWERUPS { iBonusIcon } else { iBonusIcon - NUM_POWERUPS }) << 4;
                spr_icon.draw_src(x + iBonus * 20 + 18, y + 196, &r(src, 0, 16, 16));
            }

            if !sMapThumbnail.is_null() {
                let rSrc = r(0, 0, 160, 120);
                let mut rDst = r(x + 16, y + 52, 160, 120);

                SDL_UpperBlit(sMapThumbnail, &rSrc, blitdest, &mut rDst);
            }
        } else {
            let szPrint = format!("Sort: {}", if ts.iBonusType == 0 { "Fixed" } else { "Random" });
            rm.menu_font_small.draw_chop_right(x + 52, y + 34, 164, &szPrint);

            for iBonus in 0..ts.iNumBonuses as i32 {
                let iBonusIcon: i32 = ts.wsbBonuses[iBonus as usize].iBonus as i32;
                let spr_icon: &gfxSprite;
                let iSrcX: i32;
                let iSrcY: i32;

                if iBonusIcon < NUM_POWERUPS {
                    spr_icon = &rm.spr_storedpowerupsmall;
                    iSrcX = iBonusIcon << 4;
                    iSrcY = 0;
                } else if iBonusIcon < NUM_POWERUPS + NUM_WORLD_POWERUPS {
                    spr_icon = &rm.spr_worlditemssmall;
                    iSrcX = (iBonusIcon - NUM_POWERUPS) << 4;
                    iSrcY = 0;
                } else if iBonusIcon < NUM_POWERUPS + NUM_WORLD_POWERUPS + 10 {
                    spr_icon = &rm.spr_worlditemssmall;
                    iSrcX = (iBonusIcon - NUM_POWERUPS - NUM_WORLD_POWERUPS) << 4;
                    iSrcY = 16;
                } else {
                    spr_icon = &rm.spr_worlditemssmall;
                    iSrcX = (iBonusIcon - NUM_POWERUPS - NUM_WORLD_POWERUPS - 10) << 4;
                    iSrcY = 32;
                }

                spr_icon.draw_src(x + iBonus * 20 + 18, y + 52, &r(iSrcX, iSrcY, 16, 16));
            }
        }
    }
}

pub static g_iNumGameModeSettings: [i16; GAMEMODE_LAST as usize] = [2, 2, 3, 4, 3, 10, 9, 6, 2, 1, 3, 5, 3, 3, 0, 22, 6, 4, 3, 4, 4, 3];

pub static mut rItemDst: [SDL_Rect; NUM_WORLD_ITEMS as usize] = [SDL_Rect { x: 0, y: 0, w: 0, h: 0 }; NUM_WORLD_ITEMS as usize];

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

pub fn TestAndSetBonusItem(mut ts: Ptr<TourStop>, iPlace: i16, iButtonX: i16, iButtonY: i16) {
    unsafe {
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
                if point_in(iButtonX, iButtonY, &rItemDst[iItem as usize]) {
                    //If this is a normal stage, then alert the player that they need to select the place for this item
                    if ts.iStageType == 0 && iPlace == 0 {
                        SetDisplayMessage(120, "Use Number Keys", "Hover over item", "use keys 1 to 4", "to select bonus");
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
}

static mut iLastStageType: i16 = 0;

pub fn AdjustBonuses(mut ts: Ptr<TourStop>) {
    unsafe {
        //No need to do anything if we were a stage and we're still a stage
        //(or a house and still a house)
        if ts.iStageType == iLastStageType {
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

        iLastStageType = ts.iStageType;
    }
}

pub fn SaveStage(iEditStage: i16) {
    unsafe {
        let mut ts = game_values.tourstops[iEditStage as usize];

        //Set the number of game mode settings to the maximum so we write them all out
        ts.fUseSettings = true;
        // Bonus house and minigame modes (24-27) read past the C++ array into unrelated globals.
        ts.iNumUsedSettings = g_iNumGameModeSettings.get(ts.iMode as usize).copied().unwrap_or(0);

        //Copy the working values back into the structure that will be saved
        ts.gmsSettings = game_values.gamemodemenusettings.clone();

        if ts.iMode >= 25 && ts.iMode <= 27 {
            ts.iMode += 975;
        }
    }
}

pub fn EditStage(iEditStage: i16) {
    unsafe {
        let mut ts = game_values.tourstops[iEditStage as usize];

        game_values.gamemodemenusettings = ts.gmsSettings.clone();

        //Set fields to write data to the selected stage
        miModeField.set_output_ptr(&mut ts.iMode as *mut i16);
        miNameField.set_data(&mut ts.szName, 128);

        miPointsField.set_output_ptr(&mut ts.iPoints as *mut i16);
        miFinalStageField.set_output_ptr(&mut ts.fEndStage as *mut bool);

        if ts.pszMapFile.is_empty() {
            ts.pszMapFile = maplist.current_shortmapname().to_string();
        }

        let mapfile = ts.pszMapFile.clone();
        miMapField.set_map(&mapfile, false);

        miBonusType.set_output_ptr(&mut ts.iBonusType as *mut i16);
        miBonusTextField[0].set_data(&mut ts.szBonusText[0], 128);
        miBonusTextField[1].set_data(&mut ts.szBonusText[1], 128);
        miBonusTextField[2].set_data(&mut ts.szBonusText[2], 128);
        miBonusTextField[3].set_data(&mut ts.szBonusText[3], 128);
        miBonusTextField[4].set_data(&mut ts.szBonusText[4], 128);

        ts.iBonusTextLines = 5;

        let mut iMode: i16 = ts.iMode;
        let iStageType: i16 = ts.iStageType;

        if iMode >= 25 && iMode <= 27 {
            iMode += 975;
        }

        //Show fields applicable for this mode
        miPointsField.set_visible(iStageType == 0);
        miFinalStageField.set_visible(iStageType == 0);
        miMapField.set_visible(iStageType == 0);

        miBonusType.set_visible(iStageType == 1);
        miBonusTextField[0].set_visible(iStageType == 1);
        miBonusTextField[1].set_visible(iStageType == 1);
        miBonusTextField[2].set_visible(iStageType == 1);
        miBonusTextField[3].set_visible(iStageType == 1);
        miBonusTextField[4].set_visible(iStageType == 1);

        miSpecialGoalField[0].set_visible(iMode == 1000);
        miSpecialGoalField[1].set_visible(iMode == 1001);
        miSpecialGoalField[2].set_visible(iMode == 1002);

        miBonusItemsButton.set_position(430, if iStageType == 0 { 220 } else { 340 });

        if iStageType == 0 && iMode >= 0 && (iMode as i32) < GAMEMODE_LAST {
            miModeField.set_current_value(iMode);

            miModeSettingsButton.set_visible(iMode as i32 != game_mode_owned);

            for iGameMode in 0..GAMEMODE_LAST as usize {
                miGoalField[iGameMode].set_visible(iMode as usize == iGameMode);
            }

            miGoalField[iMode as usize].set_output_ptr(&mut ts.iGoal as *mut i16);
            miGoalField[iMode as usize].set_current_value(ts.iGoal);
            miPointsField.set_current_value(ts.iPoints);
            miFinalStageField.set_current_value((if ts.fEndStage { 1 } else { 0 }) != 0);

            ts.fUseSettings = true;
            ts.iNumUsedSettings = g_iNumGameModeSettings[iMode as usize];
        } else {
            //Show the settings button for boss mode
            miModeSettingsButton.set_visible(iMode == 1001);

            for iGameMode in 0..GAMEMODE_LAST as usize {
                miGoalField[iGameMode].set_visible(false);
            }

            if iStageType == 1 {
                //Bonus House
                miModeField.set_current_value(24);
            } else if iMode >= 1000 && iMode <= 1002 {
                //Pipe, Boss and Boxes Game
                miModeField.set_current_value(iMode - 975);
                miSpecialGoalField[(iMode - 1000) as usize].set_output_ptr(&mut ts.iGoal as *mut i16);
                miSpecialGoalField[(iMode - 1000) as usize].set_current_value(ts.iGoal);
                miPointsField.set_current_value(ts.iPoints);
                miFinalStageField.set_current_value((if ts.fEndStage { 1 } else { 0 }) != 0);

                ts.iMode = iMode;
            }
        }
    }
}

pub fn EnableStageMenu(fEnable: bool) {
    unsafe {
        miNameField.disable(!fEnable);
        miModeField.disable(!fEnable);

        for iGameMode in 0..GAMEMODE_LAST as usize {
            miGoalField[iGameMode].disable(!fEnable);
        }

        miSpecialGoalField[0].disable(!fEnable);
        miSpecialGoalField[1].disable(!fEnable);
        miSpecialGoalField[2].disable(!fEnable);

        miModeSettingsButton.disable(!fEnable);

        miPointsField.disable(!fEnable);
        miFinalStageField.disable(!fEnable);
        miMapField.disable(!fEnable);
        miBonusItemsButton.disable(!fEnable);
        miBonusType.disable(!fEnable);
        miBonusTextField[0].disable(!fEnable);
        miBonusTextField[1].disable(!fEnable);
        miBonusTextField[2].disable(!fEnable);
        miBonusTextField[3].disable(!fEnable);
        miBonusTextField[4].disable(!fEnable);
        miDeleteStageButton.disable(!fEnable);
    }
}

pub fn NewStage(iEditStage: &mut i16) {
    unsafe {
        let mut ts = Ptr::new_box(TourStop::default());

        ts.iStageType = 0;

        ts.szBonusText[0].clear();
        ts.szBonusText[1].clear();
        ts.szBonusText[2].clear();
        ts.szBonusText[3].clear();
        ts.szBonusText[4].clear();

        ts.pszMapFile = maplist.current_shortmapname().to_string();
        ts.iMode = 0;

        ts.fUseSettings = true;
        ts.iNumUsedSettings = g_iNumGameModeSettings[0];

        ts.iGoal = 10;
        ts.iPoints = 1;

        ts.iBonusType = 0;
        ts.iNumBonuses = 0;

        ts.szName = format!("Tour Stop {}", game_values.tourstops.len() + 1);

        ts.fEndStage = false;

        //Copy in default values first
        ts.gmsSettings = game_values.gamemodemenusettings.clone();

        game_values.tourstops.push(ts);
        g_worldmap.iNumStages += 1;

        *iEditStage = (game_values.tourstops.len() - 1) as i16;
        EditStage(*iEditStage);
    }
}

/// Reproduces the C++ loop, which keeps iterating `vehiclelist` with iterators captured before
/// `RemoveVehicleFromTile` erases from it: after an erase the next element is skipped and the
/// stale slots past the new end (which still hold the old pointers) are visited.
fn delete_stage_from_vehicles(iEditStage: i16) {
    unsafe {
        let mut mem: Vec<Ptr<WorldVehicle>> = vehiclelist.clone();
        let lim = mem.len();
        let mut freed: Vec<Ptr<WorldVehicle>> = Vec::new();

        for i in 0..lim {
            let mut vehicle = mem[i];
            if freed.contains(&vehicle) {
                continue;
            }

            if vehicle.iActionId == iEditStage {
                let (x, y) = (vehicle.currentTile.x, vehicle.currentTile.y);
                let n = vehiclelist.len();
                if let Some(k) = vehiclelist.iter().position(|v| v.currentTile.x == x && v.currentTile.y == y) {
                    freed.push(vehiclelist[k]);
                    for j in k..n - 1 {
                        mem[j] = mem[j + 1];
                    }
                }
                RemoveVehicleFromTile(x, y);
            } else if vehicle.iActionId > iEditStage {
                vehicle.iActionId -= 1;
            }
        }
    }
}

pub fn editor_stage() -> i32 {
    unsafe {
        mCurrentMenu = mStageSettingsMenu.as_ptr();
        mCurrentMenu.reset_menu();

        let mut done = false;
        let mut iStageDisplay: i16 = -1;
        let mut fForceStageDisplay = false;
        let mut iEditStage: i16 = -1;

        let mut rStageBonusDst = [r(0, 0, 0, 0); 10];
        let mut rHouseBonusDst = [r(0, 0, 0, 0); MAX_BONUS_CHESTS as usize];

        let mut iColCount: i16 = 0;
        let mut iRowCount: i16 = 0;
        for iItem in 0..NUM_WORLD_ITEMS as usize {
            rItemDst[iItem].x = 16 + iColCount as i32 * 48;
            rItemDst[iItem].y = 16 + iRowCount as i32 * 48;
            rItemDst[iItem].w = 32;
            rItemDst[iItem].h = 32;

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
            let framestart = SDL_GetTicks() as i32;

            //Reset the keys that were down the last frame
            game_values.playerInput.clear_pressed_keys(1);

            let mut code: MenuCodeEnum = MENU_CODE_NONE;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => {
                        done = true;
                    }

                    T_KEYDOWN => {
                        if iEditStage == -1 && key_sym() == SDLK_n as i32 {
                            NewStage(&mut iEditStage);
                        } else if (key_sym() == SDLK_ESCAPE as i32 || key_sym() == SDLK_e as i32) && iEditStage == -1 {
                            if g_worldmap.iNumStages == 0 {
                                edit_mode = 1;
                            } else if set_tile < 6 || set_tile >= g_worldmap.iNumStages as i32 + 6 {
                                set_tile = 6;
                                edit_mode = 9;
                            } else {
                                edit_mode = 9; //change to edit mode using stages
                            }

                            return EDITOR_EDIT;
                        } else if mCurrentMenu == mBonusItemPicker.as_ptr() && key_sym() >= SDLK_1 as i32 && key_sym() <= SDLK_4 as i32 {
                            let ts = game_values.tourstops[iEditStage as usize];
                            if ts.iStageType == 0 {
                                let iPlace: i16 = (key_sym() - SDLK_1 as i32 + 1) as i16;

                                TestAndSetBonusItem(ts, iPlace, mouse_x as i16, mouse_y as i16);
                            }
                        } else if (key_sym() == SDLK_PAGEUP as i32 && iEditStage > 0) || (key_sym() == SDLK_PAGEDOWN as i32 && iEditStage < g_worldmap.iNumStages - 1) {
                            if iEditStage != -1 && mCurrentMenu == mStageSettingsMenu.as_ptr() {
                                SaveStage(iEditStage);

                                if key_sym() == SDLK_PAGEUP as i32 {
                                    iEditStage -= 1;
                                } else if key_sym() == SDLK_PAGEDOWN as i32 {
                                    iEditStage += 1;
                                }

                                EditStage(iEditStage);
                                mCurrentMenu.reset_menu();

                                code = MENU_CODE_MODE_CHANGED;

                                mModeOptionsMenu.refresh();
                            }
                        }
                    }

                    T_MOUSEBUTTONDOWN => {
                        let iTileX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                        let iTileY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;
                        let iButtonX: i16 = bound_to_window_w(event.button.x) as i16;
                        let iButtonY: i16 = bound_to_window_h(event.button.y) as i16;

                        if event.button.button == BUTTON_LEFT {
                            //Stages
                            if iEditStage == -1 {
                                if iTileX >= 0 && (iTileX as i32) < g_worldmap.iNumStages as i32 - (iTileY as i32 * 20) && iTileY >= 0 && iTileY <= (g_worldmap.iNumStages - 1) / 20 {
                                    set_tile = iTileX as i32 + (iTileY as i32 * 20) + 6;

                                    edit_mode = 9; //change to edit mode using warps

                                    //The user must release the mouse button before trying to add a tile
                                    ignoreclick = true;

                                    return EDITOR_EDIT;
                                }
                                //New stage button
                                else if iButtonX >= 256 && iButtonX < 384 && iButtonY >= 420 && iButtonY < 452 {
                                    NewStage(&mut iEditStage);
                                }
                            } else if mCurrentMenu == mBonusItemPicker.as_ptr() {
                                let mut ts = game_values.tourstops[iEditStage as usize];

                                //See if we clicked an item and add it if we did
                                TestAndSetBonusItem(ts, 0, iButtonX, iButtonY);

                                //See if we clicked an already added item and remove it
                                let mut iRemoveItem: i16 = 0;
                                while iRemoveItem < ts.iNumBonuses {
                                    let rects: &[SDL_Rect] = if ts.iStageType == 1 { &rHouseBonusDst } else { &rStageBonusDst };

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
                                code = mCurrentMenu.mouse_click(iButtonX, iButtonY);
                            }
                        } else if event.button.button == BUTTON_RIGHT {
                            if iEditStage == -1 {
                                if iTileX >= 0 && (iTileX as i32) < g_worldmap.iNumStages as i32 - (iTileY as i32 * 20) && iTileY >= 0 && iTileY <= (g_worldmap.iNumStages - 1) / 20 {
                                    iEditStage = iTileX + (iTileY * 20);
                                    EditStage(iEditStage);

                                    code = MENU_CODE_MODE_CHANGED;

                                    mModeOptionsMenu.refresh();
                                }
                            }
                        }
                    }

                    T_MOUSEMOTION => {
                        bound_mouse_motion_coords();
                        iStageDisplay = -1;

                        if iEditStage == -1 {
                            let iMouseX: i16 = (bound_to_window_w(event.button.x) / TILESIZE) as i16;
                            let iMouseY: i16 = (bound_to_window_h(event.button.y) / TILESIZE) as i16;

                            if iMouseX >= 0 && (iMouseX as i32) < g_worldmap.iNumStages as i32 - (iMouseY as i32 * 20) && iMouseY >= 0 && iMouseY <= (g_worldmap.iNumStages - 1) / 20 {
                                iStageDisplay = iMouseX + (iMouseY * 20);
                            }
                        }
                    }

                    _ => {}
                }

                game_values.playerInput.update(event, 1);
            }

            if iEditStage >= 0 {
                if MENU_CODE_NONE == code {
                    code = mCurrentMenu.send_input(Ptr::from_mut(&mut game_values.playerInput));
                }

                if MENU_CODE_EXIT_APPLICATION == code {
                    //Save the current stage
                    SaveStage(iEditStage);

                    iEditStage = -1;
                } else if MENU_CODE_MODE_CHANGED == code {
                    let iMode: i16 = miModeField.current_value();

                    miPointsField.set_visible(iMode != 24);
                    miFinalStageField.set_visible(iMode != 24);

                    miMapField.set_visible(iMode != 24);

                    miBonusType.set_visible(iMode == 24);
                    miBonusTextField[0].set_visible(iMode == 24);
                    miBonusTextField[1].set_visible(iMode == 24);
                    miBonusTextField[2].set_visible(iMode == 24);
                    miBonusTextField[3].set_visible(iMode == 24);
                    miBonusTextField[4].set_visible(iMode == 24);

                    miSpecialGoalField[0].set_visible(iMode == 25);
                    miSpecialGoalField[1].set_visible(iMode == 26);
                    miSpecialGoalField[2].set_visible(iMode == 27);

                    miBonusItemsButton.set_position(430, if iMode != 24 { 220 } else { 340 });

                    let mut ts = game_values.tourstops[iEditStage as usize];

                    if iMode >= 0 && (iMode as i32) < GAMEMODE_LAST {
                        miModeSettingsButton.set_visible(iMode as i32 != game_mode_owned);

                        for iGameMode in 0..GAMEMODE_LAST as usize {
                            miGoalField[iGameMode].set_visible(iMode as usize == iGameMode);
                        }

                        miGoalField[iMode as usize].set_output_ptr(&mut ts.iGoal as *mut i16);
                        miGoalField[iMode as usize].update_output();

                        ts.iStageType = 0;

                        ts.fUseSettings = true;
                        ts.iNumUsedSettings = g_iNumGameModeSettings[iMode as usize];
                    } else {
                        //Show the settings button for boss mode
                        miModeSettingsButton.set_visible(iMode == 26);

                        for iGameMode in 0..GAMEMODE_LAST as usize {
                            miGoalField[iGameMode].set_visible(false);
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
                    AdjustBonuses(ts);
                } else if MENU_CODE_TO_MODE_SETTINGS_MENU == code {
                    let mut fModeFound = false;
                    for iGameMode in 0..GAMEMODE_LAST as usize {
                        if miGoalField[iGameMode].is_visible() {
                            mCurrentMenu = mModeOptionsMenu.get_options_menu(iGameMode as i16);
                            mCurrentMenu.reset_menu();
                            fModeFound = true;
                            break;
                        }
                    }

                    //Look to see if this is the boss mode and go to boss settings
                    if !fModeFound {
                        if miSpecialGoalField[1].is_visible() {
                            mCurrentMenu = mModeOptionsMenu.get_boss_options_menu();
                            mCurrentMenu.reset_menu();
                        }
                    }
                } else if MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED == code {
                    mModeOptionsMenu.health_mode_start_life_changed();
                } else if MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED == code {
                    mModeOptionsMenu.health_mode_max_life_changed();
                } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS == code {
                    mCurrentMenu = mStageSettingsMenu.as_ptr();
                    mCurrentMenu.reset_menu();
                } else if MENU_CODE_MAP_CHANGED == code {
                    let mut ts = game_values.tourstops[iEditStage as usize];
                    ts.pszMapFile = maplist.current_shortmapname().to_string();
                    fForceStageDisplay = true;
                } else if MENU_CODE_TO_BONUS_PICKER_MENU == code {
                    mCurrentMenu = mBonusItemPicker.as_ptr();
                    mCurrentMenu.reset_menu();
                } else if MENU_CODE_DELETE_STAGE_BUTTON == code {
                    miDeleteStageDialogImage.set_visible(true);
                    miDeleteStageDialogAreYouText.set_visible(true);
                    miDeleteStageDialogSureText.set_visible(true);
                    miDeleteStageDialogYesButton.set_visible(true);
                    miDeleteStageDialogNoButton.set_visible(true);

                    EnableStageMenu(false);

                    mStageSettingsMenu.remember_current();

                    mStageSettingsMenu.set_initial_focus(ctl_ptr(miDeleteStageDialogNoButton));
                    mStageSettingsMenu.set_cancel_code(MENU_CODE_DELETE_STAGE_NO);
                    mStageSettingsMenu.reset_menu();
                } else if MENU_CODE_DELETE_STAGE_YES == code || MENU_CODE_DELETE_STAGE_NO == code {
                    miDeleteStageDialogImage.set_visible(false);
                    miDeleteStageDialogAreYouText.set_visible(false);
                    miDeleteStageDialogSureText.set_visible(false);
                    miDeleteStageDialogYesButton.set_visible(false);
                    miDeleteStageDialogNoButton.set_visible(false);

                    mStageSettingsMenu.set_initial_focus(ctl_ptr(miNameField));
                    mStageSettingsMenu.set_cancel_code(MENU_CODE_EXIT_APPLICATION);
                    mStageSettingsMenu.restore_current();

                    //Yes was selected to delete this stage
                    if MENU_CODE_DELETE_STAGE_YES == code {
                        //Scan the grid of stages and remove any references to this stage
                        //and decrement stage numbers greater than this stage
                        for iRow in 0..iWorldHeight {
                            for iCol in 0..iWorldWidth {
                                if tile(iCol, iRow).iType == iEditStage + 6 {
                                    tile(iCol, iRow).iType = 0;
                                } else if tile(iCol, iRow).iType > iEditStage + 6 {
                                    tile(iCol, iRow).iType -= 1;
                                }
                            }
                        }

                        //Scan vehicles and remove references to deleted stage
                        delete_stage_from_vehicles(iEditStage);

                        //Remove stage from tourstops vector
                        let mut iIndex: i16 = 0;
                        while (iIndex as usize) < game_values.tourstops.len() {
                            if iIndex == iEditStage {
                                game_values.tourstops[iIndex as usize].delete();

                                game_values.tourstops.erase(iIndex as usize);
                                g_worldmap.iNumStages -= 1;

                                break;
                            }

                            iIndex += 1;
                        }

                        iEditStage = -1;
                        mCurrentMenu = mStageSettingsMenu.as_ptr();
                        mCurrentMenu.reset_menu();
                    }

                    EnableStageMenu(true);
                }
            }

            drawmap(false, TILESIZE as i16);
            menu_shade.draw(0, 0);

            let color = SDL_MapRGB((*blitdest).format, 0, 0, 255);

            if iEditStage == -1 {
                for iStage in 0..g_worldmap.iNumStages as i32 {
                    let ix: i16 = ((iStage % 20) << 5) as i16;
                    let iy: i16 = ((iStage / 20) << 5) as i16;

                    let mut rr = r(ix as i32, iy as i32, 32, 32);
                    SDL_FillRect(blitdest, &mut rr, color);

                    rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r((iStage % 10) << 5, (iStage / 10) << 5, 32, 32));
                }

                if iStageDisplay >= 0 {
                    DisplayStageDetails(fForceStageDisplay, iStageDisplay, mouse_x as i16, mouse_y as i16);
                    fForceStageDisplay = false;
                }

                //Display New button
                rm.spr_selectfield.draw_src(256, 420, &r(0, 0, 64, 32));
                rm.spr_selectfield.draw_src(320, 420, &r(448, 0, 64, 32));

                rm.menu_font_large.draw_centered(320, 425, "New Stage");

                rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[LMB] Select Stage, [RMB] Edit Stage, [n] New Stage");
            } else {
                mCurrentMenu.update();
                mCurrentMenu.draw();

                if mCurrentMenu != mBonusItemPicker.as_ptr() {
                    let ix: i16 = 20;
                    let iy: i16 = 20;

                    let mut rr = r(ix as i32, iy as i32, 32, 32);
                    SDL_FillRect(blitdest, &mut rr, color);

                    rm.spr_worldforegroundspecial[0].draw_src(ix as i32, iy as i32, &r(((iEditStage % 10) as i32) << 5, ((iEditStage / 10) as i32) << 5, 32, 32));
                }
            }

            if mCurrentMenu == mBonusItemPicker.as_ptr() {
                let ts = game_values.tourstops[iEditStage as usize];

                //Game powerups
                for iItem in 0..NUM_POWERUPS as usize {
                    rm.spr_storedpoweruplarge.draw_src(rItemDst[iItem].x, rItemDst[iItem].y, &r((iItem as i32) << 5, 0, 32, 32));
                }

                //World Powerups
                for iWorldItem in 0..NUM_WORLD_POWERUPS as usize {
                    let d = rItemDst[iWorldItem + NUM_POWERUPS as usize];
                    rm.spr_worlditems.draw_src(d.x, d.y, &r((iWorldItem as i32) << 5, 0, 32, 32));
                }

                //Score Bonuses
                if ts.iStageType == 1 {
                    for iScoreBonus in 0..NUM_WORLD_SCORE_BONUSES {
                        let d = rItemDst[(iScoreBonus + NUM_POWERUPS + NUM_WORLD_POWERUPS) as usize];
                        rm.spr_worlditems.draw_src(
                            d.x,
                            d.y,
                            &r(if iScoreBonus < 10 { iScoreBonus << 5 } else { (iScoreBonus - 10) << 5 }, if iScoreBonus < 10 { 32 } else { 64 }, 32, 32),
                        );
                    }
                }

                //Draw background container
                rm.spr_worlditempopup.draw_src(0, 344, &r(0, 0, 320, 64));
                rm.spr_worlditempopup.draw_src(320, 344, &r(192, 0, 320, 64));

                let rects: &[SDL_Rect] = if ts.iStageType == 1 { &rHouseBonusDst } else { &rStageBonusDst };

                for iPickedItem in 0..ts.iNumBonuses as usize {
                    let iBonus: i32 = ts.wsbBonuses[iPickedItem].iBonus as i32;
                    let iPlace: i32 = ts.wsbBonuses[iPickedItem].iWinnerPlace as i32;

                    //Draw place behind bonus
                    if ts.iStageType == 0 {
                        rm.spr_worlditempopup.draw_src(rects[iPickedItem].x - 8, rects[iPickedItem].y - 8, &r(iPlace * 48, 256, 48, 48));
                    }

                    if iBonus >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                        let iBonusIndex: i32 = iBonus - NUM_POWERUPS - NUM_WORLD_POWERUPS;
                        rm.spr_worlditems.draw_src(
                            rects[iPickedItem].x,
                            rects[iPickedItem].y,
                            &r(if iBonusIndex < 10 { iBonusIndex << 5 } else { (iBonusIndex - 10) << 5 }, if iBonusIndex < 10 { 32 } else { 64 }, 32, 32),
                        );
                    } else if iBonus >= NUM_POWERUPS {
                        rm.spr_worlditems.draw_src(rects[iPickedItem].x, rects[iPickedItem].y, &r((iBonus - NUM_POWERUPS) << 5, 0, 32, 32));
                    } else {
                        rm.spr_storedpoweruplarge.draw_src(rects[iPickedItem].x, rects[iPickedItem].y, &r(iBonus << 5, 0, 32, 32));
                    }
                }

                if ts.iStageType == 0 {
                    rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[1-4] Select Items, [LMB] Remove Items");
                } else {
                    rm.menu_font_small.draw(0, 480 - rm.menu_font_small.get_height(), "[LMB] Select Items, [LMB] Remove Items");
                }
            }

            rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());

            DrawMessage();
            gfx_flipscreen();

            frame_wait(framestart);
        }

        EDITOR_QUIT
    }
}

pub fn display_help() -> i32 {
    unsafe {
        drawmap(false, TILESIZE as i16);
        menu_shade.draw(0, 0);
        rm.menu_font_large.draw_centered(320, 15, "Help");

        let mut offsety: i32 = 55;
        let mut offsetx: i32 = 30;
        let lh = rm.menu_font_small.get_height();
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
        rm.menu_font_small.draw(offsetx, offsety, "Modes:");
        offsety += lh + 2;
        for (i, line) in lines_left.iter().take(14).enumerate() {
            rm.menu_font_small.draw(offsetx, offsety, line);
            offsety += lh + if i == 13 { 20 } else { 2 };
        }

        rm.menu_font_small.draw(offsetx, offsety, "File:");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[n] - New World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[s] - Save World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[shift] + [s] - Save As");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[f] - Find World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[shift] + [f] - New Search");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[pageup] - Go To Previous World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[pagedown] - Go To Next World");

        offsetx = 300;
        offsety = 55;

        rm.menu_font_small.draw(offsetx, offsety, "Place Tiles:");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[Left Mouse Button] - Place Item");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[Right Mouse Button] - Remove Item");
        offsety += lh + 20;

        rm.menu_font_small.draw(offsetx, offsety, "Miscellaneous:");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[r] - Change Music Category");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[Arrow Keys] - Navigate World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[a] - Automatic Path/Land");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[k] - Resize World");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[ctrl] + [delete] - Clear All");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[insert] - Screenshot");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[alt] + [enter] - Full Screen/Window");
        offsety += lh + 2;
        rm.menu_font_small.draw(offsetx, offsety, "[space] - Toggle Stage Previews");

        gfx_flipscreen();

        loop {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => return 0,
                    T_KEYDOWN => return 0,
                    _ => {}
                }
            }

            frame_wait(framestart);
        }
    }
}

pub fn save_as() -> i32 {
    unsafe {
        let mut fileName = String::new();
        let mut mapLocation = String::from("worlds/");

        if dialog("Save As", "Enter name:", &mut fileName, 64) {
            fileName.push_str(".txt");
            mapLocation.push_str(&fileName);
            worldlist.add(mapLocation.into());
            worldlist.find(&fileName);
            game_values.worldindex = worldlist.current_index() as i16;
            savecurrentworld();
            loadcurrentworld();
        }

        0
    }
}

fn draw_dialog(title: &str, instructions: &str, input: Option<&str>) {
    unsafe {
        drawmap(false, TILESIZE as i16);
        menu_shade.draw(0, 0);
        spr_dialog.draw_src(224, 176, &r(0, 0, 192, 128));
        rm.menu_font_large.draw_centered(320, 200, title);
        rm.menu_font_small.draw(240, 235, instructions);
        if let Some(input) = input {
            rm.menu_font_small.draw(240, 255, input);
        }
        rm.menu_font_small.draw_right_justified(640, 0, &worldlist.current_path().to_string_lossy());
        gfx_flipscreen();
    }
}

/// `char* input` of capacity `inputsize`: the caller's buffer keeps whatever was typed.
pub fn dialog(title: &str, instructions: &str, input: &mut String, inputsize: i32) -> bool {
    unsafe {
        let mut currentChar: u32 = 0;

        draw_dialog(title, instructions, None);

        loop {
            let framestart = SDL_GetTicks() as i32;

            //handle messages
            while SDL_PollEvent(&mut event) != 0 {
                match event_type() {
                    T_QUIT => return false,

                    T_KEYDOWN => {
                        let sym = key_sym();
                        if sym == SDLK_KP_ENTER as i32 || sym == SDLK_RETURN as i32 {
                            return true;
                        } else if sym == SDLK_ESCAPE as i32 {
                            return false;
                        } else if sym == SDLK_BACKSPACE as i32 {
                            if currentChar > 0 {
                                input.truncate(currentChar as usize - 1);

                                draw_dialog(title, instructions, Some(input));

                                currentChar -= 1;
                            }
                        } else {
                            /* I realize the if statement below is long and can be substituted with
                            the function isalnum(event.key.keysym.sym) but I did it this way because
                            isalnum acts funny (ie wrong) when the number pad is pressed. */
                            let isdigit = (b'0' as i32..=b'9' as i32).contains(&sym);
                            if (isdigit || sym == 45 || sym == 32 || sym == 61 || (sym >= 95 && sym <= 122)) && currentChar < (inputsize as u32).wrapping_sub(1) {
                                //insert character into fileName and onScreenText and increment current char
                                let mut key: u8 = sym as u8;

                                let keystate = SDL_GetKeyboardState(null_mut());
                                if check_key(keystate, SDLK_LSHIFT as i32) || check_key(keystate, SDLK_RSHIFT as i32) {
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

                                draw_dialog(title, instructions, Some(input));
                            }
                        }
                    }

                    _ => {}
                }
            }

            frame_wait(framestart);
        }
    }
}

pub fn find() -> i32 {
    unsafe {
        let mut fileName = String::new();
        //char mapLocation[FILEBUFSIZE] = "maps/";

        if dialog("Find Map", "Enter name:", &mut fileName, 64) {
            findstring = fileName.clone();

            let fs = findstring.clone();
            if worldlist.find(&fs) {
                game_values.worldindex = worldlist.current_index() as i16;
                loadcurrentworld();
            }
        }

        0
    }
}

pub fn clear_world() -> i32 {
    unsafe {
        g_worldmap.clear();
        updateworldsurface();

        println!("World Cleared");
        0
    }
}

pub fn loadcurrentworld() {
    unsafe {
        game_values.worldindex = worldlist.current_index() as i16;
        let path = worldlist.at(game_values.worldindex as usize).to_string_lossy().into_owned();
        *g_worldmap = WorldMap::new_path(&path, TILESIZE as i16);
        ReadVehiclesIntoEditor();
        ReadWarpsIntoEditor();

        draw_offset_col = 0;
        draw_offset_row = 0;

        rectSrcSurface.x = 0;
        rectSrcSurface.y = 0;
        fNeedBlackBackground = false;

        g_worldmap.get_world_size(&mut iWorldWidth, &mut iWorldHeight);

        if iWorldWidth >= 20 {
            rectSrcSurface.w = 640;
            draw_offset_x = 0;
        } else {
            rectSrcSurface.w = iWorldWidth as i32 * TILESIZE;
            draw_offset_x = (640 - iWorldWidth as i32 * TILESIZE) >> 1;
            fNeedBlackBackground = true;
        }

        if iWorldHeight >= 15 {
            rectSrcSurface.h = 480;
            draw_offset_y = 0;
        } else {
            rectSrcSurface.h = iWorldHeight as i32 * TILESIZE;
            draw_offset_y = (480 - iWorldHeight as i32 * TILESIZE) >> 1;
            fNeedBlackBackground = true;
        }

        rectDstSurface.x = draw_offset_x;
        rectDstSurface.y = draw_offset_y;

        updateworldsurface();
    }
}

pub fn savecurrentworld() -> i32 {
    unsafe {
        SetDisplayMessage(60, "Saved", "Your world has", "been saved.", "");

        WriteVehiclesIntoWorld();
        WriteWarpsIntoWorld();
        let path = worldlist.at(game_values.worldindex as usize).to_string_lossy().into_owned();
        g_worldmap.save(&path);
        0
    }
}

pub fn SetDisplayMessage(iTime: i16, szTitle: &str, szLine1: &str, szLine2: &str, szLine3: &str) {
    unsafe {
        g_messagedisplaytimer = iTime;
        g_szMessageTitle = szTitle.to_string();
        g_szMessageLine[0] = szLine1.to_string();
        g_szMessageLine[1] = szLine2.to_string();
        g_szMessageLine[2] = szLine3.to_string();
    }
}

pub fn findcurrentstring() -> i32 {
    unsafe {
        if !findstring.is_empty() {
            let fs = findstring.clone();
            if worldlist.find(&fs) {
                game_values.worldindex = worldlist.current_index() as i16;
                loadcurrentworld();
            }
        }

        0
    }
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

fn delete_all_vehicles_and_warps(fVehicles: bool) {
    unsafe {
        if fVehicles {
            for v in vehiclelist.drain(..) {
                v.delete();
            }
        }

        for w in warplist.drain(..) {
            w.delete();
        }
    }
}

pub fn new_world() -> i32 {
    unsafe {
        let mut fileName = String::new();
        let mut worldLocation = String::from("worlds/");
        let mut szWidth = String::new();
        let mut szHeight = String::new();

        if dialog("New World", "Enter name:", &mut fileName, 64) && dialog("New World", "Width:", &mut szWidth, 4) && dialog("New World", "Height:", &mut szHeight, 4) {
            let mut iWidth: i16 = atoi(&szWidth) as i16;
            let mut iHeight: i16 = atoi(&szHeight) as i16;

            if iWidth < 1 {
                iWidth = 1;
            }

            if iHeight < 1 {
                iHeight = 1;
            }

            delete_all_vehicles_and_warps(true);

            *g_worldmap = WorldMap::new(iWidth, iHeight);
            fileName.push_str(".txt");
            worldLocation.push_str(&fileName);
            worldlist.add(convert_path(&worldLocation).into());
            worldlist.find(&fileName);
            game_values.worldindex = worldlist.current_index() as i16;
            savecurrentworld();
            loadcurrentworld();
        }

        0
    }
}

pub fn resize_world() -> i32 {
    unsafe {
        let mut szWidth = String::new();
        let mut szHeight = String::new();

        if dialog("Resize World", "Width:", &mut szWidth, 4) && dialog("Resize World", "Height:", &mut szHeight, 4) {
            let mut iWidth: i16 = atoi(&szWidth) as i16;
            let mut iHeight: i16 = atoi(&szHeight) as i16;

            if iWidth < 1 {
                iWidth = 1;
            }

            if iHeight < 1 {
                iHeight = 1;
            }

            let mut i = 0;
            while i < vehiclelist.len() {
                let v = vehiclelist[i];
                if v.currentTile.x >= iWidth || v.currentTile.y >= iHeight {
                    RemoveVehicleFromTile(v.currentTile.x, v.currentTile.y);
                    //List was modified, restart.
                    i = 0;
                } else {
                    i += 1;
                }
            }

            delete_all_vehicles_and_warps(false);

            g_worldmap.resize(iWidth, iHeight);

            savecurrentworld();
            loadcurrentworld();
        }

        0
    }
}

//take screenshots in full and thumbnail sizes
pub fn takescreenshot() {
    unsafe {
        let iTileSizes: [i16; 3] = [32, 16, 8];

        for iScreenshotSize in 0..3usize {
            let iTileSize: i16 = iTileSizes[iScreenshotSize];
            let path = worldlist.at(game_values.worldindex as usize).to_string_lossy().into_owned();
            *g_worldmap = WorldMap::new_path(&path, iTileSize);

            let mut w: i16 = 0;
            let mut h: i16 = 0;
            g_worldmap.get_world_size(&mut w, &mut h);

            //Draw most of the world to screenshot
            let sScreenshot = SDL_CreateRGBSurface((*screen).flags, iTileSize as i32 * w as i32, iTileSize as i32 * h as i32, (*(*screen).format).BitsPerPixel as i32, 0, 0, 0, 0);
            blitdest = sScreenshot;

            g_worldmap.draw_map_to_surface_full(sScreenshot);

            //Draw vehicles to screenshot
            for vehicle in vehiclelist.iter() {
                let ix: i16 = vehicle.currentTile.x * iTileSize;
                let iy: i16 = vehicle.currentTile.y * iTileSize;

                rm.spr_worldvehicle[iScreenshotSize].draw_src(
                    ix as i32,
                    iy as i32,
                    &r(vehicle.iDrawDirection as i32 * iTileSize as i32, vehicle.iDrawSprite as i32 * iTileSize as i32, iTileSize as i32, iTileSize as i32),
                );
            }

            //Draw warps to screenshot
            for warp in warplist.iter() {
                if warp.posA.x >= 0 {
                    spr_warps[iScreenshotSize].draw_src(
                        warp.posA.x as i32 * iTileSize as i32,
                        warp.posA.y as i32 * iTileSize as i32,
                        &r(warp.id as i32 * iTileSize as i32, 0, iTileSize as i32, iTileSize as i32),
                    );
                }

                if warp.posB.x >= 0 {
                    spr_warps[iScreenshotSize].draw_src(
                        warp.posB.x as i32 * iTileSize as i32,
                        warp.posB.y as i32 * iTileSize as i32,
                        &r(warp.id as i32 * iTileSize as i32, 0, iTileSize as i32, iTileSize as i32),
                    );
                }
            }

            //Save the screenshot with the same name as the map file
            let mut szSaveFile = String::from("worlds/screenshots/");
            szSaveFile += &get_name_from_file_name(&worldlist.current_path().to_string_lossy(), false);

            if iTileSize as i32 == PREVIEWTILESIZE {
                szSaveFile += "_preview";
            } else if iTileSize as i32 == THUMBTILESIZE {
                szSaveFile += "_thumb";
            }

            szSaveFile += ".png";
            let c = CString::new(convert_path(&szSaveFile)).unwrap();
            IMG_SavePNG(sScreenshot, c.as_ptr());

            SDL_FreeSurface(sScreenshot);
        }

        let path = worldlist.at(game_values.worldindex as usize).to_string_lossy().into_owned();
        *g_worldmap = WorldMap::new_path(&path, iTileSizes[0]);
        blitdest = screen;
    }
}
