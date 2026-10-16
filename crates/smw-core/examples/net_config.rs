//! Twin of tools/ref/net_config_ref.cpp: loads servers.toml with NetConfigManager, then saves it back.
//!
//! HOME=<dir> cargo run --example net_config

#![allow(static_mut_refs)]

use smw::game::game;
use smw::smw::network::net_config_manager::NetConfigManager;

fn main() {
    let g = smw::game::Game::create();

    let mut gp = g.ptr();    smw::globals::init_globals(gp.get());
    unsafe {
        gp.netplay.myPlayerName = "Player".to_string();
    }
    NetConfigManager.load(gp.get());
    NetConfigManager.save(gp.get());
}
