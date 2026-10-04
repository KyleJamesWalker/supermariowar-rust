//! Gives every WebSocket client a virtual IPv4 address and connects its virtual connections to the
//! embedded lobby or to another client's listened port.

use crate::lobby::{LobbyEvent, LobbyIo, RelayPeer};
use smw::common_netplay::protocol_definitions::NET_LOBBYSERVER_PORT;
use smw::common_netplay::relay_frame::{self as frame, *};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type ClientId = u64;
pub type Sink = Rc<dyn Fn(Vec<u8>)>;

const MAX_LISTENED_PORTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Endpoint {
    Lobby,
    Client(ClientId, u16),
}

struct Client {
    sink: Sink,
    host: [u8; 4],
    port: u16,
    conns: HashMap<u16, Endpoint>,
    listening: Vec<u16>,
}

impl Client {
    fn send(&self, kind: u8, conn: u16, payload: &[u8]) {
        (self.sink)(frame::encode(kind, conn, payload));
    }

    fn free_incoming_conn(&self) -> Option<u16> {
        (INCOMING_CONN..=u16::MAX).find(|c| !self.conns.contains_key(c))
    }
}

pub struct Switch {
    clients: HashMap<ClientId, Client>,
    by_host: HashMap<[u8; 4], ClientId>,
    next_address: u32,
    lobby: Rc<RefCell<LobbyIo>>,
    max_conns: usize,
}

impl Switch {
    pub fn new(lobby: Rc<RefCell<LobbyIo>>, max_conns: usize) -> Self {
        Switch { clients: HashMap::new(), by_host: HashMap::new(), next_address: 0, lobby, max_conns }
    }

    /// The next free address in 10.0.0.0/8 (never .0 or .255, never 127.0.0.1, which the game rewrites).
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

    pub fn open(&mut self, id: ClientId, sink: Sink) {
        let host = self.allocate_host();
        let port = 20000 + (self.next_address % 40000) as u16;
        let client = Client { sink, host, port, conns: HashMap::new(), listening: Vec::new() };
        client.send(WELCOME, 0, &frame::encode_address(host, port));
        println!("[relay] client {} is {}.{}.{}.{}:{}", id, host[0], host[1], host[2], host[3], port);
        self.by_host.insert(host, id);
        self.clients.insert(id, client);
    }

    pub fn close(&mut self, id: ClientId) {
        let Some(client) = self.clients.remove(&id) else { return };
        self.by_host.remove(&client.host);
        for (conn, endpoint) in client.conns {
            match endpoint {
                Endpoint::Lobby => self.lobby_event(LobbyEvent::Disconnect(self.lobby_peer(id, conn, &client.sink, client.host, client.port))),
                Endpoint::Client(other, other_conn) => self.drop_far_end(other, other_conn),
            }
        }
        println!("[relay] client {} left", id);
    }

    pub fn receive(&mut self, id: ClientId, data: &[u8]) {
        let Some((kind, conn, payload)) = frame::decode(data) else { return };
        match kind {
            CONNECT => self.connect(id, conn, payload),
            DATA => self.forward(id, conn, payload),
            DISCONNECT => self.disconnect(id, conn),
            LISTEN | UNLISTEN => {
                let Some(port) = frame::decode_port(payload) else { return };
                let Some(client) = self.clients.get_mut(&id) else { return };
                client.listening.retain(|&p| p != port);
                if kind == LISTEN && port != NET_LOBBYSERVER_PORT && client.listening.len() < MAX_LISTENED_PORTS {
                    client.listening.push(port);
                }
            }
            _ => {}
        }
    }

    /// Forgets the connections the lobby closed (it already told the client).
    pub fn apply_lobby_disconnects(&mut self) {
        let closed = std::mem::take(&mut self.lobby.borrow_mut().closed);
        for (id, conn) in closed {
            if let Some(client) = self.clients.get_mut(&id) {
                if client.conns.get(&conn) == Some(&Endpoint::Lobby) {
                    client.conns.remove(&conn);
                }
            }
        }
    }

    fn connect(&mut self, id: ClientId, conn: u16, payload: &[u8]) {
        let Some(client) = self.clients.get(&id) else { return };
        let (sink, my_host, my_port) = (client.sink.clone(), client.host, client.port);
        let refuse = || (sink)(frame::encode(REFUSED, conn, &[]));

        let Some((host, port)) = frame::decode_address(payload) else { return refuse() };
        if conn >= INCOMING_CONN || client.conns.contains_key(&conn) || client.conns.len() >= self.max_conns {
            return refuse();
        }

        if port == NET_LOBBYSERVER_PORT {
            self.clients.get_mut(&id).unwrap().conns.insert(conn, Endpoint::Lobby);
            (sink)(frame::encode(ACCEPTED, conn, &[]));
            let peer = self.lobby_peer(id, conn, &sink, my_host, my_port);
            self.lobby_event(LobbyEvent::Connect(peer));
            return;
        }

        let Some(&target_id) = self.by_host.get(&host) else { return refuse() };
        let target = &self.clients[&target_id];
        if !target.listening.contains(&port) || target.conns.len() >= self.max_conns {
            return refuse();
        }
        let Some(target_conn) = target.free_incoming_conn() else { return refuse() };

        self.clients.get_mut(&target_id).unwrap().conns.insert(target_conn, Endpoint::Client(id, conn));
        self.clients.get_mut(&id).unwrap().conns.insert(conn, Endpoint::Client(target_id, target_conn));
        self.clients[&target_id].send(INCOMING, target_conn, &frame::encode_address(my_host, my_port));
        (sink)(frame::encode(ACCEPTED, conn, &[]));
    }

    fn forward(&mut self, id: ClientId, conn: u16, payload: &[u8]) {
        let Some(client) = self.clients.get(&id) else { return };
        match client.conns.get(&conn) {
            Some(Endpoint::Lobby) => {
                let peer = self.lobby_peer(id, conn, &client.sink, client.host, client.port);
                self.lobby_event(LobbyEvent::Receive(peer, payload.to_vec()));
            }
            Some(&Endpoint::Client(other, other_conn)) => {
                if let Some(other) = self.clients.get(&other) {
                    other.send(DATA, other_conn, payload);
                }
            }
            None => {}
        }
    }

    fn disconnect(&mut self, id: ClientId, conn: u16) {
        let Some(client) = self.clients.get_mut(&id) else { return };
        let (sink, host, port) = (client.sink.clone(), client.host, client.port);
        match client.conns.remove(&conn) {
            Some(Endpoint::Lobby) => {
                let peer = self.lobby_peer(id, conn, &sink, host, port);
                self.lobby_event(LobbyEvent::Disconnect(peer));
            }
            Some(Endpoint::Client(other, other_conn)) => self.drop_far_end(other, other_conn),
            None => {}
        }
    }

    fn drop_far_end(&mut self, id: ClientId, conn: u16) {
        if let Some(client) = self.clients.get_mut(&id) {
            if client.conns.remove(&conn).is_some() {
                client.send(DISCONNECT, conn, &[]);
            }
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
    use smw::common_netplay::protocol_definitions::NET_GAMEHOST_PORT;

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
    fn game_host_connection() {
        let (mut switch, io, a, b) = setup();
        let (host_a, _) = welcome(&a);
        let (host_b, port_b) = welcome(&b);
        let to_a = frame::encode_address(host_a, NET_GAMEHOST_PORT);

        switch.receive(2, &frame::encode(CONNECT, 1, &to_a));
        assert_eq!(take(&b), [(REFUSED, 1, vec![])]);

        switch.receive(1, &frame::encode(LISTEN, 0, &NET_GAMEHOST_PORT.to_be_bytes()));
        switch.receive(2, &frame::encode(CONNECT, 1, &to_a));
        assert_eq!(take(&b), [(ACCEPTED, 1, vec![])]);
        assert_eq!(take(&a), [(INCOMING, INCOMING_CONN, frame::encode_address(host_b, port_b).to_vec())]);

        switch.receive(2, &frame::encode(DATA, 1, &[1, 2, 3]));
        switch.receive(1, &frame::encode(DATA, INCOMING_CONN, &[4]));
        assert_eq!(take(&a), [(DATA, INCOMING_CONN, vec![1, 2, 3])]);
        assert_eq!(take(&b), [(DATA, 1, vec![4])]);

        switch.receive(2, &frame::encode(DISCONNECT, 1, &[]));
        assert_eq!(take(&a), [(DISCONNECT, INCOMING_CONN, vec![])]);
        switch.receive(1, &frame::encode(DATA, INCOMING_CONN, &[4]));
        assert!(take(&b).is_empty());
        assert!(io.borrow().events.is_empty());
    }

    #[test]
    fn closing_a_client_ends_its_connections() {
        let (mut switch, io, a, b) = setup();
        let (host_a, _) = welcome(&a);
        welcome(&b);
        switch.receive(1, &frame::encode(LISTEN, 0, &NET_GAMEHOST_PORT.to_be_bytes()));
        switch.receive(2, &frame::encode(CONNECT, 1, &frame::encode_address(host_a, NET_GAMEHOST_PORT)));
        switch.receive(1, &frame::encode(CONNECT, 2, &frame::encode_address([0; 4], NET_LOBBYSERVER_PORT)));
        take(&a);
        take(&b);
        io.borrow_mut().events.clear();

        switch.close(1);
        assert_eq!(take(&b), [(DISCONNECT, 1, vec![])]);
        assert!(matches!(io.borrow().events.front(), Some(LobbyEvent::Disconnect(p)) if p.conn == 2));

        switch.receive(2, &frame::encode(CONNECT, 2, &frame::encode_address(host_a, NET_GAMEHOST_PORT)));
        assert_eq!(take(&b), [(REFUSED, 2, vec![])]);
    }

    #[test]
    fn limits() {
        let (mut switch, _, a, _) = setup();
        welcome(&a);
        let lobby = frame::encode_address([0; 4], NET_LOBBYSERVER_PORT);
        switch.receive(1, &frame::encode(CONNECT, INCOMING_CONN, &lobby));
        for conn in 1..=5 {
            switch.receive(1, &frame::encode(CONNECT, conn, &lobby));
        }
        switch.receive(1, &frame::encode(CONNECT, 1, &lobby));
        let kinds: Vec<_> = take(&a).iter().map(|f| (f.0, f.1)).collect();
        assert_eq!(kinds, [(REFUSED, INCOMING_CONN), (ACCEPTED, 1), (ACCEPTED, 2), (ACCEPTED, 3), (ACCEPTED, 4), (REFUSED, 5), (REFUSED, 1)]);
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
