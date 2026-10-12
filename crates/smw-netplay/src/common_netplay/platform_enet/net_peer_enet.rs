//! Port of src/common_netplay/platform_enet/NetPeerENet.cpp

use crate::globals::Aliased;
use crate::common_netplay::network_interface::NetPeer;
use enet_sys::*;
use std::any::Any;

const UNRELIABLE_CHANNEL: u8 = 0;
const RELIABLE_CHANNEL: u8 = 1;

/// Wraps a peer slot owned by the ENet host; never frees it.
pub struct NetPeerENet {
    foreign_peer: *mut ENetPeer,
    player_id: u64,
    _alias: Aliased,
}

impl NetPeerENet {
    pub fn new(peer: *mut ENetPeer) -> Self {
        unsafe {
            let mut player_id: u64 = (*peer).address.host as u64;
            player_id <<= std::mem::size_of_val(&(*peer).address.port) * 8;
            player_id += (*peer).address.port as u64;
            NetPeerENet { _alias: Aliased::new(), foreign_peer: peer, player_id }
        }
    }

    pub fn raw(&self) -> *mut ENetPeer {
        self.foreign_peer
    }

    fn send_with(&mut self, data: &[u8], flags: u32, channel: u8) -> bool {
        if self.foreign_peer.is_null() || data.is_empty() {
            return false;
        }

        unsafe {
            let packet = enet_packet_create(data.as_ptr() as *const _, data.len(), flags);
            if packet.is_null() {
                return false;
            }

            if enet_peer_send(self.foreign_peer, channel, packet) < 0 {
                return false;
            }
        }

        true
    }
}

impl NetPeer for NetPeerENet {
    fn send(&mut self, data: &[u8]) -> bool {
        self.send_with(data, 0, UNRELIABLE_CHANNEL)
    }

    fn send_reliable(&mut self, data: &[u8]) -> bool {
        self.send_with(data, _ENetPacketFlag_ENET_PACKET_FLAG_RELIABLE as u32, RELIABLE_CHANNEL)
    }

    fn disconnect(&mut self) {
        if !self.foreign_peer.is_null() {
            unsafe { enet_peer_disconnect_now(self.foreign_peer, 0) };
        }
    }

    fn address_host(&self) -> u32 {
        if !self.foreign_peer.is_null() {
            return unsafe { (*self.foreign_peer).address.host };
        }
        0
    }

    fn address_port(&self) -> u16 {
        if !self.foreign_peer.is_null() {
            return unsafe { (*self.foreign_peer).address.port };
        }
        0
    }

    fn address_as_string(&self) -> String {
        unsafe {
            let addr = (*self.foreign_peer).address.host.to_ne_bytes();
            format!("{}.{}.{}.{}:{}", addr[0], addr[1], addr[2], addr[3], (*self.foreign_peer).address.port)
        }
    }

    fn average_rtt(&self) -> u32 {
        unsafe { (*self.foreign_peer).roundTripTime }
    }

    fn get_player_id(&self) -> u64 {
        self.player_id
    }

    fn peer_key(&self) -> u64 {
        self.foreign_peer as u64
    }

    fn same_peer(&self, other: &dyn NetPeer) -> bool {
        match other.as_any().downcast_ref::<NetPeerENet>() {
            Some(o) => self.foreign_peer == o.foreign_peer,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
