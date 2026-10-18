//! Connects to a lobby server on a clock that advances on every read and checks the Servers menu would move on: the
//! connect reply must be recorded before the skin is sent. Prints one `EV ...` line.
//!
//! SDL_VIDEODRIVER=dummy cargo run --example lobby_connect -- <datadir> [server]

use smw::common::file_list::{FiltersList, GraphicsList, SkinList};
use smw::common::game::ensure_settings_dir;
use smw::common::game_values::CGameValues;
use smw::common::gfx::gfx_init;
use smw::common::resource_manager::CResourceManager;
use smw::globals::*;
use smw::smw::main::create_gamemodes;
use smw::common_netplay::protocol_definitions::{NET_NOTICE_SKIN_CHANGE, NET_RESPONSE_CONNECT_OK};
use smw::smw::net::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

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

fn main() {
    smw::services::set_backend(smw_sdl2::services);
    let g = smw::game::Game::create();
    let mut gp = g.ptr();
    let args: Vec<String> = std::env::args().collect();
    let data = args[1].clone();
    let server = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    smw::services::services(gp.get()).clock = Box::new(TickingClock(Cell::new(0)));

    smw::globals::init_globals(gp.get());
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
    gp.netplay.myPlayerName = "Connect".to_string();
    gp.netplay.savedServers = vec![ServerAddress { hostname: server }];
    gp.netplay.selectedServerIndex = 0;

    if !net_start_session(gp.get()) || !gp.get().netplay.client.send_connect_request_to_selected_server(gp.get()) {
        println!("EV connect failed");
        std::process::exit(2);
    }

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        gp.get().netplay.client.update(gp.get());
        if gp.netplay.connectSuccessful {
            let (sent, reply) = (gp.netplay.client.lastSentMessage, gp.netplay.client.lastReply);
            let advances = sent.packageType == NET_NOTICE_SKIN_CHANGE && reply.packageType == NET_RESPONSE_CONNECT_OK;
            println!("EV servers menu advances {} (sent {}@{}, reply {}@{})", advances, sent.packageType, sent.timestamp, reply.packageType, reply.timestamp);
            net_close(gp.get());
            std::process::exit(if advances { 0 } else { 1 });
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("EV no connect reply");
    net_close(gp.get());
    std::process::exit(1);
}
