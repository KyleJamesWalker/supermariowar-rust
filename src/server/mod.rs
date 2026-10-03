//! Port of src/server (the `smw-server` lobby server). Compiled into the `smw_server` binary only,
//! where `IS_SERVER=1`.

pub mod blob;
pub mod clock;
pub mod log;
pub mod main_server;
pub mod network_layer;
pub mod network_layer_enet;
pub mod player;
pub mod room;
pub mod server;
pub mod unordered_map;
pub mod util;
