//! The transport a lobby server listens on, installed with `set_net_layer`.

use smw::common_netplay::network_interface::NetworkEventHandler;

pub trait NetworkLayer {
    /// Prepares the transport for up to `max_players` connections; false when it cannot.
    fn init(&mut self, max_players: u64) -> bool;
    fn cleanup(&mut self);
    /// Hands every pending connect, receive and disconnect event to `handler`, without blocking.
    fn listen(&mut self, handler: &mut dyn NetworkEventHandler);
}
