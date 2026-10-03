//! Port of src/smw/platform/network/null/NetworkLayerNULL.h

use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::smw::network::network_layer::NetworkLayer;

#[derive(Default)]
pub struct NetworkLayerNULL;

impl NetworkLayer for NetworkLayerNULL {
    fn init(&mut self) -> bool {
        false
    }

    fn client_restart(&mut self) -> bool {
        false
    }
    fn client_listen(&mut self, _handler: &mut dyn NetworkEventHandler) {}
    fn client_shutdown(&mut self) {}

    fn gamehost_restart(&mut self) -> bool {
        false
    }
    fn gamehost_listen(&mut self, _handler: &mut dyn NetworkEventHandler) {}
    fn gamehost_shutdown(&mut self) {}

    fn connect_to_lobby_server(&mut self, _hostname: &str, _port: u16) -> bool {
        false
    }
    fn connect_to_foreign_game_host(&mut self, _hostname: &str, _port: u16) -> bool {
        false
    }
    fn nat_punch(&mut self, _host: u32, _port: u16) -> bool {
        false
    }
}
