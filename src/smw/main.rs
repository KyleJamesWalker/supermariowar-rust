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
use crate::common::path::{convert_path_pack, get_home_directory};
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

#[cfg(target_os = "emscripten")]
extern "C" {
    fn emscripten_set_main_loop(func: unsafe extern "C-unwind" fn(), fps: i32, simulate_infinite_loop: i32);
    fn emscripten_get_now() -> f64;
    fn emscripten_cancel_main_loop();
    fn emscripten_run_script(script: *const std::ffi::c_char);
    fn emscripten_run_script_int(script: *const std::ffi::c_char) -> i32;
}

pub fn gameloop() {
    unsafe {
        SplashScreenState::instance().init();
        GameStateManager::instance().currentState = Ptr::from_mut(SplashScreenState::instance() as &mut dyn GameState);
        if let Some(segment) = harness::segment_state() {
            GameStateManager::instance().currentState = Ptr::from_box(Box::new(segment) as Box<dyn GameState>);
        }

        #[cfg(target_os = "emscripten")]
        emscripten_set_main_loop(gameloop_frame_web, 0, 1);

        #[cfg(not(target_os = "emscripten"))]
        while game_values.appstate != crate::common::game_values::AppState::Quit {
            gameloop_frame();
        }
    }
}

/// Not in upstream, whose web build runs one game frame per requestAnimationFrame, i.e. at the
/// display refresh rate. The game advances in fixed steps of `framelimiter` ms of wall time instead,
/// as the native frame limiter paces it, and presents once per animation frame. Under SMW_NOLIMIT
/// (replays) every callback runs exactly one frame, so the dumps stay deterministic.
#[cfg(target_os = "emscripten")]
unsafe extern "C-unwind" fn gameloop_frame_web() {
    const MAX_CATCH_UP_FRAMES: u32 = 5;
    static mut last_time: f64 = -1.0;
    static mut accumulated: f64 = 0.0;

    let frame_ms = game_values.framelimiter as f64;
    if harness::no_limit() || frame_ms <= 0.0 {
        gameloop_frame();
        web_quit_if_done();
        return;
    }

    let now = emscripten_get_now();
    if last_time >= 0.0 {
        accumulated += now - last_time;
    }
    last_time = now;

    let mut frames = 0;
    while accumulated >= frame_ms && frames < MAX_CATCH_UP_FRAMES && game_values.appstate != crate::common::game_values::AppState::Quit {
        harness::frame_start();
        GameStateManager::instance().currentState.get().update();
        harness::frame_end();

        accumulated -= frame_ms;
        frames += 1;
    }
    if frames == MAX_CATCH_UP_FRAMES {
        accumulated = accumulated.min(frame_ms);
    }

    if frames > 0 {
        gfx_flipscreen();
    }
    web_quit_if_done();
}

/// The browser main loop never returns, so when the game quits (the end of a watched replay) stop
/// the loop, close the harness, and let the page offer what to do next (web/shell.html).
#[cfg(target_os = "emscripten")]
unsafe fn web_quit_if_done() {
    if game_values.appstate != crate::common::game_values::AppState::Quit {
        return;
    }
    emscripten_cancel_main_loop();
    harness::finish();
    emscripten_run_script(c"Module.onGameQuit && Module.onGameQuit()".as_ptr());
}

unsafe extern "C-unwind" fn gameloop_frame() {
    #[cfg(not(target_os = "emscripten"))]
    use crate::smw::pad::phase;
    #[cfg(target_os = "emscripten")]
    fn phase(_: usize) {}

    phase(1);
    FPSLimiter::instance().frame_start();

    harness::frame_start();
    phase(5);
    GameStateManager::instance().currentState.get().update();
    phase(6);
    harness::frame_end();

    phase(1);
    FPSLimiter::instance().before_flip();
    phase(7);
    gfx_flipscreen();
    phase(1);
    FPSLimiter::instance().after_flip();
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
        // Not in upstream: a browser replay opens no pads, so a connected one cannot act in it.
        if let Some(count) = harness::replay_joysticks() {
            joystickcount = count;
            joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); count.max(0) as usize].into_boxed_slice()).as_mut_ptr();
            SDL_JoystickEventState(SDL_ENABLE as i32);
            return;
        }

        SDL_InitSubSystem(SDL_INIT_JOYSTICK);
        joystickcount = SDL_NumJoysticks() as i16;
        joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); joystickcount.max(0) as usize].into_boxed_slice()).as_mut_ptr();

        for i in 0..joystickcount as usize {
            *joysticks.add(i) = SDL_JoystickOpen(i as i32);
        }

        SDL_JoystickEventState(SDL_ENABLE as i32);
        #[cfg(not(target_os = "emscripten"))]
        crate::smw::pad::init(!harness::replaying() && std::env::var_os("SMW_PAD_TRANSLATE").is_some());
    }
}

/// Not in upstream: every launch gives the players the connected joysticks first, then the right
/// keyboard set (player 1's keyboard bindings), then the left one (player 2's), and makes them human.
/// The page's touch controls press the right set's default keys, so while they are on, the right set
/// goes to player 1 ahead of the joysticks. A joystick's bindings belong to it
/// (`inputConfiguration[pad][1]`), so they follow it between players.
/// The first launch with pads keeps the players' saved settings in `PAD_PLAYERS_FILE`; the next launch
/// without pads puts them back, so only launches after a pad session differ from upstream.
fn assign_inputs() {
    unsafe {
        let marker = get_home_directory() + PAD_PLAYERS_FILE;
        #[cfg(not(target_os = "emscripten"))]
        let mut pads = crate::smw::pad::player_pads();
        #[cfg(target_os = "emscripten")]
        let mut pads: Vec<usize> = (0..joystickcount.max(0) as usize).collect();
        pads.retain(|&k| k < MAX_PLAYERS as usize);
        if pads.is_empty() {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                let saved: Vec<i16> = text.split_whitespace().filter_map(|v| v.parse().ok()).collect();
                if saved.len() == MAX_PLAYERS as usize {
                    for p in 0..MAX_PLAYERS as usize {
                        game_values.playerInput.inputControls[p] = Ptr::from_mut(&mut game_values.inputConfiguration[p][0]);
                        game_values.playercontrol[p] = saved[p];
                    }
                    game_values.write_config();
                }
                let _ = std::fs::remove_file(&marker);
            }
            return;
        }
        if !std::path::Path::new(&marker).exists() {
            let saved: Vec<String> = game_values.playercontrol.iter().map(|c| c.to_string()).collect();
            let _ = std::fs::write(&marker, saved.join(" ") + "\n");
        }

        // (joystick, index): a pad, or a keyboard set (0 right, 1 left).
        let touch = touch_controls_on();
        let mut order: Vec<(bool, usize)> = Vec::new();
        if touch {
            order.push((false, 0));
        }
        order.extend(pads.iter().map(|&k| (true, k)));
        let keyboard = std::env::var_os("SMW_NO_KEYBOARD").is_none();
        if !touch && keyboard {
            order.push((false, 0));
        }
        if keyboard {
            order.push((false, 1));
        }

        for p in 0..MAX_PLAYERS as usize {
            let control = match order.get(p) {
                Some(&(true, k)) => {
                    game_values.inputConfiguration[k][1].iDevice = k as i16;
                    &mut game_values.inputConfiguration[k][1]
                }
                Some(&(false, q)) => &mut game_values.inputConfiguration[q][0],
                None => {
                    game_values.playerInput.inputControls[p] = Ptr::from_mut(&mut game_values.inputConfiguration[p][0]);
                    if !keyboard && game_values.playercontrol[p] == 1 {
                        game_values.playercontrol[p] = 2;
                    }
                    continue;
                }
            };
            game_values.playerInput.inputControls[p] = Ptr::from_mut(control);
            game_values.playercontrol[p] = 1;
        }
    }
}

/// Whether the page shows its touch controls (web/touch.js), which is decided before Play.
fn touch_controls_on() -> bool {
    #[cfg(target_os = "emscripten")]
    unsafe {
        return emscripten_run_script_int(c"document.documentElement.classList.contains('touch') ? 1 : 0".as_ptr()) != 0;
    }
    #[cfg(not(target_os = "emscripten"))]
    false
}

/// The players' settings (`playercontrol`, one number each) from before pads were assigned.
const PAD_PLAYERS_FILE: &str = "pad_players.txt";

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
    run(std::env::args().collect());
}

pub fn run(argv: Vec<String>) {
    crate::globals::init_globals();

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
    if !cmd.replay.is_empty() {
        harness::prepare_watch(&cmd.replay, cmd.replay_speed, cmd.segment);
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around main_game().
    let result = std::panic::catch_unwind(main_game);
    if let Err(payload) = result {
        let what = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        harness::finish();
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
        assign_inputs();

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
        let png = std::panic::catch_unwind(|| gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.png", &pack))));
        if let Err(payload) = png {
            let Some(err) = payload.downcast_ref::<String>() else { std::panic::resume_unwind(payload) };
            println!("\nwarning: {} -> falling back to BMP", err);
            gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.bmp", &pack)));
        }

        srand(harness::libc_seed(time(null_mut()) as u32));

        //**********************************************************

        gameloop(); // all the game logic happens here

        //**********************************************************

        println!("\n---------------- shutdown ----------------");
        harness::finish();

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
