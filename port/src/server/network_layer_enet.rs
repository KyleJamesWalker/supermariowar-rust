//! Port of src/server/NetworkLayerENet.cpp

use super::log::{log, log_silently};
use super::network_layer::NetworkLayer;
use enet_sys::*;
use smw::common_netplay::network_interface::NetworkEventHandler;
use smw::common_netplay::platform_enet::net_peer_enet::NetPeerENet;
use smw::common_netplay::protocol_definitions::NET_LOBBYSERVER_PORT;

pub struct NetworkLayerENet {
    server_host: *mut ENetHost,
    last_event: ENetEvent,
}

impl NetworkLayerENet {
    pub fn new() -> Self {
        NetworkLayerENet { server_host: std::ptr::null_mut(), last_event: unsafe { std::mem::zeroed() } }
    }
}

impl Drop for NetworkLayerENet {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl NetworkLayer for NetworkLayerENet {
    fn init(&mut self, max_players: u64) -> bool {
        unsafe {
            if enet_initialize() != 0 {
                log("[error] Could not initialize network system.");
                return false;
            }

            let version = enet_linked_version();
            log_silently(&format!("[info] ENet {}.{}.{} initialized.\n", (version >> 16) & 0xFF, (version >> 8) & 0xFF, version & 0xFF));

            let server_address = ENetAddress { host: ENET_HOST_ANY, port: NET_LOBBYSERVER_PORT };

            // bind address
            // multiple incoming connections
            // two channels
            // no up/down speed limit
            self.server_host = enet_host_create(&server_address, max_players as usize, 2, 0, 0);
            if self.server_host.is_null() {
                log("[error] Could not open connection port.");
                return false;
            }

            enet_host_compress(self.server_host, std::ptr::null());
            true
        }
    }

    fn cleanup(&mut self) {
        unsafe {
            // if network code is still active
            if !self.server_host.is_null() {
                enet_host_destroy(self.server_host);
                self.server_host = std::ptr::null_mut();

                enet_deinitialize();
            }
        }
    }

    fn listen(&mut self, server: &mut dyn NetworkEventHandler) {
        unsafe {
            while enet_host_service(self.server_host, &mut self.last_event, 0) > 0 {
                match self.last_event.type_ {
                    _ENetEventType_ENET_EVENT_TYPE_CONNECT => {
                        let newClient = Box::new(NetPeerENet::new(self.last_event.peer));
                        server.on_connect(newClient);
                    }

                    _ENetEventType_ENET_EVENT_TYPE_RECEIVE => {
                        let mut client = NetPeerENet::new(self.last_event.peer);
                        let packet = self.last_event.packet;
                        let data = std::slice::from_raw_parts((*packet).data, (*packet).dataLength);
                        server.on_receive(&mut client, data);
                        enet_packet_destroy(packet);
                    }

                    _ENetEventType_ENET_EVENT_TYPE_DISCONNECT => {
                        let mut client = NetPeerENet::new(self.last_event.peer);
                        server.on_disconnect(&mut client);
                    }

                    _ => {}
                }
            }
        }
    }
}
