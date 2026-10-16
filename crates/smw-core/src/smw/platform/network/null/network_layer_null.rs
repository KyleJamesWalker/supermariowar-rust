//! Port of src/smw/platform/network/null/NetworkLayerNULL.h

use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::game::Game;
use crate::smw::network::network_layer::NetworkLayer;

#[derive(Default)]
pub struct NetworkLayerNULL;

impl NetworkLayer for NetworkLayerNULL {
    fn init(&mut self, _g: &mut Game) -> bool {
        false
    }

    fn client_restart(&mut self, _g: &mut Game) -> bool {
        false
    }
    fn client_listen(&mut self, _g: &mut Game, _handler: &mut dyn NetworkEventHandler<Game>) {}
    fn client_shutdown(&mut self, _g: &mut Game) {}

    fn gamehost_restart(&mut self, _g: &mut Game) -> bool {
        false
    }
    fn gamehost_listen(&mut self, _g: &mut Game, _handler: &mut dyn NetworkEventHandler<Game>) {}
    fn gamehost_shutdown(&mut self, _g: &mut Game) {}

    fn connect_to_lobby_server(&mut self, _g: &mut Game, _hostname: &str, _port: u16) -> bool {
        false
    }
    fn connect_to_foreign_game_host(&mut self, _g: &mut Game, _hostname: &str, _port: u16) -> bool {
        false
    }
    fn nat_punch(&mut self, _g: &mut Game, _host: u32, _port: u16) -> bool {
        false
    }
}
