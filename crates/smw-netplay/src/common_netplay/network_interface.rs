//! Port of src/common_netplay/NetworkInterface.h

use std::any::Any;

/// `class NetPeer`. `operator==` compares the underlying ENet peer.
pub trait NetPeer: Any {
    fn send(&mut self, data: &[u8]) -> bool;
    fn send_reliable(&mut self, data: &[u8]) -> bool;
    fn disconnect(&mut self);

    /// Host and port in network byte order.
    fn address_host(&self) -> u32;
    fn address_port(&self) -> u16;
    fn address_as_string(&self) -> String;

    fn average_rtt(&self) -> u32;

    fn get_player_id(&self) -> u64;

    /// Tells this peer apart from the layer's other peers, as `same_peer` does.
    fn peer_key(&self) -> u64 {
        self.get_player_id()
    }

    fn same_peer(&self, other: &dyn NetPeer) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl PartialEq for dyn NetPeer {
    fn eq(&self, other: &dyn NetPeer) -> bool {
        self.same_peer(other)
    }
}

/// `class NetworkEventHandler`. `on_connect` takes ownership of the C++ `new`ed peer.
pub trait NetworkEventHandler {
    fn on_connect(&mut self, peer: Box<dyn NetPeer>);
    fn on_receive(&mut self, peer: &mut dyn NetPeer, data: &[u8]);
    fn on_disconnect(&mut self, client: &mut dyn NetPeer);
}
