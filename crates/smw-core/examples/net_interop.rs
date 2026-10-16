//! Scripted netplay session for interop tests against the C++ lobby server and C++ clients
//! (twin of tools/ref/net_driver.cpp). Prints `EV ...` lines describing protocol progress.
//!
//! SDL_VIDEODRIVER=dummy cargo run --example net_interop -- <host|join|connect> <datadir> [server] [name]
//! `connect` only connects, on a clock that advances on every read, and checks the Servers menu would move on.

use smw::common::file_list::{FiltersList, GraphicsList, SkinList};
use smw::common::game::ensure_settings_dir;
use smw::common::game_values::CGameValues;
use smw::common::gfx::gfx_init;
use smw::common::input::COutputControl;
use smw::common::resource_manager::CResourceManager;
use smw::common::tileset_manager::CTilesetManager;
use smw::globals::*;
use smw::smw::main::create_gamemodes;
use smw::common_netplay::protocol_definitions::{NET_NOTICE_SKIN_CHANGE, NET_RESPONSE_CONNECT_OK};
use smw::smw::net::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

const FRAMES: u32 = 30;

struct TickingClock(Cell<u64>);

impl smw_platform::Clock for TickingClock {
    fn now_ms(&self) -> u64 {
        self.0.set(self.0.get() + 1);
        self.0.get()
    }

    fn sleep_ms(&self, ms: u32) {
        std::thread::sleep(Duration::from_millis(ms as u64));
    }
}

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
    let g = game().get();
    let mut gp = g.ptr();
    let args: Vec<String> = std::env::args().collect();
    let role = args[1].clone();
    let data = args[2].clone();
    let server = args.get(3).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let name = args.get(4).cloned().unwrap_or_else(|| if role == "host" { "RustHost".into() } else { "RustJoin".into() });
    let is_host = role == "host";
    if role == "connect" {
        smw::services::services(gp.get()).clock = Box::new(TickingClock(Cell::new(0)));
    }

    smw::globals::init_globals(gp.get());
    unsafe {
        gp.RootDataDirectory = data.clone();
        ensure_settings_dir(g);
        gfx_init(gp.get(), 640, 480, false);
        gp.blitdest = gp.screen;
        gp.rm = Ptr::new_box(CResourceManager::new());
        gp.filterslist = Ptr::new_box(FiltersList::new(g));
        gp.skinlist = Ptr::new_box(SkinList::new(gp.get()));
        gp.menugraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        gp.worldgraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        gp.gamegraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        gp.menugraphicspacklist.set_current_index(0);
        if !gp.game_values.is_initialized() {
            gp.game_values.init(CGameValues::new());
        }
        CGameValues::init(&mut gp.get().game_values, gp.get());
        create_gamemodes(gp.get());

        if !net_init(gp.get()) {
            println!("EV net_init failed");
            std::process::exit(2);
        }
        gp.netplay.myPlayerName = name;
        gp.netplay.savedServers = vec![ServerAddress { hostname: server }];
        gp.netplay.selectedServerIndex = 0;
        gp.netplay.mapfilepath = format!("{}/maps/0smw.map", data);
        gp.netplay.newroom_name = "InteropRoom".to_string();

        if !net_start_session(gp.get()) || !gp.get().netplay.client.send_connect_request_to_selected_server(gp.get()) {
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
            gp.get().netplay.client.update(gp.get());

            match stage {
                0 if gp.netplay.connectSuccessful && role == "connect" => {
                    let (sent, recv) = (gp.netplay.client.lastSentMessage, gp.netplay.client.lastReceivedMessage);
                    let advances = sent.packageType == NET_NOTICE_SKIN_CHANGE && recv.packageType == NET_RESPONSE_CONNECT_OK && sent.timestamp >= recv.timestamp;
                    println!("EV servers menu advances {} (sent {}@{}, received {}@{})", advances, sent.packageType, sent.timestamp, recv.packageType, recv.timestamp);
                    net_close(gp.get());
                    std::process::exit(if advances { 0 } else { 1 });
                }
                0 if gp.netplay.connectSuccessful => {
                    println!("EV connected");
                    if is_host {
                        gp.get().netplay.client.send_create_room_message(gp.get());
                    } else {
                        gp.get().netplay.client.request_room_list(gp.get());
                    }
                    stage = 1;
                }
                1 if is_host && gp.netplay.joinSuccessful => {
                    println!("EV room created");
                    stage = 2;
                }
                1 if !is_host && !gp.netplay.currentRooms.is_empty() => {
                    println!("EV room listed: {} ({}/4)", gp.netplay.currentRooms[0].name, gp.netplay.currentRooms[0].playerCount);
                    gp.netplay.selectedRoomIndex = 0;
                    gp.get().netplay.client.send_join_room_message(gp.get());
                    stage = 2;
                }
                2 if gp.netplay.currentRoom.player_count() >= 2 => {
                    println!(
                        "EV room {} players: {} | {} | {} | {} me={} host={}",
                        gp.netplay.currentRoom.name,
                        gp.netplay.currentRoom.playerNames[0],
                        gp.netplay.currentRoom.playerNames[1],
                        gp.netplay.currentRoom.playerNames[2],
                        gp.netplay.currentRoom.playerNames[3],
                        gp.netplay.remotePlayerNumber,
                        gp.netplay.hostPlayerNumber
                    );
                    seen_players = gp.netplay.currentRoom.player_count();
                    if is_host {
                        gp.get().netplay.client.send_chat_message(gp.get(), "hello from host");
                        gp.get().netplay.client.local_gamehost.send_start_room_message(gp.get());
                    }
                    stage = 3;
                }
                3 if gp.netplay.gameRunning => {
                    println!("EV game running, players in room {}", seen_players);
                    stage = 4;
                }
                4 => {
                    if frame < FRAMES {
                        gp.game_values.playerInput.outputControls[0] = pattern(if is_host { 1 } else { 2 }, frame);
                        gp.get().netplay.client.store_local_input(gp.get());
                        gp.get().netplay.client.send_local_input(gp.get());
                        frame += 1;
                    }
                    while let Some((_, keys)) = gp.netplay.remote_input_buffer[other].pop_front() {
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

        if gp.netplay.gameRunning {
            gp.get().netplay.client.send_leave_game_message(gp.get());
        }
        for _ in 0..20 {
            gp.get().netplay.client.update(gp.get());
            std::thread::sleep(Duration::from_millis(5));
        }
        net_close(gp.get());
        std::process::exit(if stage == 5 && matched { 0 } else { 1 });
    }
}
