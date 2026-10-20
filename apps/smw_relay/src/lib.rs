//! `smw_relay`: the netplay lobby over WebSocket, through a connection switch, and netplay v2 signaling on
//! `/signal/<room>`. See docs/guides/RELAY.md.
//! A native game can also run it in-process to host on its own network (`run_embedded`).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, static_mut_refs, dead_code)]

extern crate smw_netplay as smw;

use smw::server;

pub mod config;
mod lobby;
mod signal;
mod switch;
mod websocket;

pub use config::Config;
use lobby::{LobbyIo, RelayNetworkLayer};
use server::server::SMWServer;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;
use switch::Switch;
use websocket::Event;

/// True when a relay on `127.0.0.1:<port>` answers `/healthz` with 200.
pub fn healthcheck(port: u16) -> bool {
    websocket::healthcheck(port)
}

/// Serves `config` until the process ends.
pub fn run(config: &Config) -> Result<(), String> {
    let (events_tx, events) = mpsc::channel();
    websocket::serve(config, events_tx).map_err(|e| format!("Could not listen on port {}: {}", config.port, e))?;

    let io = Rc::new(RefCell::new(LobbyIo::default()));
    let mut lobby = SMWServer::new(Box::new(RelayNetworkLayer::new(io.clone())));
    if let Some(url) = &config.signal_url {
        lobby.v2.signal_url = url.clone();
    }
    let started = if config.log_file { lobby.init(&config.lobby_config) } else { lobby.init_without_log_file(&config.lobby_config) };
    if !started {
        return Err("Could not start the lobby".to_string());
    }
    if let Some(max) = config.lobby_max_players {
        lobby.set_max_players(max);
    }
    lobby.max_rooms = config.lobby_max_rooms;
    let mut switch = Switch::new(io, config.max_connections_per_client);

    println!("Ready! Port {}, allowed origins: {}", config.port, config.allowed_origins.join(", "));

    let mut running = true;
    loop {
        match events.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                handle(&mut switch, event);
                while let Ok(event) = events.try_recv() {
                    handle(&mut switch, event);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        lobby.update(&mut running);
        switch.apply_lobby_disconnects();
    }
}

/// A relay for a game hosting on its own network, on a thread of its own: any origin, no log file, signaling on
/// this relay (`/signal`), and one room of up to four players.
pub fn run_embedded(port: u16) {
    let config = Config::embedded(port);
    std::thread::Builder::new()
        .name("smw-lan-host".into())
        .spawn(move || {
            if let Err(e) = run(&config) {
                eprintln!("[net] LAN host: {e}");
            }
        })
        .expect("spawn the LAN host thread");
}

fn handle(switch: &mut Switch, event: Event) {
    match event {
        Event::Open { id, outbox } => switch.open(id, outbox.into_sink()),
        Event::Message { id, data } => switch.receive(id, &data),
        Event::Closed { id } => switch.close(id),
    }
}
