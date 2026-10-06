//! Twin of tools/ref/net_config_ref.cpp: loads servers.toml with NetConfigManager, then saves it back.
//!
//! HOME=<dir> cargo run --example net_config

#![allow(static_mut_refs)]

use smw::smw::net::netplay;
use smw::smw::network::net_config_manager::NetConfigManager;

fn main() {
    smw::globals::init_globals();
    unsafe {
        netplay.myPlayerName = "Player".to_string();
    }
    NetConfigManager.load();
    NetConfigManager.save();
}
