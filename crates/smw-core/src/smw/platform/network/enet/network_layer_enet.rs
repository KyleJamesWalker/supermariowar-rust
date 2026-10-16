//! Port of src/smw/platform/network/enet/NetworkLayerENet.cpp

use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::common_netplay::platform_enet::net_peer_enet::NetPeerENet;
use crate::common_netplay::protocol_definitions::NET_GAMEHOST_PORT;
use crate::game::Game;
use crate::globals::Aliased;
use crate::smw::network::network_layer::NetworkLayer;
use enet_sys::*;
use std::ffi::CString;
use std::ptr::null_mut;

pub struct NetworkLayerENet {
    local_client: *mut ENetHost,
    last_client_event: ENetEvent,

    local_gamehost: *mut ENetHost,
    last_gamehost_event: ENetEvent,

    _alias: Aliased,
}

impl Default for NetworkLayerENet {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkLayerENet {
    pub fn new() -> Self {
        NetworkLayerENet {
            _alias: Aliased::new(),
            local_client: null_mut(),
            last_client_event: unsafe { std::mem::zeroed() },
            local_gamehost: null_mut(),
            last_gamehost_event: unsafe { std::mem::zeroed() },
        }
    }

    fn open_connection(&mut self, hostname: &str, port: u16) -> bool {
        if self.local_client.is_null() {
            println!("[net] Could not initiate connection: no client connection port.");
            return false;
        }
        unsafe {
            let mut target_address: ENetAddress = std::mem::zeroed();
            let c = CString::new(hostname).unwrap_or_default();
            enet_address_set_host(&mut target_address, c.as_ptr());
            target_address.port = port;

            let target_host = enet_host_connect(self.local_client, &target_address, 2, 0);
            if target_host.is_null() {
                println!("[net] Could not initiate connection to this address.");
                return false;
            }
        }

        true
    }

    fn close(&mut self) {
        self.close_gamehost();
        self.close_client();
        unsafe { enet_deinitialize() };
    }

    fn close_client(&mut self) {
        if self.local_client.is_null() {
            return;
        }

        unsafe { enet_host_destroy(self.local_client) };
        self.local_client = null_mut();
    }

    fn close_gamehost(&mut self) {
        if self.local_gamehost.is_null() {
            return;
        }

        unsafe { enet_host_destroy(self.local_gamehost) };
        self.local_gamehost = null_mut();
    }

    /// Not in upstream: a host that failed to open (no socket permission on Android) is null, and ENet dereferences it.
    fn listen(g: &mut Game, host: *mut ENetHost, last_host_event: &mut ENetEvent, listener: &mut dyn NetworkEventHandler<Game>) {
        if host.is_null() {
            return;
        }
        unsafe {
            while enet_host_service(host, last_host_event, 0) > 0 {
                match last_host_event.type_ {
                    _ENetEventType_ENET_EVENT_TYPE_CONNECT => {
                        let peer = Box::new(NetPeerENet::new(last_host_event.peer));
                        listener.on_connect(g, peer);
                    }
                    _ENetEventType_ENET_EVENT_TYPE_RECEIVE => {
                        let mut peer = NetPeerENet::new(last_host_event.peer);
                        let packet = last_host_event.packet;
                        let data = std::slice::from_raw_parts((*packet).data, (*packet).dataLength);
                        listener.on_receive(g, &mut peer, data);
                        enet_packet_destroy(packet);
                    }
                    _ENetEventType_ENET_EVENT_TYPE_DISCONNECT => {
                        let mut peer = NetPeerENet::new(last_host_event.peer);
                        listener.on_disconnect(g, &mut peer);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl NetworkLayer for NetworkLayerENet {
    fn init(&mut self, _g: &mut Game) -> bool {
        unsafe {
            if enet_initialize() != 0 {
                eprintln!("[error][net] Could not initialize network system.");
                return false;
            }

            let version = enet_linked_version();
            println!("[net] ENet {}.{}.{} initialized.", (version >> 16) & 0xFF, (version >> 8) & 0xFF, version & 0xFF);
        }
        true
    }

    fn client_restart(&mut self, _g: &mut Game) -> bool {
        self.close_client();

        self.local_client = unsafe { enet_host_create(std::ptr::null(), 2, 2, 0, 0) };
        if self.local_client.is_null() {
            eprintln!("[error][net] Could not open client connection port.");
            return false;
        }
        true
    }

    fn gamehost_restart(&mut self, _g: &mut Game) -> bool {
        self.close_gamehost();

        unsafe {
            let mut local_gamehost_address: ENetAddress = std::mem::zeroed();
            local_gamehost_address.host = ENET_HOST_ANY;
            local_gamehost_address.port = NET_GAMEHOST_PORT;

            self.local_gamehost = enet_host_create(&local_gamehost_address, 6, 2, 0, 0);
        }
        if self.local_gamehost.is_null() {
            eprintln!("[error][net] Could not open game host connection port.");
            return false;
        }
        true
    }

    fn connect_to_lobby_server(&mut self, _g: &mut Game, hostname: &str, port: u16) -> bool {
        self.open_connection(hostname, port)
    }

    fn connect_to_foreign_game_host(&mut self, _g: &mut Game, hostname: &str, port: u16) -> bool {
        self.open_connection(hostname, port)
    }

    fn nat_punch(&mut self, _g: &mut Game, host: u32, port: u16) -> bool {
        if self.local_gamehost.is_null() {
            return false;
        }
        unsafe {
            let target_address = ENetAddress { host, port };

            let target_host = enet_host_connect(self.local_gamehost, &target_address, 2, 0);
            if target_host.is_null() {
                println!("[net] Could not initiate connection to a client.");
                return false;
            }
        }

        true
    }

    fn client_listen(&mut self, g: &mut Game, client: &mut dyn NetworkEventHandler<Game>) {
        Self::listen(g, self.local_client, &mut self.last_client_event, client);
    }

    fn gamehost_listen(&mut self, g: &mut Game, gamehost: &mut dyn NetworkEventHandler<Game>) {
        Self::listen(g, self.local_gamehost, &mut self.last_gamehost_event, gamehost);
    }

    fn cleanup(&mut self, _g: &mut Game) {
        self.close();
    }

    fn client_shutdown(&mut self, _g: &mut Game) {
        self.close_client();
    }

    fn gamehost_shutdown(&mut self, _g: &mut Game) {
        self.close_gamehost();
    }
}

impl Drop for NetworkLayerENet {
    fn drop(&mut self) {
        self.close();
    }
}
