//! Scripted netplay session for interop tests against the C++ lobby server and C++ clients
//! (twin of tools/ref/net_driver.cpp). Prints `EV ...` lines describing protocol progress.
//!
//! SDL_VIDEODRIVER=dummy cargo run --example net_interop -- <host|join> <datadir> [server] [name]

use smw::common::file_list::{FiltersList, GraphicsList, SkinList};
use smw::common::game::ensure_settings_dir;
use smw::common::game_values::CGameValues;
use smw::common::gfx::gfx_init;
use smw::common::input::COutputControl;
use smw::common::resource_manager::CResourceManager;
use smw::common::tileset_manager::CTilesetManager;
use smw::globals::*;
use smw::smw::main::create_gamemodes;
use smw::smw::net::*;
use std::time::{Duration, Instant};

const FRAMES: u32 = 30;

/// Deterministic per-frame key pattern so each side can check what the other sent.
fn pattern(seed: u32, frame: u32) -> COutputControl {
    let mut c = COutputControl::default();
    for k in 0..8u32 {
        let bits = (seed.wrapping_mul(31).wrapping_add(frame * 7 + k * 3)) % 4;
        c.keys[k as usize].fDown = bits & 1 != 0;
        c.keys[k as usize].fPressed = bits & 2 != 0;
    }
    c
}

fn bits(c: &COutputControl) -> String {
    c.keys.iter().map(|k| format!("{}{}", k.fDown as u8, k.fPressed as u8)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let role = args[1].clone();
    let data = args[2].clone();
    let server = args.get(3).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let name = args.get(4).cloned().unwrap_or_else(|| if role == "host" { "RustHost".into() } else { "RustJoin".into() });
    let is_host = role == "host";

    smw::globals::init_globals();
    unsafe {
        RootDataDirectory = data.clone();
        ensure_settings_dir();
        gfx_init(640, 480, false);
        blitdest = screen;
        rm = Ptr::new_box(CResourceManager::new());
        g_tilesetmanager = Ptr::new_box(CTilesetManager::new());
        filterslist = Ptr::new_box(FiltersList::new());
        skinlist = Ptr::new_box(SkinList::new());
        menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        gamegraphicspacklist = Ptr::new_box(GraphicsList::new());
        menugraphicspacklist.set_current_index(0);
        if !game_values.is_initialized() {
            game_values.init(CGameValues::new());
        }
        CGameValues::init(&mut game_values);
        create_gamemodes();

        if !net_init() {
            println!("EV net_init failed");
            std::process::exit(2);
        }
        netplay.myPlayerName = name;
        netplay.savedServers = vec![ServerAddress { hostname: server }];
        netplay.selectedServerIndex = 0;
        netplay.mapfilepath = format!("{}/maps/0smw.map", data);
        netplay.newroom_name = "InteropRoom".to_string();

        if !net_start_session() || !netplay.client.send_connect_request_to_selected_server() {
            println!("EV connect failed");
            std::process::exit(2);
        }

        let start = Instant::now();
        let mut stage = 0;
        let mut frame = 0u32;
        let mut seen_players = 0u8;
        let mut received: Vec<String> = Vec::new();
        let other = if is_host { 1 } else { 0 };

        while start.elapsed() < Duration::from_secs(20) {
            netplay.client.update();

            match stage {
                0 if netplay.connectSuccessful => {
                    println!("EV connected");
                    if is_host {
                        netplay.client.send_create_room_message();
                    } else {
                        netplay.client.request_room_list();
                    }
                    stage = 1;
                }
                1 if is_host && netplay.joinSuccessful => {
                    println!("EV room created");
                    stage = 2;
                }
                1 if !is_host && !netplay.currentRooms.is_empty() => {
                    println!("EV room listed: {} ({}/4)", netplay.currentRooms[0].name, netplay.currentRooms[0].playerCount);
                    netplay.selectedRoomIndex = 0;
                    netplay.client.send_join_room_message();
                    stage = 2;
                }
                2 if netplay.currentRoom.player_count() >= 2 => {
                    println!(
                        "EV room {} players: {} | {} | {} | {} me={} host={}",
                        netplay.currentRoom.name,
                        netplay.currentRoom.playerNames[0],
                        netplay.currentRoom.playerNames[1],
                        netplay.currentRoom.playerNames[2],
                        netplay.currentRoom.playerNames[3],
                        netplay.remotePlayerNumber,
                        netplay.hostPlayerNumber
                    );
                    seen_players = netplay.currentRoom.player_count();
                    if is_host {
                        netplay.client.send_chat_message("hello from host");
                        netplay.client.local_gamehost.send_start_room_message();
                    }
                    stage = 3;
                }
                3 if netplay.gameRunning => {
                    println!("EV game running, players in room {}", seen_players);
                    stage = 4;
                }
                4 => {
                    if frame < FRAMES {
                        game_values.playerInput.outputControls[0] = pattern(if is_host { 1 } else { 2 }, frame);
                        netplay.client.store_local_input();
                        netplay.client.send_local_input();
                        frame += 1;
                    }
                    while let Some((_, keys)) = netplay.remote_input_buffer[other].pop_front() {
                        received.push(bits(&keys));
                    }
                    if frame >= FRAMES && (received.len() >= FRAMES as usize || start.elapsed() > Duration::from_secs(8)) {
                        stage = 5;
                    }
                }
                5 => break,
                _ => {}
            }

            std::thread::sleep(Duration::from_millis(5));
        }

        // Host-to-client keys go over the unreliable channel, so a few may be dropped; order must hold.
        let expected: Vec<String> = (0..FRAMES).map(|f| bits(&pattern(if is_host { 2 } else { 1 }, f))).collect();
        let mut it = expected.iter();
        let in_order = received.iter().all(|r| it.any(|e| e == r));
        let matched = in_order && received.len() + 3 >= expected.len();
        println!("EV inputs received {} matching {}", received.len(), matched);
        println!("EV final stage {}", stage);

        if netplay.gameRunning {
            netplay.client.send_leave_game_message();
        }
        for _ in 0..20 {
            netplay.client.update();
            std::thread::sleep(Duration::from_millis(5));
        }
        net_close();
        std::process::exit(if stage == 5 && matched { 0 } else { 1 });
    }
}
