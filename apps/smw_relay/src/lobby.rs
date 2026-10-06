//! The lobby server's `NetworkLayer` and `NetPeer` over the switch's virtual connections.

use crate::server::network_layer::NetworkLayer;
use crate::switch::{ClientId, Sink};
use smw::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use smw::common_netplay::relay_frame::{self as frame, DATA, DISCONNECT};
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub enum LobbyEvent {
    Connect(RelayPeer),
    Receive(RelayPeer, Vec<u8>),
    Disconnect(RelayPeer),
}

/// What the switch hands the lobby, and the connections the lobby closed itself.
#[derive(Default)]
pub struct LobbyIo {
    pub events: VecDeque<LobbyEvent>,
    pub closed: Vec<(ClientId, u16)>,
}

#[derive(Clone)]
pub struct RelayPeer {
    pub client: ClientId,
    pub conn: u16,
    pub host: [u8; 4],
    pub port: u16,
    pub sink: Sink,
    pub io: Rc<RefCell<LobbyIo>>,
}

impl NetPeer for RelayPeer {
    fn send(&mut self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        (self.sink)(frame::encode(DATA, self.conn, data));
        true
    }

    fn send_reliable(&mut self, data: &[u8]) -> bool {
        self.send(data)
    }

    fn disconnect(&mut self) {
        (self.sink)(frame::encode(DISCONNECT, self.conn, &[]));
        self.io.borrow_mut().closed.push((self.client, self.conn));
    }

    fn address_host(&self) -> u32 {
        u32::from_ne_bytes(self.host)
    }

    fn address_port(&self) -> u16 {
        self.port
    }

    fn address_as_string(&self) -> String {
        let h = self.host;
        format!("{}.{}.{}.{}:{}", h[0], h[1], h[2], h[3], self.port)
    }

    fn average_rtt(&self) -> u32 {
        0
    }

    fn get_player_id(&self) -> u64 {
        ((self.address_host() as u64) << 16) + self.port as u64
    }

    fn same_peer(&self, other: &dyn NetPeer) -> bool {
        match other.as_any().downcast_ref::<RelayPeer>() {
            Some(o) => self.client == o.client && self.conn == o.conn,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct RelayNetworkLayer {
    io: Rc<RefCell<LobbyIo>>,
}

impl RelayNetworkLayer {
    pub fn new(io: Rc<RefCell<LobbyIo>>) -> Self {
        RelayNetworkLayer { io }
    }
}

impl NetworkLayer for RelayNetworkLayer {
    fn init(&mut self, _max_players: u64) -> bool {
        true
    }

    fn cleanup(&mut self) {}

    fn listen(&mut self, server: &mut dyn NetworkEventHandler) {
        loop {
            let event = self.io.borrow_mut().events.pop_front();
            match event {
                Some(LobbyEvent::Connect(peer)) => server.on_connect(Box::new(peer)),
                Some(LobbyEvent::Receive(mut peer, data)) => server.on_receive(&mut peer, &data),
                Some(LobbyEvent::Disconnect(mut peer)) => server.on_disconnect(&mut peer),
                None => break,
            }
        }
    }
}
