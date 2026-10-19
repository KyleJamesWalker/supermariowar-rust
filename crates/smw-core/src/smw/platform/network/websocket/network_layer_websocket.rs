//! The network layer of every build: the lobby connection is a virtual connection to `smw_relay` over one WebSocket
//! (`socket`, docs/guides/RELAY.md).

use crate::game::Game;
use crate::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use crate::common_netplay::relay_frame::{self as frame, *};
use crate::globals::{Aliased, Ptr};
use crate::smw::network::network_layer::NetworkLayer;
use std::any::Any;
use std::collections::{BTreeMap, VecDeque};
use super::socket::{self, CLOSED as WS_CLOSED, CONNECTING as WS_CONNECTING, OPEN as WS_OPEN};

#[cfg(target_os = "emscripten")]
extern "C" {
    fn smw_relay_url_param(buf: *mut u8, cap: usize) -> i32;
}

pub const DEFAULT_RELAY_URL: &str = match option_env!("SMW_RELAY_URL") {
    Some(url) => url,
    None => "wss://smw-relay.vps.pocketsquirrel.com",
};

/// The page's `?relay=` URL, else the build's default; none for an empty `?relay=` (web_replay.mjs).
#[cfg(target_os = "emscripten")]
pub fn relay_url() -> Option<String> {
    let mut buf = [0u8; 512];
    let len = unsafe { smw_relay_url_param(buf.as_mut_ptr(), buf.len()) };
    match len {
        0 => Some(DEFAULT_RELAY_URL.to_string()),
        n if n > 0 => Some(String::from_utf8_lossy(&buf[..n as usize]).into_owned()),
        _ => None,
    }
}

/// A server list entry is a relay URL; a bare host name means `wss://<host>`.
fn url_for_server(hostname: &str) -> String {
    if hostname.contains("://") {
        hostname.to_string()
    } else {
        format!("wss://{hostname}")
    }
}

#[derive(Clone, Copy)]
pub struct Conn {
    host: [u8; 4],
    port: u16,
    open: bool,
}

pub enum Event {
    Connect(NetPeerWebSocket),
    Receive(NetPeerWebSocket, Vec<u8>),
    Disconnect(NetPeerWebSocket),
}

pub struct Relay {
    _alias: Aliased,
    url: String,
    outbox: Vec<Vec<u8>>,
    conns: BTreeMap<u16, Conn>,
    events: VecDeque<Event>,
}

impl Relay {
    pub const fn new() -> Self {
        Relay {
            _alias: Aliased::new(),
            url: String::new(),
            outbox: Vec::new(),
            conns: BTreeMap::new(),
            events: VecDeque::new(),
        }
    }
}

impl Relay {
    fn send(&mut self, kind: u8, conn: u16, payload: &[u8]) {
        let message = frame::encode(kind, conn, payload);
        match socket::state() {
            WS_OPEN => socket::send(&message),
            WS_CONNECTING => self.outbox.push(message),
            _ => {}
        }
    }

    fn peer(&mut self, conn: u16, c: &Conn) -> NetPeerWebSocket {
        NetPeerWebSocket { relay: Ptr::from_mut(self), conn, host: c.host, port: c.port }
    }

    fn open(&mut self, url: &str) {
        let state = socket::state();
        if self.url == url && (state == WS_OPEN || state == WS_CONNECTING) {
            return;
        }
        self.drop_all();
        self.url = url.to_string();
        println!("[net] Connecting to relay {}", url);
        socket::open(url);
    }

    fn close_if_idle(&mut self) {
        if self.conns.is_empty() {
            socket::close();
            self.url.clear();
            self.outbox.clear();
        }
    }

    fn connect(&mut self, host: [u8; 4], port: u16) -> bool {
        let state = socket::state();
        if state != WS_OPEN && state != WS_CONNECTING {
            return false;
        }
        let Some(conn) = (1..=u16::MAX).find(|c| !self.conns.contains_key(c)) else {
            return false;
        };
        self.conns.insert(conn, Conn { host, port, open: false });
        self.send(CONNECT, conn, &frame::encode_address(host, port));
        true
    }

    fn close_all(&mut self) {
        for conn in std::mem::take(&mut self.conns).into_keys() {
            self.send(DISCONNECT, conn, &[]);
        }
        self.events.clear();
    }

    /// Every connection ends, as when the relay goes away.
    fn drop_all(&mut self) {
        for (conn, c) in std::mem::take(&mut self.conns) {
            let peer = self.peer(conn, &c);
            self.events.push_back(Event::Disconnect(peer));
        }
        self.outbox.clear();
    }

    fn pump(&mut self, g: &mut Game) {
        let state = socket::state();
        if state == WS_OPEN {
            for message in std::mem::take(&mut self.outbox) {
                socket::send(&message);
            }
        }

        loop {
            let len = socket::recv(&mut g.recv_buffer);
            if len == -1 {
                break;
            }
            if len >= 0 {
                let message = g.recv_buffer[..len as usize].to_vec();
                self.dispatch(&message);
            }
        }

        if state == WS_CLOSED && !self.conns.is_empty() {
            println!("[net] Lost the connection to the relay.");
            self.drop_all();
        }
    }

    fn dispatch(&mut self, message: &[u8]) {
        let Some((kind, conn, payload)) = frame::decode(message) else { return };
        match kind {
            WELCOME => {
                if let Some((h, p)) = frame::decode_address(payload) {
                    println!("[net] Relay address: {}.{}.{}.{}:{}", h[0], h[1], h[2], h[3], p);
                }
            }
            ACCEPTED => {
                if let Some(c) = self.conns.get_mut(&conn) {
                    if !c.open {
                        c.open = true;
                        let c = *c;
                        let peer = self.peer(conn, &c);
                        self.events.push_back(Event::Connect(peer));
                    }
                }
            }
            DATA => {
                if let Some(c) = self.conns.get(&conn).copied() {
                    if c.open {
                        let peer = self.peer(conn, &c);
                        self.events.push_back(Event::Receive(peer, payload.to_vec()));
                    }
                }
            }
            REFUSED | DISCONNECT => {
                if let Some(c) = self.conns.remove(&conn) {
                    let peer = self.peer(conn, &c);
                    self.events.push_back(Event::Disconnect(peer));
                }
            }
            _ => {}
        }
    }
}

fn listen(g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>) {
    let mut gp = g.ptr();
    gp.relay.pump(g);
    while let Some(event) = g.relay.events.pop_front() {
        match event {
            Event::Connect(peer) => handler.on_connect(g, Box::new(peer)),
            Event::Receive(mut peer, data) => handler.on_receive(g, &mut peer, &data),
            Event::Disconnect(mut peer) => handler.on_disconnect(g, &mut peer),
        }
    }
}

pub struct NetPeerWebSocket {
    relay: Ptr<Relay>,
    conn: u16,
    host: [u8; 4],
    port: u16,
}

impl NetPeer for NetPeerWebSocket {
    fn send(&mut self, data: &[u8]) -> bool {
        if data.is_empty() || !self.relay.conns.contains_key(&self.conn) {
            return false;
        }
        self.relay.send(DATA, self.conn, data);
        true
    }

    fn send_reliable(&mut self, data: &[u8]) -> bool {
        self.send(data)
    }

    fn disconnect(&mut self) {
        if self.relay.conns.remove(&self.conn).is_some() {
            self.relay.send(DISCONNECT, self.conn, &[]);
        }
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

    fn peer_key(&self) -> u64 {
        self.conn as u64
    }

    fn same_peer(&self, other: &dyn NetPeer) -> bool {
        match other.as_any().downcast_ref::<NetPeerWebSocket>() {
            Some(o) => self.conn == o.conn,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default)]
pub struct NetworkLayerWebSocket;

impl NetworkLayer for NetworkLayerWebSocket {
    fn init(&mut self, _g: &mut Game) -> bool {
        println!("[net] WebSocket relay networking initialized.");
        true
    }

    fn cleanup(&mut self, g: &mut Game) {
        self.client_shutdown(g);
        socket::close();
    }

    fn client_restart(&mut self, g: &mut Game) -> bool {
        self.client_shutdown(g);
        true
    }

    fn client_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>) {
        listen(g, handler);
    }

    fn client_shutdown(&mut self, g: &mut Game) {
        g.relay.close_all();
        g.relay.close_if_idle();
    }

    fn connect_to_lobby_server(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        g.relay.open(&url_for_server(hostname));
        g.relay.connect([0; 4], port)
    }

}
