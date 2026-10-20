//! Gives every WebSocket client a virtual IPv4 address and connects its virtual connections to the embedded lobby.

use crate::lobby::{LobbyEvent, LobbyIo, RelayPeer};
use smw::common_netplay::protocol_definitions::NET_LOBBYSERVER_PORT;
use smw::common_netplay::relay_frame::{self as frame, *};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// A WebSocket client, numbered from 1 in accept order.
pub type ClientId = u64;
/// Queues one frame to a WebSocket client.
pub type Sink = Rc<dyn Fn(Vec<u8>)>;

struct Client {
    sink: Sink,
    /// The virtual address the client got in its `WELCOME`.
    host: [u8; 4],
    port: u16,
    /// Its open virtual connections to the lobby.
    conns: HashSet<u16>,
}

impl Client {
    fn send(&self, kind: u8, conn: u16, payload: &[u8]) {
        (self.sink)(frame::encode(kind, conn, payload));
    }
}

/// Maps WebSocket clients and their virtual connections to lobby peers. Runs on the main thread.
pub struct Switch {
    clients: HashMap<ClientId, Client>,
    by_host: HashMap<[u8; 4], ClientId>,
    /// The low 24 bits of the last address handed out.
    next_address: u32,
    lobby: Rc<RefCell<LobbyIo>>,
    /// Virtual connections allowed per client.
    max_conns: usize,
}

impl Switch {
    pub fn new(lobby: Rc<RefCell<LobbyIo>>, max_conns: usize) -> Self {
        Switch { clients: HashMap::new(), by_host: HashMap::new(), next_address: 0, lobby, max_conns }
    }

    /// The next free address in 10.0.0.0/8, never ending in .0 or .255.
    fn allocate_host(&mut self) -> [u8; 4] {
        loop {
            self.next_address = (self.next_address + 1) & 0x00FF_FFFF;
            let n = self.next_address.to_be_bytes();
            let host = [10, n[1], n[2], n[3]];
            if n[3] != 0 && n[3] != 255 && !self.by_host.contains_key(&host) {
                return host;
            }
        }
    }

    /// Registers a new client and sends it its virtual address in a `WELCOME`.
    pub fn open(&mut self, id: ClientId, sink: Sink) {
        let host = self.allocate_host();
        let port = 20000 + (self.next_address % 40000) as u16;
        let client = Client { sink, host, port, conns: HashSet::new() };
        client.send(WELCOME, 0, &frame::encode_address(host, port));
        println!("[relay] client {} is {}.{}.{}.{}:{}", id, host[0], host[1], host[2], host[3], port);
        self.by_host.insert(host, id);
        self.clients.insert(id, client);
    }

    /// Forgets a client that went away and disconnects its lobby connections.
    pub fn close(&mut self, id: ClientId) {
        let Some(client) = self.clients.remove(&id) else { return };
        self.by_host.remove(&client.host);
        for conn in client.conns {
            self.lobby_event(LobbyEvent::Disconnect(self.lobby_peer(id, conn, &client.sink, client.host, client.port)));
        }
        println!("[relay] client {} left", id);
    }

    /// Handles one frame from a client; unknown and malformed frames are ignored.
    pub fn receive(&mut self, id: ClientId, data: &[u8]) {
        let Some((kind, conn, payload)) = frame::decode(data) else { return };
        match kind {
            CONNECT => self.connect(id, conn, payload),
            DATA => self.forward(id, conn, payload),
            DISCONNECT => self.disconnect(id, conn),
            _ => {}
        }
    }

    /// Forgets the connections the lobby closed (it already told the client).
    pub fn apply_lobby_disconnects(&mut self) {
        let closed = std::mem::take(&mut self.lobby.borrow_mut().closed);
        for (id, conn) in closed {
            if let Some(client) = self.clients.get_mut(&id) {
                client.conns.remove(&conn);
            }
        }
    }

    /// Accepts a connection to the lobby port only, up to `max_conns` per client, and refuses the rest.
    fn connect(&mut self, id: ClientId, conn: u16, payload: &[u8]) {
        let Some(client) = self.clients.get_mut(&id) else { return };
        let (sink, my_host, my_port) = (client.sink.clone(), client.host, client.port);
        let lobby = frame::decode_address(payload).is_some_and(|(_, port)| port == NET_LOBBYSERVER_PORT);
        if !lobby || client.conns.contains(&conn) || client.conns.len() >= self.max_conns {
            (sink)(frame::encode(REFUSED, conn, &[]));
            return;
        }
        client.conns.insert(conn);
        (sink)(frame::encode(ACCEPTED, conn, &[]));
        let peer = self.lobby_peer(id, conn, &sink, my_host, my_port);
        self.lobby_event(LobbyEvent::Connect(peer));
    }

    fn forward(&mut self, id: ClientId, conn: u16, payload: &[u8]) {
        let Some(client) = self.clients.get(&id) else { return };
        if client.conns.contains(&conn) {
            let peer = self.lobby_peer(id, conn, &client.sink, client.host, client.port);
            self.lobby_event(LobbyEvent::Receive(peer, payload.to_vec()));
        }
    }

    fn disconnect(&mut self, id: ClientId, conn: u16) {
        let Some(client) = self.clients.get_mut(&id) else { return };
        let (sink, host, port) = (client.sink.clone(), client.host, client.port);
        if client.conns.remove(&conn) {
            let peer = self.lobby_peer(id, conn, &sink, host, port);
            self.lobby_event(LobbyEvent::Disconnect(peer));
        }
    }

    fn lobby_peer(&self, client: ClientId, conn: u16, sink: &Sink, host: [u8; 4], port: u16) -> RelayPeer {
        RelayPeer { client, conn, host, port, sink: sink.clone(), io: self.lobby.clone() }
    }

    fn lobby_event(&self, event: LobbyEvent) {
        self.lobby.borrow_mut().events.push_back(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smw::common_netplay::network_interface::NetPeer;

    type Outbox = Rc<RefCell<Vec<Vec<u8>>>>;

    fn sink(outbox: &Outbox) -> Sink {
        let outbox = outbox.clone();
        Rc::new(move |f| outbox.borrow_mut().push(f))
    }

    fn take(outbox: &Outbox) -> Vec<(u8, u16, Vec<u8>)> {
        outbox.borrow_mut().drain(..).map(|f| {
            let (k, c, p) = frame::decode(&f).unwrap();
            (k, c, p.to_vec())
        }).collect()
    }

    fn welcome(outbox: &Outbox) -> ([u8; 4], u16) {
        let frames = take(outbox);
        assert_eq!(frames[0].0, WELCOME);
        frame::decode_address(&frames[0].2).unwrap()
    }

    fn setup() -> (Switch, Rc<RefCell<LobbyIo>>, Outbox, Outbox) {
        let io = Rc::new(RefCell::new(LobbyIo::default()));
        let mut switch = Switch::new(io.clone(), 4);
        let (a, b) = (Outbox::default(), Outbox::default());
        switch.open(1, sink(&a));
        switch.open(2, sink(&b));
        (switch, io, a, b)
    }

    #[test]
    fn lobby_connection() {
        let (mut switch, io, a, _) = setup();
        let (host, port) = welcome(&a);
        assert_eq!(host[0], 10);

        switch.receive(1, &frame::encode(CONNECT, 3, &frame::encode_address([0; 4], NET_LOBBYSERVER_PORT)));
        assert_eq!(take(&a), [(ACCEPTED, 3, vec![])]);
        switch.receive(1, &frame::encode(DATA, 3, &[0, 5, 1]));
        switch.receive(1, &frame::encode(DATA, 9, &[0, 5, 1]));

        let events: Vec<_> = io.borrow_mut().events.drain(..).collect();
        assert_eq!(events.len(), 2);
        match &events[0] {
            LobbyEvent::Connect(p) => {
                assert_eq!((p.conn, p.address_host(), p.address_port()), (3, u32::from_ne_bytes(host), port));
                assert_eq!(p.get_player_id(), ((u32::from_ne_bytes(host) as u64) << 16) + port as u64);
            }
            _ => panic!("expected a connect"),
        }
        match &events[1] {
            LobbyEvent::Receive(p, data) => {
                assert_eq!(data, &[0, 5, 1]);
                let mut p = p.clone();
                p.send_reliable(&[0, 5, 3]);
                p.disconnect();
            }
            _ => panic!("expected a receive"),
        }
        assert_eq!(take(&a), [(DATA, 3, vec![0, 5, 3]), (DISCONNECT, 3, vec![])]);
        switch.apply_lobby_disconnects();
        switch.receive(1, &frame::encode(DATA, 3, &[0, 5, 1]));
        assert!(io.borrow().events.is_empty());
    }

    #[test]
    fn closing_a_client_ends_its_connections() {
        let (mut switch, io, a, _) = setup();
        welcome(&a);
        switch.receive(1, &frame::encode(CONNECT, 2, &frame::encode_address([0; 4], NET_LOBBYSERVER_PORT)));
        take(&a);
        io.borrow_mut().events.clear();

        switch.close(1);
        assert!(matches!(io.borrow().events.front(), Some(LobbyEvent::Disconnect(p)) if p.conn == 2));
    }

    #[test]
    fn limits() {
        let (mut switch, _, a, _) = setup();
        welcome(&a);
        let lobby = frame::encode_address([0; 4], NET_LOBBYSERVER_PORT);
        switch.receive(1, &frame::encode(CONNECT, 9, &frame::encode_address([10, 0, 0, 2], 12522)));
        for conn in 1..=5 {
            switch.receive(1, &frame::encode(CONNECT, conn, &lobby));
        }
        switch.receive(1, &frame::encode(CONNECT, 1, &lobby));
        let kinds: Vec<_> = take(&a).iter().map(|f| (f.0, f.1)).collect();
        assert_eq!(kinds, [(REFUSED, 9), (ACCEPTED, 1), (ACCEPTED, 2), (ACCEPTED, 3), (ACCEPTED, 4), (REFUSED, 5), (REFUSED, 1)]);
    }

    #[test]
    fn addresses_are_unique_and_skip_broadcast() {
        let io = Rc::new(RefCell::new(LobbyIo::default()));
        let mut switch = Switch::new(io, 1);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..600 {
            let host = switch.allocate_host();
            assert!(host[3] != 0 && host[3] != 255 && host[0] == 10);
            assert!(seen.insert(host));
            switch.by_host.insert(host, 0);
        }
    }
}
