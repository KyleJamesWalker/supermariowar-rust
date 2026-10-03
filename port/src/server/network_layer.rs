//! Port of src/server/NetworkLayer.h

use smw::common_netplay::network_interface::NetworkEventHandler;

pub trait NetworkLayer {
    fn init(&mut self, max_players: u64) -> bool;
    fn cleanup(&mut self);
    fn listen(&mut self, handler: &mut dyn NetworkEventHandler);
}
