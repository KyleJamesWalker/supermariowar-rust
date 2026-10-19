//! The browser side: web/peer_socket.js owns the WebSocket and the RTCPeerConnections; this polls it.

use crate::{IceConfig, PeerId, PeerState, UNRELIABLE};
use std::ffi::CString;
use std::os::raw::c_char;

extern "C" {
    fn smw_peer_open(url: *const c_char, ice_servers_json: *const c_char, relay_only: i32);
    fn smw_peer_signaling_closed() -> i32;
    fn smw_peer_id(id: *mut u8) -> i32;
    fn smw_peer_change(id: *mut u8) -> i32;
    fn smw_peer_send(channel: i32, id: *const u8, data: *const u8, len: usize) -> i32;
    fn smw_peer_recv(channel: i32, id: *mut u8, buf: *mut u8, cap: usize) -> i32;
    fn smw_peer_close();
}

const MAX_PACKET: usize = 64 * 1024;

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn ice_servers_json(ice: &IceConfig) -> String {
    let servers: Vec<String> = ice
        .servers
        .iter()
        .map(|s| {
            let urls: Vec<String> = s.urls.iter().map(|u| json_string(u)).collect();
            format!(
                "{{\"urls\":[{}],\"username\":{},\"credential\":{}}}",
                urls.join(","),
                json_string(&s.username),
                json_string(&s.credential)
            )
        })
        .collect();
    format!("[{}]", servers.join(","))
}

fn receive(channel: usize) -> Vec<(PeerId, Vec<u8>)> {
    let mut packets = Vec::new();
    let mut buf = vec![0u8; MAX_PACKET];
    loop {
        let mut id = [0u8; 16];
        let len =
            unsafe { smw_peer_recv(channel as i32, id.as_mut_ptr(), buf.as_mut_ptr(), buf.len()) };
        match len {
            -1 => return packets,
            n if n >= 0 => {
                packets.push((PeerId(u128::from_be_bytes(id)), buf[..n as usize].to_vec()))
            }
            _ => {}
        }
    }
}

fn send(channel: usize, peer: PeerId, data: &[u8]) {
    let id = peer.0.to_be_bytes();
    unsafe { smw_peer_send(channel as i32, id.as_ptr(), data.as_ptr(), data.len()) };
}

pub struct Socket {
    unreliable_taken: bool,
}

impl Socket {
    pub fn connect(room_url: &str, ice: &IceConfig) -> Socket {
        let url = CString::new(room_url).unwrap_or_default();
        let servers = CString::new(ice_servers_json(ice)).unwrap_or_default();
        unsafe { smw_peer_open(url.as_ptr(), servers.as_ptr(), ice.relay_only as i32) };
        Socket {
            unreliable_taken: false,
        }
    }

    pub fn id(&mut self) -> Option<PeerId> {
        let mut id = [0u8; 16];
        (unsafe { smw_peer_id(id.as_mut_ptr()) } == 1).then(|| PeerId(u128::from_be_bytes(id)))
    }

    pub fn update_peers(&mut self) -> Vec<(PeerId, PeerState)> {
        let mut changes = Vec::new();
        loop {
            let mut id = [0u8; 16];
            let state = match unsafe { smw_peer_change(id.as_mut_ptr()) } {
                1 => PeerState::Connected,
                0 => PeerState::Disconnected,
                _ => return changes,
            };
            changes.push((PeerId(u128::from_be_bytes(id)), state));
        }
    }

    pub fn signaling_closed(&self) -> bool {
        unsafe { smw_peer_signaling_closed() == 1 }
    }

    pub fn send(&mut self, channel: usize, peer: PeerId, data: &[u8]) {
        send(channel, peer, data);
    }

    pub fn receive(&mut self, channel: usize) -> Vec<(PeerId, Vec<u8>)> {
        if channel == UNRELIABLE && self.unreliable_taken {
            return Vec::new();
        }
        receive(channel)
    }

    pub fn take_unreliable(&mut self) -> Channel {
        self.unreliable_taken = true;
        Channel
    }

    pub fn close(&mut self) {
        unsafe { smw_peer_close() };
    }
}

pub struct Channel;

impl Channel {
    pub fn send(&mut self, peer: PeerId, packet: &[u8]) {
        send(UNRELIABLE, peer, packet);
    }

    pub fn receive(&mut self) -> Vec<(PeerId, Vec<u8>)> {
        receive(UNRELIABLE)
    }
}
