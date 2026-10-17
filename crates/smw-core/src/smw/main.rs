//! Port of src/smw/main.cpp

use crate::game::{Device, Game, MAX_DEVICES};
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
    fn emscripten_set_main_loop_arg(func: unsafe extern "C-unwind" fn(*mut std::ffi::c_void), arg: *mut std::ffi::c_void, fps: i32, simulate_infinite_loop: i32);
    fn emscripten_get_now() -> f64;
    fn emscripten_cancel_main_loop();
    fn emscripten_run_script(script: *const std::ffi::c_char);
    fn emscripten_run_script_int(script: *const std::ffi::c_char) -> i32;
}

pub fn gameloop(g: &mut Game) {
    unsafe {
        gameloop_start(g);

        #[cfg(target_os = "emscripten")]
        emscripten_set_main_loop_arg(gameloop_frame_web, g as *mut Game as *mut std::ffi::c_void, 0, 1);

        #[cfg(not(target_os = "emscripten"))]
        while g.game_values.appstate != crate::common::game_values::AppState::Quit {
            gameloop_frame(g);
        }
    }
}

/// The state before the first frame.
pub fn gameloop_start(g: &mut Game) {
    SplashScreenState::instance(g).init(g);
    GameStateManager::instance(g).currentState = crate::smw::game_state::StateId::Splash;
    if let Some(segment) = harness::segment_state(g) {
        GameStateManager::instance(g).start_segment(Box::new(segment));
    }
}

/// One frame of the native main loop.
pub fn frame(g: &mut Game) {
    unsafe { gameloop_frame(g) }
}

/// One frame's game logic, without pacing or presenting: the web loop's step, and a frame of a `Game::clone_match` copy.
pub fn step(g: &mut Game) {
    let mut gp = g.ptr();
    assign_hotplugged(g);
    harness::frame_start(g);
    GameStateManager::instance(gp.get()).current(gp.get()).get().update(gp.get());
    harness::frame_end(g);
}

/// Not in upstream, whose web build runs one game frame per requestAnimationFrame, i.e. at the
/// display refresh rate. The game advances in fixed steps of `framelimiter` ms of wall time instead,
/// as the native frame limiter paces it, and presents once per animation frame. Under SMW_NOLIMIT
/// (replays) every callback runs exactly one frame, so the dumps stay deterministic.
#[cfg(target_os = "emscripten")]
unsafe extern "C-unwind" fn gameloop_frame_web(arg: *mut std::ffi::c_void) {
    const MAX_CATCH_UP_FRAMES: u32 = 5;
    let g = &mut *(arg as *mut Game);
    let mut gp = g.ptr();
    let ml = &mut gp.get().main_loop;

    let frame_ms = gp.game_values.framelimiter as f64;
    if harness::no_limit(g) || frame_ms <= 0.0 {
        gameloop_frame(gp.get());
        web_quit_if_done(gp.get());
        return;
    }

    let now = emscripten_get_now();
    if ml.web_last_time >= 0.0 {
        ml.web_accumulated += now - ml.web_last_time;
    }
    ml.web_last_time = now;

    let mut frames = 0;
    while ml.web_accumulated >= frame_ms && frames < MAX_CATCH_UP_FRAMES && gp.game_values.appstate != crate::common::game_values::AppState::Quit {
        step(gp.get());

        ml.web_accumulated -= frame_ms;
        frames += 1;
    }
    if frames == MAX_CATCH_UP_FRAMES {
        ml.web_accumulated = ml.web_accumulated.min(frame_ms);
    }

    if frames > 0 {
        gfx_flipscreen(gp.get());
    }
    web_quit_if_done(gp.get());
}

/// The browser main loop never returns, so when the game quits (the end of a watched replay) stop
/// the loop, close the harness, and let the page offer what to do next (web/shell.html).
#[cfg(target_os = "emscripten")]
unsafe fn web_quit_if_done(g: &mut Game) {
    if g.game_values.appstate != crate::common::game_values::AppState::Quit {
        return;
    }
    emscripten_cancel_main_loop();
    harness::finish(g);
    emscripten_run_script(c"Module.onGameQuit && Module.onGameQuit()".as_ptr());
}

unsafe fn gameloop_frame(g: &mut Game) {
    let mut gp = g.ptr();
    #[cfg(not(target_os = "emscripten"))]
    use crate::smw::pad::phase;
    #[cfg(target_os = "emscripten")]
    fn phase(_: usize) {}

    phase(1);
    FPSLimiter::instance(gp.get()).frame_start(g);

    assign_hotplugged(gp.get());
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::touch::flush(g);
    harness::frame_start(gp.get());
    phase(5);
    GameStateManager::instance(gp.get()).current(gp.get()).get().update(gp.get());
    phase(6);
    harness::frame_end(gp.get());

    phase(1);
    FPSLimiter::instance(gp.get()).before_flip(gp.get());
    phase(7);
    gfx_flipscreen(gp.get());
    phase(1);
    FPSLimiter::instance(gp.get()).after_flip(gp.get());
}

pub fn create_globals(g: &mut Game) {
        let mut gp = g.ptr();
    // this instance will contain the other relevant objects
    g.rm = Ptr::new_box(CResourceManager::new());

    g.g_map.set(CMap::new());
    g.filterslist = Ptr::new_box(FiltersList::new(g));
    g.maplist = Ptr::new_box(MapList::new(g, false));

    //TODO: add proper test via size
    if g.maplist.is_empty() {
        std::panic::panic_any("Empty map directory!".to_string());
    }

    g.skinlist = Ptr::new_box(SkinList::new(g));
    g.musiclist = Ptr::new_box(MusicList::new(g));
    g.worldmusiclist = Ptr::new_box(WorldMusicList::new(g));
    g.soundpacklist = Ptr::new_box(SoundsList::new(g));
    g.announcerlist = Ptr::new_box(AnnouncerList::new(g));
    g.tourlist = Ptr::new_box(TourList::new(g));
    g.worldlist = Ptr::new_box(WorldList::new(g));

    g.menugraphicspacklist = Ptr::new_box(GraphicsList::new(g));
    g.worldgraphicspacklist = Ptr::new_box(GraphicsList::new(g));
    g.gamegraphicspacklist = Ptr::new_box(GraphicsList::new(g));

    g.announcerlist.set_current_index(0);
    g.musiclist.set_current(0);
    g.worldmusiclist.set_current(0);
    g.menugraphicspacklist.set_current_index(0);
    g.worldgraphicspacklist.set_current_index(0);
    g.gamegraphicspacklist.set_current_index(0);
    g.soundpacklist.set_current_index(0);

    g.players.reserve(4);

    update_music_with_overrides(g, &mut gp.get().musiclist, &mut gp.get().worldmusiclist);
}

pub fn init_joysticks(g: &mut Game) {
    // Not in upstream: a replay opens no pads, so a connected one cannot act in it.
    let replay = harness::replay_joysticks(g);
    #[cfg(not(target_os = "emscripten"))]
    let pads = {
        let translate = !harness::replaying(g) && std::env::var_os("SMW_PAD_TRANSLATE").is_none_or(|v| v != "0");
        crate::smw::pad::init(g, replay.is_none(), translate)
    };
    #[cfg(target_os = "emscripten")]
    let pads: Vec<Option<Device>> =
        crate::services::services(g).pads.open_all(replay.is_none(), false).into_iter().map(|p| p.map(|p| Device::Pad { pad: p.pad, controller: p.controller })).collect();
    g.devices = match replay {
        Some(count) => vec![Some(Device::Replay); count],
        None => pads,
    };
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::touch::init(g);
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
        let mut gp = g.ptr();
    let marker = get_home_directory(g) + PAD_PLAYERS_FILE;
    #[cfg(not(target_os = "emscripten"))]
    let mut pads = crate::smw::pad::player_pads(g);
    #[cfg(target_os = "emscripten")]
    let mut pads: Vec<usize> = (0..gp.devices.len()).filter(|&k| gp.devices[k].is_some()).collect();
    pads.retain(|&k| k < MAX_PLAYERS as usize);
    let touch_session = touch_session(g);
    if pads.is_empty() && !touch_session {
        if let Ok(text) = std::fs::read_to_string(&marker) {
            let saved: Vec<i16> = text.split_whitespace().filter_map(|v| v.parse().ok()).collect();
            if saved.len() == MAX_PLAYERS as usize {
                for p in 0..MAX_PLAYERS as usize {
                    g.game_values.playerInput.inputControls[p] = Some(((p) as u8, (0) as u8));
                    g.game_values.playercontrol[p] = saved[p];
                }
                gp.game_values.write_config(g);
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
    let touch = touch_controls_on(g);
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
                (k as u8, 1)
            }
            Some(&(false, q)) => (q as u8, 0),
            None => {
                g.game_values.playerInput.inputControls[p] = Some(((p) as u8, (0) as u8));
                if !keyboard && g.game_values.playercontrol[p] == 1 {
                    g.game_values.playercontrol[p] = 2;
                }
                continue;
            }
        };
        g.game_values.playerInput.inputControls[p] = Some(control);
        g.game_values.playercontrol[p] = 1;
    }
}

/// Not in upstream: a pad connected after launch (`pad::hotplug`, or a replay's `jadd` line) gets a player as at
/// launch, once no match is on.
fn assign_hotplugged(g: &mut Game) {
        let mut gp = g.ptr();
    let pending = &mut gp.main_loop.hotplug_pending;
    for (index, added) in harness::replay_device_changes(g) {
        replay_device(g, index, added);
        *pending |= added;
    }
    #[cfg(not(target_os = "emscripten"))]
    if crate::smw::pad::hotplug(g) {
        *pending = true;
    }
    let playing = GameStateManager::instance(g).currentState == crate::smw::game_state::StateId::Gameplay;
    if *pending && !playing {
        *pending = false;
        assign_inputs(g);
    }
}

/// Not in the C++: the main loop's own state.
#[derive(Clone)]
pub struct MainLoopState {
    #[cfg(target_os = "emscripten")]
    web_last_time: f64,
    #[cfg(target_os = "emscripten")]
    web_accumulated: f64,
    /// A replay of a touch session: whether no pad was open at launch, as Android's `touch::init` decides.
    #[cfg(target_os = "emscripten")]
    web_replay_touch_first: Option<bool>,
    hotplug_pending: bool,
    /// Frames the current screen has waited for an all-CPU game to continue (smw/auto_advance.rs).
    pub(crate) auto_advance: u16,
}

impl MainLoopState {
    pub const fn new() -> Self {
        MainLoopState {
            #[cfg(target_os = "emscripten")]
            web_last_time: -1.0,
            #[cfg(target_os = "emscripten")]
            web_replay_touch_first: None,
            #[cfg(target_os = "emscripten")]
            web_accumulated: 0.0,
            hotplug_pending: false,
            auto_advance: 0,
        }
    }
}

/// A replay's `jadd` (`added`) or `jremove` line.
fn replay_device(g: &mut Game, index: usize, added: bool) {
    if index >= MAX_DEVICES || (!added && index >= g.devices.len()) {
        return;
    }
    if index >= g.devices.len() {
        g.devices.resize(index + 1, None);
    }
    if g.devices[index].is_some() == added {
        return;
    }
    g.devices[index] = added.then_some(Device::Replay);
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::devices_changed();
}

fn touch_session(g: &mut Game) -> bool {
    #[cfg(not(target_os = "emscripten"))]
    return crate::smw::touch::session(g);
    #[cfg(target_os = "emscripten")]
    harness::replay_touch(g)
}

/// Whether the touch keys go to player 1 ahead of the pads: the page shows its touch controls (web/touch.js), decided
/// before Play, or Android's are on with no pad at launch (`touch::touch_first`).
fn touch_controls_on(g: &mut Game) -> bool {
    #[cfg(target_os = "emscripten")]
    unsafe {
        if harness::replay_touch(g) {
            let no_pads = g.devices.is_empty();
            return *g.main_loop.web_replay_touch_first.get_or_insert(no_pads);
        }
        return emscripten_run_script_int(c"document.documentElement.classList.contains('touch') ? 1 : 0".as_ptr()) != 0;
    }
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::touch::touch_first(g)
}

/// The players' settings (`playercontrol`, one number each) from before pads were assigned.
const PAD_PLAYERS_FILE: &str = "pad_players.txt";

pub fn create_gamemodes(g: &mut Game) {
    //set game modes
    g.gamemodes[0].set(Box::new(CGM_Classic::new()));
    g.gamemodes[1].set(Box::new(CGM_Frag::new()));
    g.gamemodes[2].set(Box::new(CGM_TimeLimit::new()));
    g.gamemodes[3].set(Box::new(CGM_Jail::new()));
    g.gamemodes[4].set(Box::new(CGM_Coins::new()));
    g.gamemodes[5].set(Box::new(CGM_Stomp::new()));
    g.gamemodes[6].set(Box::new(CGM_Eggs::new()));
    g.gamemodes[7].set(Box::new(CGM_CaptureTheFlag::new()));
    g.gamemodes[8].set(Box::new(CGM_Chicken::new()));
    g.gamemodes[9].set(Box::new(CGM_Tag::new()));
    g.gamemodes[10].set(Box::new(CGM_Star::new()));
    g.gamemodes[11].set(Box::new(CGM_Domination::new()));
    g.gamemodes[12].set(Box::new(CGM_KingOfTheHill::new()));
    g.gamemodes[13].set(Box::new(CGM_Race::new()));
    g.gamemodes[14].set(Box::new(CGM_Owned::new()));
    g.gamemodes[15].set(Box::new(CGM_Frenzy::new()));
    g.gamemodes[16].set(Box::new(CGM_Survival::new()));
    g.gamemodes[17].set(Box::new(CGM_Greed::new()));
    g.gamemodes[18].set(Box::new(CGM_Health::new()));
    g.gamemodes[19].set(Box::new(CGM_Collection::new()));
    g.gamemodes[20].set(Box::new(CGM_Chase::new()));
    g.gamemodes[21].set(Box::new(CGM_ShyGuyTag::new()));

    g.currentgamemode = 0;
    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((g.currentgamemode as usize) as u8));

    //Special modes
    g.bonushousemode.set(Box::new(CGM_Bonus::new()));
    g.pipegamemode.set(Box::new(CGM_Pipe_MiniGame::new()));
    g.bossgamemode.set(Box::new(CGM_Boss_MiniGame::new()));
    g.boxesgamemode.set(Box::new(CGM_Boxes_MiniGame::new()));
}

pub fn init_spawnlocations(g: &mut Game) {
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

//*************************************
//  PROGRAM ENTRY POINT
//*************************************

pub fn main() {
    cmd::attach_parent_console();
    run(std::env::args().collect());
}

pub fn run(argv: Vec<String>) {
    run_with(argv, None);
}

/// `run` on the given backend instead of SDL2.
pub fn run_with(argv: Vec<String>, services: Option<smw_platform::Services>) {
    let g = Game::create();
    g.services = services;
    crate::globals::init_globals(g);

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
        g.RootDataDirectory = cmd.data_root.clone();
    }
    if !cmd.replay.is_empty() {
        harness::prepare_watch(g, &cmd.replay, cmd.replay_speed, cmd.segment);
    }

    // C++ catches `const char*`, `std::string`, `std::exception` and `...` around main_game().
    let mut gp = g.ptr();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| main_game(gp.get())));
    if let Err(payload) = result {
        let what = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        harness::finish(g);
        gfx_show_catched_error(g, &what);
        std::process::exit(1);
    }
}

pub fn main_game(g: &mut Game) {
    startup(g);

    //**********************************************************

    gameloop(g); // all the game logic happens here

    //**********************************************************

    shutdown(g);
}

/// Everything `main_game` does before its main loop.
pub fn startup(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        println!("-------------------------------------------------------------------------------");
        println!(" {} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        println!("-------------------------------------------------------------------------------");
        println!("\n---------------- startup ----------------");

        harness::init(g);

        ensure_settings_dir(g);
        create_globals(g);

        gfx_init(g, App::screenWidth, App::screenHeight, false); //initialize the graphics (SDL)
        g.blitdest = g.screen;

        sfx_init(g); //init the sound system
        net_init(g); //init the networking

        init_joysticks(g);

        //currently this only sets the title, not the icon.
        //setting the icon isn't implemented in sdl ->  i'll ask on the mailing list
        let title = format!("{} {} {}", TITLESTRING, GIT_REVISION, GIT_DATE);
        gfx_settitle(g, &title);

        println!("\n---------------- loading ----------------");

        for iScore in 0..4 {
            g.score[iScore].init(CScore::new(iScore as i16));
        }

        CGameValues::init(&mut gp.game_values, g);

        create_gamemodes(g);

        gp.game_values.read_binary_config(g);
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
        let png = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gfx_loadpalette(gp.get(), Path::new(&convert_path_pack(gp.get(), "gfx/packs/palette.png", &pack)))));
        if let Err(payload) = png {
            let Some(err) = payload.downcast_ref::<String>() else { std::panic::resume_unwind(payload) };
            println!("\nwarning: {} -> falling back to BMP", err);
            gfx_loadpalette(g, Path::new(&convert_path_pack(gp.get(), "gfx/packs/palette.bmp", &pack)));
        }

        srand(harness::libc_seed(g, time(null_mut()) as u32));
    }
}

/// Everything `main_game` does after its main loop.
pub fn shutdown(g: &mut Game) {
    println!("\n---------------- shutdown ----------------");
    harness::finish(g);

    sfx_close(g);
    gfx_close();
    net_close(g);

    // release all resources
    g.rm.delete();
}
