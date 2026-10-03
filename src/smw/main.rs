//! Port of src/smw/main.cpp

use crate::common::cmd_args as cmd;
use crate::common::file_list::*;
use crate::common::game::{ensure_settings_dir, App};
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::game_values::{CGameValues, TITLESTRING};
use crate::smw::gamemodes::{
    bonus_house::CGM_Bonus, capture_the_flag::CGM_CaptureTheFlag, card_collection::CGM_Collection, chase::CGM_Chase, chicken::CGM_Chicken, classic::CGM_Classic, coin::CGM_Coins,
    domination::CGM_Domination, eggs::CGM_Eggs, frag::CGM_Frag, frenzy::CGM_Frenzy, greed::CGM_Greed, health::CGM_Health, jail::CGM_Jail, king_of_the_hill::CGM_KingOfTheHill,
    mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame, owned::CGM_Owned, race::CGM_Race, shy_guy_tag::CGM_ShyGuyTag, star::CGM_Star,
    stomp::CGM_Stomp, survival::CGM_Survival, tag::CGM_Tag, time_limit::CGM_TimeLimit,
};
use crate::common::map::CMap;
use crate::common::map_list::MapList;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::resource_manager::CResourceManager;
use crate::smw::player::CPlayer;
use crate::common::gfx::{gfx_close, gfx_init, gfx_flipscreen, gfx_loadpalette, gfx_settitle, gfx_show_catched_error};
use crate::common::global_constants::{HALF_PI, MAX_PLAYERS, NUM_POWERUPS, PI, THREE_HALF_PI};
use crate::common::path::convert_path_pack;
use crate::common::score::CScore;
use crate::common::sfx::{sfx_close, sfx_init};
use crate::globals::*;
use crate::smw::fps_limiter::FPSLimiter;
use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gs_splash_screen::SplashScreenState;
use crate::smw::harness;
use crate::smw::net::{net_close, net_init};
use sdl2::sys::{SDL_InitSubSystem, SDL_Joystick, SDL_JoystickEventState, SDL_JoystickOpen, SDL_NumJoysticks, SDL_ShowCursor, SDL_Surface, SDL_DISABLE, SDL_ENABLE, SDL_INIT_JOYSTICK};
use std::path::Path;
use std::ptr::null_mut;

extern "C" {
    fn srand(seed: u32);
    fn time(t: *mut i64) -> i64;
}

/// Empty in the reference build, which is compiled outside a git checkout.
const GIT_REVISION: &str = "";
const GIT_DATE: &str = "";

//------ system stuff ------
pub static mut screen: *mut SDL_Surface = null_mut(); //for gfx (maybe the gfx system should be improved -> resource manager)
pub static mut blitdest: *mut SDL_Surface = null_mut(); //the destination surface for all drawing (can be swapped from screen to another surface)

pub static mut x_shake: i16 = 0;
pub static mut y_shake: i16 = 0;

//------ game relevant stuff ------
pub static mut players: Vec<Ptr<CPlayer>> = Vec::new();

pub static mut score: [Ptr<CScore>; 4] = [Ptr::null(); 4];

pub static mut score_cnt: i16 = 0;

//Locations for swirl spawn effects
pub static mut g_iSwirlSpawnLocations: [[[i16; 25]; 2]; 4] = [[[0; 25]; 2]; 4];

pub static mut gamemodes: [Ptr<dyn CGameModeTrait>; GAMEMODE_LAST as usize] = [Ptr::null(); GAMEMODE_LAST as usize];
pub static mut bonushousemode: Ptr<CGM_Bonus> = Ptr::null();
pub static mut pipegamemode: Ptr<CGM_Pipe_MiniGame> = Ptr::null();
pub static mut bossgamemode: Ptr<CGM_Boss_MiniGame> = Ptr::null();
pub static mut boxesgamemode: Ptr<CGM_Boxes_MiniGame> = Ptr::null();

pub static mut currentgamemode: i16 = 0;

//*************************************
//  MAIN LOOP
//*************************************

pub fn gameloop() {
    unsafe {
        SplashScreenState::instance().init();
        GameStateManager::instance().currentState = Ptr::from_mut(SplashScreenState::instance() as &mut dyn GameState);

        while game_values.appstate != crate::common::game_values::AppState::Quit {
            FPSLimiter::instance().frame_start();

            harness::frame_start();
            GameStateManager::instance().currentState.get().update();
            harness::frame_end();

            FPSLimiter::instance().before_flip();
            gfx_flipscreen();
            FPSLimiter::instance().after_flip();
        }
    }
}

pub fn create_globals() {
    unsafe {
        // this instance will contain the other relevant objects
        rm = Ptr::new_box(CResourceManager::new());

        g_map = Ptr::from_box(CMap::new());
        filterslist = Ptr::new_box(FiltersList::new());
        maplist = Ptr::new_box(MapList::new(false));

        //TODO: add proper test via size
        if maplist.is_empty() {
            std::panic::panic_any("Empty map directory!".to_string());
        }

        skinlist = Ptr::new_box(SkinList::new());
        musiclist = Ptr::new_box(MusicList::new());
        worldmusiclist = Ptr::new_box(WorldMusicList::new());
        soundpacklist = Ptr::new_box(SoundsList::new());
        announcerlist = Ptr::new_box(AnnouncerList::new());
        tourlist = Ptr::new_box(TourList::new());
        worldlist = Ptr::new_box(WorldList::new());

        menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        gamegraphicspacklist = Ptr::new_box(GraphicsList::new());

        announcerlist.set_current_index(0);
        musiclist.set_current(0);
        worldmusiclist.set_current(0);
        menugraphicspacklist.set_current_index(0);
        worldgraphicspacklist.set_current_index(0);
        gamegraphicspacklist.set_current_index(0);
        soundpacklist.set_current_index(0);

        players.reserve(4);

        update_music_with_overrides(&mut musiclist, &mut worldmusiclist);
    }
}

pub fn init_joysticks() {
    unsafe {
        SDL_InitSubSystem(SDL_INIT_JOYSTICK);
        joystickcount = SDL_NumJoysticks() as i16;
        joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); joystickcount.max(0) as usize].into_boxed_slice()).as_mut_ptr();

        for i in 0..joystickcount as usize {
            *joysticks.add(i) = SDL_JoystickOpen(i as i32);
        }

        SDL_JoystickEventState(SDL_ENABLE as i32);
    }
}

fn new_mode<T: CGameModeTrait + 'static>(mode: T) -> Ptr<dyn CGameModeTrait> {
    Ptr::from_raw(Ptr::new_box(mode).as_ptr() as *mut dyn CGameModeTrait)
}

pub fn create_gamemodes() {
    unsafe {
        //set game modes
        gamemodes[0] = new_mode(CGM_Classic::new());
        gamemodes[1] = new_mode(CGM_Frag::new());
        gamemodes[2] = new_mode(CGM_TimeLimit::new());
        gamemodes[3] = new_mode(CGM_Jail::new());
        gamemodes[4] = new_mode(CGM_Coins::new());
        gamemodes[5] = new_mode(CGM_Stomp::new());
        gamemodes[6] = new_mode(CGM_Eggs::new());
        gamemodes[7] = new_mode(CGM_CaptureTheFlag::new());
        gamemodes[8] = new_mode(CGM_Chicken::new());
        gamemodes[9] = new_mode(CGM_Tag::new());
        gamemodes[10] = new_mode(CGM_Star::new());
        gamemodes[11] = new_mode(CGM_Domination::new());
        gamemodes[12] = new_mode(CGM_KingOfTheHill::new());
        gamemodes[13] = new_mode(CGM_Race::new());
        gamemodes[14] = new_mode(CGM_Owned::new());
        gamemodes[15] = new_mode(CGM_Frenzy::new());
        gamemodes[16] = new_mode(CGM_Survival::new());
        gamemodes[17] = new_mode(CGM_Greed::new());
        gamemodes[18] = new_mode(CGM_Health::new());
        gamemodes[19] = new_mode(CGM_Collection::new());
        gamemodes[20] = new_mode(CGM_Chase::new());
        gamemodes[21] = new_mode(CGM_ShyGuyTag::new());

        currentgamemode = 0;
        game_values.gamemode = gamemodes[currentgamemode as usize];

        //Special modes
        bonushousemode = Ptr::new_box(CGM_Bonus::new());
        pipegamemode = Ptr::new_box(CGM_Pipe_MiniGame::new());
        bossgamemode = Ptr::new_box(CGM_Boss_MiniGame::new());
        boxesgamemode = Ptr::new_box(CGM_Boxes_MiniGame::new());
    }
}

pub fn init_spawnlocations() {
    unsafe {
        //Calculate the swirl spawn effect locations
        let mut spawnradius: f32 = 100.0;
        let mut spawnangle: f32 = 0.0;

        for i in 0..25usize {
            g_iSwirlSpawnLocations[0][0][i] = (spawnradius * spawnangle.cos()) as i16;
            g_iSwirlSpawnLocations[0][1][i] = (spawnradius * spawnangle.sin()) as i16;

            let mut angle = spawnangle + HALF_PI;
            g_iSwirlSpawnLocations[1][0][i] = (spawnradius * angle.cos()) as i16;
            g_iSwirlSpawnLocations[1][1][i] = (spawnradius * angle.sin()) as i16;

            angle = spawnangle + PI;
            g_iSwirlSpawnLocations[2][0][i] = (spawnradius * angle.cos()) as i16;
            g_iSwirlSpawnLocations[2][1][i] = (spawnradius * angle.sin()) as i16;

            angle = spawnangle + THREE_HALF_PI;
            g_iSwirlSpawnLocations[3][0][i] = (spawnradius * angle.cos()) as i16;
            g_iSwirlSpawnLocations[3][1][i] = (spawnradius * angle.sin()) as i16;

            spawnradius -= 4.0;
            spawnangle += 0.1;
        }
    }
}

//*************************************
//  PROGRAM ENTRY POINT
//*************************************

pub fn main() {
    crate::globals::init_globals();

    let argv: Vec<String> = std::env::args().collect();
    let cmd = cmd::parse_args(&argv);
    if !cmd.success {
        std::process::exit(1);
    }
    if cmd.show_help {
        cmd::print_help(TITLESTRING, GIT_REVISION);
        return;
    }
    if cmd.debug {
        cmd::show_windows_console();
    }
    if !cmd.data_root.is_empty() {
        unsafe { RootDataDirectory = cmd.data_root.clone() };
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around main_game().
    let result = std::panic::catch_unwind(main_game);
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

pub fn main_game() {
    unsafe {
        println!("-------------------------------------------------------------------------------");
        println!(" {} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        harness::init();

        ensure_settings_dir();
        create_globals();

        gfx_init(App::screenWidth, App::screenHeight, false); //initialize the graphics (SDL)
        blitdest = screen;

        sfx_init(); //init the sound system
        net_init(); //init the networking

        init_joysticks();

        //currently this only sets the title, not the icon.
        //setting the icon isn't implemented in sdl ->  i'll ask on the mailing list
        let title = format!("{} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        gfx_settitle(&title);
        SDL_ShowCursor(SDL_DISABLE as i32);

        println!("\n---------------- loading ----------------");

        for iScore in 0..4 {
            score[iScore] = Ptr::new_box(CScore::new(iScore as i16));
        }

        CGameValues::init(&mut game_values);

        create_gamemodes();

        game_values.read_binary_config();

        //Assign the powerup weights to the selected preset
        for iPowerup in 0..NUM_POWERUPS as usize {
            game_values.powerupweights[iPowerup] = game_values.allPowerupPresets[game_values.poweruppreset as usize][iPowerup];
        }

        if game_values.fullscreen {
            crate::common::gfx::gfx_changefullscreen(true);
            blitdest = screen;
        }

        init_spawnlocations();

        //Load the gfx color palette
        let pack = gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let pngPalette = gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.png", &pack)));
        if !pngPalette {
            gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.bmp", &pack)));
        }

        srand(harness::libc_seed(time(null_mut()) as u32));

        //**********************************************************

        gameloop(); // all the game logic happens here

        //**********************************************************

        println!("\n---------------- shutdown ----------------");

        for i in 0..GAMEMODE_LAST as usize {
            gamemodes[i].delete();
        }

        sfx_close();
        gfx_close();
        net_close();

        //Delete player skins
        for k in 0..MAX_PLAYERS as usize {
            score[k].delete();
        }

        // release all resources
        rm.delete();
    }
}
