//! Port of src/smw/network/NetworkLayer.h

use crate::common_netplay::network_interface::NetworkEventHandler;

pub trait NetworkLayer {
    fn init(&mut self) -> bool {
        true
    }
    fn cleanup(&mut self) {}

    fn client_restart(&mut self) -> bool;
    fn client_listen(&mut self, handler: &mut dyn NetworkEventHandler);
    fn client_shutdown(&mut self);

    fn gamehost_restart(&mut self) -> bool;
    fn gamehost_listen(&mut self, handler: &mut dyn NetworkEventHandler);
    fn gamehost_shutdown(&mut self);

    fn connect_to_lobby_server(&mut self, hostname: &str, port: u16) -> bool;
    fn connect_to_foreign_game_host(&mut self, hostname: &str, port: u16) -> bool;
    fn nat_punch(&mut self, host: u32, port: u16) -> bool;
}
