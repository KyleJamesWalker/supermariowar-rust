//! Port of src/server (the `smw-server` lobby server, built with `IS_SERVER=1`). Part of smw-netplay; the relay
//! builds it without the `enet` feature, so without the ENet transport and the `smw_server` entry point.

pub mod blob;
pub mod clock;
pub mod log;
#[cfg(feature = "enet")]
pub mod main_server;
pub mod network_layer;
#[cfg(feature = "enet")]
pub mod network_layer_enet;
pub mod player;
pub mod room;
pub mod server;
pub mod unordered_map;
pub mod util;
