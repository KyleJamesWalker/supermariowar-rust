//! Port of src/smw/main.cpp

use crate::game::Game;
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
use crate::common::math::trig::{cosf, sinf};
use crate::common::path::{convert_path_pack, get_home_directory};
use crate::common::score::CScore;
use crate::common::sfx::{sfx_close, sfx_init};
use crate::globals::*;
use crate::smw::fps_limiter::FPSLimiter;
use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gs_splash_screen::SplashScreenState;
use crate::smw::harness;
use crate::smw::net::{net_close, net_init};
use sdl2::sys::{SDL_InitSubSystem, SDL_Joystick, SDL_JoystickEventState, SDL_JoystickOpen, SDL_NumJoysticks, SDL_Surface, SDL_ENABLE, SDL_INIT_JOYSTICK};
use std::path::Path;
use std::ptr::null_mut;

extern "C" {
    fn srand(seed: u32);
    #[cfg_attr(windows, link_name = "_time64")]
    fn time(t: *mut i64) -> i64;
}

/// Empty in the reference build, which is compiled outside a git checkout.
const GIT_REVISION: &str = "";
const GIT_DATE: &str = "";

//------ system stuff ------


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

pub fn gameloop(g: &mut Game) {
    unsafe {
        SplashScreenState::instance().init(g);
        GameStateManager::instance().currentState = Ptr::from_mut(SplashScreenState::instance() as &mut dyn GameState);
        if let Some(segment) = harness::segment_state() {
            GameStateManager::instance().currentState = Ptr::from_box(Box::new(segment) as Box<dyn GameState>);
        }

        #[cfg(target_os = "emscripten")]
        emscripten_set_main_loop(gameloop_frame_web, 0, 1);

        #[cfg(not(target_os = "emscripten"))]
        while g.game_values.appstate != crate::common::game_values::AppState::Quit {
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
    let ml = &mut game().get().main_loop;

    let frame_ms = game().game_values.framelimiter as f64;
    if harness::no_limit() || frame_ms <= 0.0 {
        gameloop_frame();
        web_quit_if_done(game().get());
        return;
    }

    let now = emscripten_get_now();
    if ml.web_last_time >= 0.0 {
        ml.web_accumulated += now - ml.web_last_time;
    }
    ml.web_last_time = now;

    let mut frames = 0;
    while ml.web_accumulated >= frame_ms && frames < MAX_CATCH_UP_FRAMES && game().game_values.appstate != crate::common::game_values::AppState::Quit {
        assign_hotplugged(game().get());
        harness::frame_start();
        GameStateManager::instance().currentState.get().update(game().get());
        harness::frame_end(game().get());

        ml.web_accumulated -= frame_ms;
        frames += 1;
    }
    if frames == MAX_CATCH_UP_FRAMES {
        ml.web_accumulated = ml.web_accumulated.min(frame_ms);
    }

    if frames > 0 {
        gfx_flipscreen(game().get());
    }
    web_quit_if_done(game().get());
}

/// The browser main loop never returns, so when the game quits (the end of a watched replay) stop
/// the loop, close the harness, and let the page offer what to do next (web/shell.html).
#[cfg(target_os = "emscripten")]
unsafe fn web_quit_if_done(g: &mut Game) {
    if g.game_values.appstate != crate::common::game_values::AppState::Quit {
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

    assign_hotplugged(game().get());
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::touch::flush();
    harness::frame_start();
    phase(5);
    GameStateManager::instance().currentState.get().update(game().get());
    phase(6);
    harness::frame_end(game().get());

    phase(1);
    FPSLimiter::instance().before_flip(game().get());
    phase(7);
    gfx_flipscreen(game().get());
    phase(1);
    FPSLimiter::instance().after_flip(game().get());
}

pub fn create_globals(g: &mut Game) {
    unsafe {
        // this instance will contain the other relevant objects
        g.rm = Ptr::new_box(CResourceManager::new());

        g.g_map = Ptr::from_box(CMap::new());
        g.filterslist = Ptr::new_box(FiltersList::new());
        g.maplist = Ptr::new_box(MapList::new(false));

        //TODO: add proper test via size
        if g.maplist.is_empty() {
            std::panic::panic_any("Empty map directory!".to_string());
        }

        g.skinlist = Ptr::new_box(SkinList::new());
        g.musiclist = Ptr::new_box(MusicList::new());
        g.worldmusiclist = Ptr::new_box(WorldMusicList::new());
        g.soundpacklist = Ptr::new_box(SoundsList::new());
        g.announcerlist = Ptr::new_box(AnnouncerList::new());
        g.tourlist = Ptr::new_box(TourList::new());
        g.worldlist = Ptr::new_box(WorldList::new());

        g.menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        g.worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        g.gamegraphicspacklist = Ptr::new_box(GraphicsList::new());

        g.announcerlist.set_current_index(0);
        g.musiclist.set_current(0);
        g.worldmusiclist.set_current(0);
        g.menugraphicspacklist.set_current_index(0);
        g.worldgraphicspacklist.set_current_index(0);
        g.gamegraphicspacklist.set_current_index(0);
        g.soundpacklist.set_current_index(0);

        g.players.reserve(4);

        update_music_with_overrides(&mut g.musiclist, &mut g.worldmusiclist);
    }
}

pub fn init_joysticks(g: &mut Game) {
    unsafe {
        // Not in upstream: a browser replay opens no pads, so a connected one cannot act in it.
        if let Some(count) = harness::replay_joysticks() {
            g.joystickcount = count;
            g.joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); count.max(0) as usize].into_boxed_slice()).as_mut_ptr();
            SDL_JoystickEventState(SDL_ENABLE as i32);
            return;
        }

        SDL_InitSubSystem(SDL_INIT_JOYSTICK);
        g.joystickcount = SDL_NumJoysticks() as i16;
        g.joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); g.joystickcount.max(0) as usize].into_boxed_slice()).as_mut_ptr();

        for i in 0..game().joystickcount as usize {
            *g.joysticks.add(i) = SDL_JoystickOpen(i as i32);
        }

        SDL_JoystickEventState(SDL_ENABLE as i32);
        #[cfg(not(target_os = "emscripten"))]
        crate::smw::pad::init(g, !harness::replaying() && std::env::var_os("SMW_PAD_TRANSLATE").is_some());
        #[cfg(not(target_os = "emscripten"))]
        crate::smw::touch::init();
    }
}

/// Not in upstream: every launch gives the players the connected joysticks first, then the right
/// keyboard set (player 1's keyboard bindings), then the left one (player 2's), and makes them human.
/// The page's touch controls press the right set's default keys, so while they are on, the right set
/// goes to player 1 ahead of the joysticks. A joystick's bindings belong to it
/// (`inputConfiguration[pad][1]`), so they follow it between players.
/// The first launch with pads keeps the players' saved settings in `PAD_PLAYERS_FILE`; the next launch
/// without pads puts them back, so only launches after a pad session differ from upstream.
/// With Android's touch controls (`touch::session`) the touch keys take player 1 or the slot after the pads,
/// the keyboard sets get no other player, and the rest are bots, also when no pad is connected.
fn assign_inputs(g: &mut Game) {
    unsafe {
        let marker = get_home_directory() + PAD_PLAYERS_FILE;
        #[cfg(not(target_os = "emscripten"))]
        let mut pads = crate::smw::pad::player_pads(g);
        #[cfg(target_os = "emscripten")]
        let mut pads: Vec<usize> = (0..game().joystickcount.max(0) as usize).filter(|k| !g.main_loop.web_removed.contains(k)).collect();
        pads.retain(|&k| k < MAX_PLAYERS as usize);
        let touch_session = touch_session();
        if pads.is_empty() && !touch_session {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                let saved: Vec<i16> = text.split_whitespace().filter_map(|v| v.parse().ok()).collect();
                if saved.len() == MAX_PLAYERS as usize {
                    for p in 0..MAX_PLAYERS as usize {
                        g.game_values.playerInput.inputControls[p] = Ptr::from_mut(&mut g.game_values.inputConfiguration[p][0]);
                        g.game_values.playercontrol[p] = saved[p];
                    }
                    game().game_values.write_config(g);
                }
                let _ = std::fs::remove_file(&marker);
            }
            return;
        }
        if !touch_session && !std::path::Path::new(&marker).exists() {
            let saved: Vec<String> = g.game_values.playercontrol.iter().map(|c| c.to_string()).collect();
            let _ = std::fs::write(&marker, saved.join(" ") + "\n");
        }

        // (joystick, index): a pad, or a keyboard set (0 right, 1 left).
        let touch = touch_controls_on();
        let mut order: Vec<(bool, usize)> = Vec::new();
        if touch {
            order.push((false, 0));
        }
        order.extend(pads.iter().map(|&k| (true, k)));
        let keyboard = std::env::var_os("SMW_NO_KEYBOARD").is_none() && !touch_session;
        if !touch && (keyboard || touch_session) {
            order.push((false, 0));
        }
        if keyboard {
            order.push((false, 1));
        }

        for p in 0..MAX_PLAYERS as usize {
            let control = match order.get(p) {
                Some(&(true, k)) => {
                    g.game_values.inputConfiguration[k][1].iDevice = k as i16;
                    &mut g.game_values.inputConfiguration[k][1]
                }
                Some(&(false, q)) => &mut g.game_values.inputConfiguration[q][0],
                None => {
                    g.game_values.playerInput.inputControls[p] = Ptr::from_mut(&mut g.game_values.inputConfiguration[p][0]);
                    if !keyboard && g.game_values.playercontrol[p] == 1 {
                        g.game_values.playercontrol[p] = 2;
                    }
                    continue;
                }
            };
            g.game_values.playerInput.inputControls[p] = Ptr::from_mut(control);
            g.game_values.playercontrol[p] = 1;
        }
    }
}

/// Not in upstream: a pad connected after launch (`pad::hotplug`, or a replay's `jadd` line) gets a player as at
/// launch, once no match is on.
fn assign_hotplugged(g: &mut Game) {
    unsafe {
        let pending = &mut game().main_loop.hotplug_pending;
        for (index, added) in harness::replay_device_changes() {
            #[cfg(not(target_os = "emscripten"))]
            crate::smw::pad::replay_device(g, index, added);
            #[cfg(target_os = "emscripten")]
            web_replay_device(g, index, added);
            *pending |= added;
        }
        #[cfg(not(target_os = "emscripten"))]
        if crate::smw::pad::hotplug(g) {
            *pending = true;
        }
        let playing = GameStateManager::instance().currentState.addr() == crate::smw::gs_gameplay::GameplayState::instance() as *mut _ as usize;
        if *pending && !playing {
            *pending = false;
            assign_inputs(g);
        }
    }
}

/// Not in the C++: the main loop's own state.
pub struct MainLoopState {
    #[cfg(target_os = "emscripten")]
    web_last_time: f64,
    #[cfg(target_os = "emscripten")]
    web_accumulated: f64,
    hotplug_pending: bool,
    /// A browser replay's joysticks exist only as the device index its events carry.
    #[cfg(target_os = "emscripten")]
    web_removed: Vec<usize>,
}

impl MainLoopState {
    pub const fn new() -> Self {
        MainLoopState {
            #[cfg(target_os = "emscripten")]
            web_last_time: -1.0,
            #[cfg(target_os = "emscripten")]
            web_accumulated: 0.0,
            hotplug_pending: false,
            #[cfg(target_os = "emscripten")]
            web_removed: Vec::new(),
        }
    }
}

#[cfg(target_os = "emscripten")]
unsafe fn web_replay_device(g: &mut Game, index: usize, added: bool) {
    let web_removed = &mut g.main_loop.web_removed;
    web_removed.retain(|&i| i != index);
    if !added {
        web_removed.push(index);
    } else if index >= g.joystickcount.max(0) as usize {
        g.joystickcount = index as i16 + 1;
        g.joysticks = Box::leak(vec![null_mut::<SDL_Joystick>(); index + 1].into_boxed_slice()).as_mut_ptr();
    }
}

fn touch_session() -> bool {
    #[cfg(not(target_os = "emscripten"))]
    return crate::smw::touch::session();
    #[cfg(target_os = "emscripten")]
    false
}

/// Whether the touch keys go to player 1 ahead of the pads: the page shows its touch controls (web/touch.js), decided
/// before Play, or Android's are on with no pad at launch (`touch::touch_first`).
fn touch_controls_on() -> bool {
    #[cfg(target_os = "emscripten")]
    unsafe {
        return emscripten_run_script_int(c"document.documentElement.classList.contains('touch') ? 1 : 0".as_ptr()) != 0;
    }
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::touch::touch_first()
}

/// The players' settings (`playercontrol`, one number each) from before pads were assigned.
const PAD_PLAYERS_FILE: &str = "pad_players.txt";

fn new_mode<T: CGameModeTrait + 'static>(mode: T) -> Ptr<dyn CGameModeTrait> {
    Ptr::from_raw(Ptr::new_box(mode).as_ptr() as *mut dyn CGameModeTrait)
}

pub fn create_gamemodes(g: &mut Game) {
    unsafe {
        //set game modes
        g.gamemodes[0] = new_mode(CGM_Classic::new());
        g.gamemodes[1] = new_mode(CGM_Frag::new());
        g.gamemodes[2] = new_mode(CGM_TimeLimit::new());
        g.gamemodes[3] = new_mode(CGM_Jail::new());
        g.gamemodes[4] = new_mode(CGM_Coins::new());
        g.gamemodes[5] = new_mode(CGM_Stomp::new());
        g.gamemodes[6] = new_mode(CGM_Eggs::new());
        g.gamemodes[7] = new_mode(CGM_CaptureTheFlag::new());
        g.gamemodes[8] = new_mode(CGM_Chicken::new());
        g.gamemodes[9] = new_mode(CGM_Tag::new());
        g.gamemodes[10] = new_mode(CGM_Star::new());
        g.gamemodes[11] = new_mode(CGM_Domination::new());
        g.gamemodes[12] = new_mode(CGM_KingOfTheHill::new());
        g.gamemodes[13] = new_mode(CGM_Race::new());
        g.gamemodes[14] = new_mode(CGM_Owned::new());
        g.gamemodes[15] = new_mode(CGM_Frenzy::new());
        g.gamemodes[16] = new_mode(CGM_Survival::new());
        g.gamemodes[17] = new_mode(CGM_Greed::new());
        g.gamemodes[18] = new_mode(CGM_Health::new());
        g.gamemodes[19] = new_mode(CGM_Collection::new());
        g.gamemodes[20] = new_mode(CGM_Chase::new());
        g.gamemodes[21] = new_mode(CGM_ShyGuyTag::new());

        g.currentgamemode = 0;
        g.game_values.gamemode = g.gamemodes[g.currentgamemode as usize];

        //Special modes
        g.bonushousemode = Ptr::new_box(CGM_Bonus::new());
        g.pipegamemode = Ptr::new_box(CGM_Pipe_MiniGame::new());
        g.bossgamemode = Ptr::new_box(CGM_Boss_MiniGame::new());
        g.boxesgamemode = Ptr::new_box(CGM_Boxes_MiniGame::new());
    }
}

pub fn init_spawnlocations(g: &mut Game) {
    unsafe {
        //Calculate the swirl spawn effect locations
        let mut spawnradius: f32 = 100.0;
        let mut spawnangle: f32 = 0.0;

        for i in 0..25usize {
            g.g_iSwirlSpawnLocations[0][0][i] = (spawnradius * cosf(spawnangle)) as i16;
            g.g_iSwirlSpawnLocations[0][1][i] = (spawnradius * sinf(spawnangle)) as i16;

            let mut angle = spawnangle + HALF_PI;
            g.g_iSwirlSpawnLocations[1][0][i] = (spawnradius * cosf(angle)) as i16;
            g.g_iSwirlSpawnLocations[1][1][i] = (spawnradius * sinf(angle)) as i16;

            angle = spawnangle + PI;
            g.g_iSwirlSpawnLocations[2][0][i] = (spawnradius * cosf(angle)) as i16;
            g.g_iSwirlSpawnLocations[2][1][i] = (spawnradius * sinf(angle)) as i16;

            angle = spawnangle + THREE_HALF_PI;
            g.g_iSwirlSpawnLocations[3][0][i] = (spawnradius * cosf(angle)) as i16;
            g.g_iSwirlSpawnLocations[3][1][i] = (spawnradius * sinf(angle)) as i16;

            spawnradius -= 4.0;
            spawnangle += 0.1;
        }
    }
}

//*************************************
//  PROGRAM ENTRY POINT
//*************************************

pub fn main() {
    cmd::attach_parent_console();
    run(std::env::args().collect());
}

pub fn run(argv: Vec<String>) {
    let g = game().get();
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
        unsafe { g.RootDataDirectory = cmd.data_root.clone() };
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
        gfx_show_catched_error(g, &what);
        std::process::exit(1);
    }
}

pub fn main_game() {
    let g = game().get();
    unsafe {
        println!("-------------------------------------------------------------------------------");
        println!(" {} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        harness::init(g);

        ensure_settings_dir();
        create_globals(g);

        gfx_init(g, App::screenWidth, App::screenHeight, false); //initialize the graphics (SDL)
        g.blitdest = g.screen;

        sfx_init(); //init the sound system
        net_init(g); //init the networking

        init_joysticks(g);

        //currently this only sets the title, not the icon.
        //setting the icon isn't implemented in sdl ->  i'll ask on the mailing list
        let title = format!("{} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        gfx_settitle(g, &title);

        println!("\n---------------- loading ----------------");

        for iScore in 0..4 {
            g.score[iScore] = Ptr::new_box(CScore::new(iScore as i16));
        }

        CGameValues::init(&mut game().game_values, g);

        create_gamemodes(g);

        g.game_values.read_binary_config();
        assign_inputs(g);

        //Assign the powerup weights to the selected preset
        for iPowerup in 0..NUM_POWERUPS as usize {
            g.game_values.powerupweights[iPowerup] = g.game_values.allPowerupPresets[g.game_values.poweruppreset as usize][iPowerup];
        }

        if g.game_values.fullscreen {
            crate::common::gfx::gfx_changefullscreen(g, true);
            g.blitdest = g.screen;
        }

        init_spawnlocations(g);

        //Load the gfx color palette
        let pack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let png = std::panic::catch_unwind(|| gfx_loadpalette(game().get(), Path::new(&convert_path_pack("gfx/packs/palette.png", &pack))));
        if let Err(payload) = png {
            let Some(err) = payload.downcast_ref::<String>() else { std::panic::resume_unwind(payload) };
            println!("\nwarning: {} -> falling back to BMP", err);
            gfx_loadpalette(g, Path::new(&convert_path_pack("gfx/packs/palette.bmp", &pack)));
        }

        srand(harness::libc_seed(time(null_mut()) as u32));

        //**********************************************************

        gameloop(g); // all the game logic happens here

        //**********************************************************

        println!("\n---------------- shutdown ----------------");
        crate::common::gfx::blit::report_fallbacks();
        harness::finish();

        for i in 0..GAMEMODE_LAST as usize {
            g.gamemodes[i].delete();
        }

        sfx_close();
        gfx_close();
        net_close(g);

        //Delete player skins
        for k in 0..MAX_PLAYERS as usize {
            g.score[k].delete();
        }

        // release all resources
        g.rm.delete();
    }
}
