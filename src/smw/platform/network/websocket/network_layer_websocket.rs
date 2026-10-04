//! Not in the C++: the browser's network layer. Each ENet connection becomes a virtual connection to
//! `smw_relay`, all of them over one WebSocket (web/relay_socket.js, RELAY.md).

use crate::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use crate::common_netplay::protocol_definitions::NET_GAMEHOST_PORT;
use crate::common_netplay::relay_frame::{self as frame, *};
use crate::smw::network::network_layer::NetworkLayer;
use std::any::Any;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::CString;
use std::net::Ipv4Addr;
use std::os::raw::c_char;

extern "C" {
    fn smw_ws_open(url: *const c_char);
    fn smw_ws_state() -> i32;
    fn smw_ws_send(data: *const u8, len: usize) -> i32;
    fn smw_ws_recv(buf: *mut u8, cap: usize) -> i32;
    fn smw_ws_close();
    fn smw_relay_url_param(buf: *mut u8, cap: usize) -> i32;
}

const WS_CONNECTING: i32 = 0;
const WS_OPEN: i32 = 1;
const WS_CLOSED: i32 = 2;

pub const DEFAULT_RELAY_URL: &str = match option_env!("SMW_RELAY_URL") {
    Some(url) => url,
    None => "wss://smw-relay.kylejameswalker.com",
};

/// The page's `?relay=` URL, else the build's default.
pub fn relay_url() -> String {
    let mut buf = [0u8; 512];
    let len = unsafe { smw_relay_url_param(buf.as_mut_ptr(), buf.len()) };
    if len > 0 {
        String::from_utf8_lossy(&buf[..len as usize]).into_owned()
    } else {
        DEFAULT_RELAY_URL.to_string()
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

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Client,
    GameHost,
}

#[derive(Clone, Copy)]
struct Conn {
    side: Side,
    host: [u8; 4],
    port: u16,
    open: bool,
}

enum Event {
    Connect(NetPeerWebSocket),
    Receive(NetPeerWebSocket, Vec<u8>),
    Disconnect(NetPeerWebSocket),
}

struct Relay {
    url: String,
    outbox: Vec<Vec<u8>>,
    conns: BTreeMap<u16, Conn>,
    client_events: VecDeque<Event>,
    gamehost_events: VecDeque<Event>,
    listening: bool,
}

static mut relay: Relay = Relay {
    url: String::new(),
    outbox: Vec::new(),
    conns: BTreeMap::new(),
    client_events: VecDeque::new(),
    gamehost_events: VecDeque::new(),
    listening: false,
};

static mut recv_buffer: [u8; MAX_MESSAGE_LEN] = [0; MAX_MESSAGE_LEN];

impl Relay {
    fn send(&mut self, kind: u8, conn: u16, payload: &[u8]) {
        let message = frame::encode(kind, conn, payload);
        match unsafe { smw_ws_state() } {
            WS_OPEN => unsafe {
                smw_ws_send(message.as_ptr(), message.len());
            },
            WS_CONNECTING => self.outbox.push(message),
            _ => {}
        }
    }

    fn events(&mut self, side: Side) -> &mut VecDeque<Event> {
        match side {
            Side::Client => &mut self.client_events,
            Side::GameHost => &mut self.gamehost_events,
        }
    }

    fn open(&mut self, url: &str) {
        let state = unsafe { smw_ws_state() };
        if self.url == url && (state == WS_OPEN || state == WS_CONNECTING) {
            return;
        }
        self.drop_all();
        self.url = url.to_string();
        println!("[net] Connecting to relay {}", url);
        let c_url = CString::new(url).unwrap_or_default();
        unsafe { smw_ws_open(c_url.as_ptr()) };
    }

    fn close_if_idle(&mut self) {
        if self.conns.is_empty() && !self.listening {
            unsafe { smw_ws_close() };
            self.url.clear();
            self.outbox.clear();
        }
    }

    fn connect(&mut self, host: [u8; 4], port: u16) -> bool {
        let state = unsafe { smw_ws_state() };
        if state != WS_OPEN && state != WS_CONNECTING {
            return false;
        }
        let Some(conn) = (1..INCOMING_CONN).find(|c| !self.conns.contains_key(c)) else {
            return false;
        };
        self.conns.insert(conn, Conn { side: Side::Client, host, port, open: false });
        self.send(CONNECT, conn, &frame::encode_address(host, port));
        true
    }

    fn close_side(&mut self, side: Side) {
        let closing: Vec<u16> = self.conns.iter().filter(|(_, c)| c.side == side).map(|(&id, _)| id).collect();
        for conn in closing {
            self.conns.remove(&conn);
            self.send(DISCONNECT, conn, &[]);
        }
        self.events(side).clear();
    }

    /// Every connection ends, as when ENet's peers time out.
    fn drop_all(&mut self) {
        for (conn, c) in std::mem::take(&mut self.conns) {
            self.events(c.side).push_back(Event::Disconnect(NetPeerWebSocket::new(conn, &c)));
        }
        self.outbox.clear();
    }

    fn pump(&mut self) {
        let state = unsafe { smw_ws_state() };
        if state == WS_OPEN {
            for message in std::mem::take(&mut self.outbox) {
                unsafe { smw_ws_send(message.as_ptr(), message.len()) };
            }
        }

        loop {
            let len = unsafe { smw_ws_recv(recv_buffer.as_mut_ptr(), MAX_MESSAGE_LEN) };
            if len == -1 {
                break;
            }
            if len >= 0 {
                let message = unsafe { recv_buffer[..len as usize].to_vec() };
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
                        self.events(c.side).push_back(Event::Connect(NetPeerWebSocket::new(conn, &c)));
                    }
                }
            }
            INCOMING => match frame::decode_address(payload) {
                Some((host, port)) if self.listening && conn >= INCOMING_CONN && !self.conns.contains_key(&conn) => {
                    let c = Conn { side: Side::GameHost, host, port, open: true };
                    self.conns.insert(conn, c);
                    self.gamehost_events.push_back(Event::Connect(NetPeerWebSocket::new(conn, &c)));
                }
                _ => self.send(DISCONNECT, conn, &[]),
            },
            DATA => {
                if let Some(c) = self.conns.get(&conn).copied() {
                    if c.open {
                        self.events(c.side).push_back(Event::Receive(NetPeerWebSocket::new(conn, &c), payload.to_vec()));
                    }
                }
            }
            REFUSED | DISCONNECT => {
                if let Some(c) = self.conns.remove(&conn) {
                    self.events(c.side).push_back(Event::Disconnect(NetPeerWebSocket::new(conn, &c)));
                }
            }
            _ => {}
        }
    }
}

fn listen(side: Side, handler: &mut dyn NetworkEventHandler) {
    unsafe {
        relay.pump();
        while let Some(event) = relay.events(side).pop_front() {
            match event {
                Event::Connect(peer) => handler.on_connect(Box::new(peer)),
                Event::Receive(mut peer, data) => handler.on_receive(&mut peer, &data),
                Event::Disconnect(mut peer) => handler.on_disconnect(&mut peer),
            }
        }
    }
}

pub struct NetPeerWebSocket {
    conn: u16,
    host: [u8; 4],
    port: u16,
}

impl NetPeerWebSocket {
    fn new(conn: u16, c: &Conn) -> Self {
        NetPeerWebSocket { conn, host: c.host, port: c.port }
    }
}

impl NetPeer for NetPeerWebSocket {
    fn send(&mut self, data: &[u8]) -> bool {
        unsafe {
            if data.is_empty() || !relay.conns.contains_key(&self.conn) {
                return false;
            }
            relay.send(DATA, self.conn, data);
        }
        true
    }

    fn send_reliable(&mut self, data: &[u8]) -> bool {
        self.send(data)
    }

    fn disconnect(&mut self) {
        unsafe {
            if relay.conns.remove(&self.conn).is_some() {
                relay.send(DISCONNECT, self.conn, &[]);
            }
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
    fn init(&mut self) -> bool {
        println!("[net] WebSocket relay networking initialized.");
        true
    }

    fn cleanup(&mut self) {
        self.gamehost_shutdown();
        self.client_shutdown();
        unsafe { smw_ws_close() };
    }

    fn client_restart(&mut self) -> bool {
        self.client_shutdown();
        true
    }

    fn client_listen(&mut self, handler: &mut dyn NetworkEventHandler) {
        listen(Side::Client, handler);
    }

    fn client_shutdown(&mut self) {
        unsafe {
            relay.close_side(Side::Client);
            relay.close_if_idle();
        }
    }

    fn gamehost_restart(&mut self) -> bool {
        self.gamehost_shutdown();
        unsafe {
            let state = smw_ws_state();
            if state != WS_OPEN && state != WS_CONNECTING {
                eprintln!("[error][net] Could not open game host connection port.");
                return false;
            }
            relay.send(LISTEN, 0, &NET_GAMEHOST_PORT.to_be_bytes());
            relay.listening = true;
        }
        true
    }

    fn gamehost_listen(&mut self, handler: &mut dyn NetworkEventHandler) {
        listen(Side::GameHost, handler);
    }

    fn gamehost_shutdown(&mut self) {
        unsafe {
            if relay.listening {
                relay.send(UNLISTEN, 0, &NET_GAMEHOST_PORT.to_be_bytes());
                relay.listening = false;
            }
            relay.close_side(Side::GameHost);
            relay.close_if_idle();
        }
    }

    fn connect_to_lobby_server(&mut self, hostname: &str, port: u16) -> bool {
        unsafe {
            relay.open(&url_for_server(hostname));
            relay.connect([0; 4], port)
        }
    }

    fn connect_to_foreign_game_host(&mut self, hostname: &str, port: u16) -> bool {
        let Ok(address) = hostname.parse::<Ipv4Addr>() else {
            println!("[net] Could not initiate connection to this address.");
            return false;
        };
        unsafe { relay.connect(address.octets(), port) }
    }

    /// The relay connects players; there is no NAT to open.
    fn nat_punch(&mut self, _host: u32, _port: u16) -> bool {
        true
    }
}
