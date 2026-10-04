//! `smw_relay`: the netplay lobby server plus a connection switch, both over WebSocket, so the browser
//! build can play online. See RELAY.md.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, static_mut_refs, dead_code)]

extern crate self as smw;

#[path = "../../src/globals/pointers.rs"]
pub mod globals;

#[path = "../../src/common"]
pub mod common {
    pub mod file_io;
}

#[path = "../../src/common_netplay"]
pub mod common_netplay {
    pub mod network_interface;
    pub mod protocol_definitions;
    pub mod protocol_packages;
    pub mod relay_frame;
}

#[path = "../../src/server"]
mod server {
    pub mod blob;
    pub mod clock;
    pub mod log;
    pub mod network_layer;
    pub mod player;
    pub mod room;
    pub mod server;
    pub mod unordered_map;
    pub mod util;
}

mod config;
mod lobby;
mod switch;
mod websocket;

use config::Config;
use lobby::{LobbyIo, RelayNetworkLayer};
use server::server::{set_net_layer, SMWServer};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;
use switch::Switch;
use websocket::Event;

fn main() {
    let config = match Config::from_args_and_env() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    if config.healthcheck {
        std::process::exit(if websocket::healthcheck(config.port) { 0 } else { 1 });
    }
    exit_on_signal();

    println!("SMW relay");
    let (events_tx, events) = mpsc::channel();
    if let Err(error) = websocket::serve(&config, events_tx) {
        eprintln!("[error] Could not listen on port {}: {}", config.port, error);
        std::process::exit(1);
    }

    let io = Rc::new(RefCell::new(LobbyIo::default()));
    set_net_layer(Box::new(RelayNetworkLayer::new(io.clone())));
    let mut lobby = SMWServer::new();
    if !lobby.init(&config.lobby_config) {
        std::process::exit(1);
    }
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
            Err(RecvTimeoutError::Disconnected) => break,
        }
        lobby.update(&mut running);
        switch.apply_lobby_disconnects();
    }
}

fn handle(switch: &mut Switch, event: Event) {
    match event {
        Event::Open { id, outbox } => switch.open(id, outbox.into_sink()),
        Event::Message { id, data } => switch.receive(id, &data),
        Event::Closed { id } => switch.close(id),
    }
}

/// PID 1 in a container ignores SIGTERM unless it installs a handler.
#[cfg(unix)]
fn exit_on_signal() {
    extern "C" {
        fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
        fn _exit(status: i32) -> !;
    }
    extern "C" fn on_signal(_: i32) {
        unsafe { _exit(0) }
    }
    unsafe {
        signal(2 /* SIGINT */, on_signal);
        signal(15 /* SIGTERM */, on_signal);
    }
}

#[cfg(not(unix))]
fn exit_on_signal() {}
