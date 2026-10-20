//! The interface between a network layer and the lobby client or server that handles its events.

use std::any::Any;

/// One connection of a network layer. `same_peer` compares the underlying connection.
pub trait NetPeer: Any {
    /// Sends one message; false when it could not be queued.
    fn send(&mut self, data: &[u8]) -> bool;
    fn send_reliable(&mut self, data: &[u8]) -> bool;
    fn disconnect(&mut self);

    /// Host and port in network byte order.
    fn address_host(&self) -> u32;
    fn address_port(&self) -> u16;
    fn address_as_string(&self) -> String;

    /// Round-trip time in ms; 0 where the layer does not measure it.
    fn average_rtt(&self) -> u32;

    /// The lobby's key for this connection's player.
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

/// Receives a network layer's events. `on_connect` takes ownership of the new peer.
/// `ctx` is the state the handler works on besides its own: the game for a player, nothing for a server.
pub trait NetworkEventHandler<C: ?Sized = ()> {
    fn on_connect(&mut self, ctx: &mut C, peer: Box<dyn NetPeer>);
    fn on_receive(&mut self, ctx: &mut C, peer: &mut dyn NetPeer, data: &[u8]);
    fn on_disconnect(&mut self, ctx: &mut C, client: &mut dyn NetPeer);
}
