//! Native builds' network layer: ENet for a server saved as a host name, the relay layer for a `ws://` or `wss://`
//! URL, which puts native and browser players in the same lobby.

use super::enet::network_layer_enet::NetworkLayerENet;
use super::websocket::network_layer_websocket::NetworkLayerWebSocket;
use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::game::Game;
use crate::smw::network::network_layer::NetworkLayer;

#[derive(Default)]
pub struct NetworkLayerDual {
    enet: NetworkLayerENet,
    relay: NetworkLayerWebSocket,
    use_relay: bool,
}

impl NetworkLayerDual {
    fn active(&mut self) -> &mut dyn NetworkLayer {
        if self.use_relay {
            &mut self.relay
        } else {
            &mut self.enet
        }
    }
}

impl NetworkLayer for NetworkLayerDual {
    fn init(&mut self, g: &mut Game) -> bool {
        self.enet.init(g) && self.relay.init(g)
    }

    fn cleanup(&mut self, g: &mut Game) {
        self.enet.cleanup(g);
        self.relay.cleanup(g);
    }

    fn client_restart(&mut self, g: &mut Game) -> bool {
        self.relay.client_restart(g) && self.enet.client_restart(g)
    }

    fn client_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>) {
        self.active().client_listen(g, handler);
    }

    fn client_shutdown(&mut self, g: &mut Game) {
        self.enet.client_shutdown(g);
        self.relay.client_shutdown(g);
    }

    fn gamehost_restart(&mut self, g: &mut Game) -> bool {
        self.active().gamehost_restart(g)
    }

    fn gamehost_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>) {
        self.active().gamehost_listen(g, handler);
    }

    fn gamehost_shutdown(&mut self, g: &mut Game) {
        self.enet.gamehost_shutdown(g);
        self.relay.gamehost_shutdown(g);
    }

    fn connect_to_lobby_server(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        self.use_relay = hostname.starts_with("ws://") || hostname.starts_with("wss://");
        self.active().connect_to_lobby_server(g, hostname, port)
    }

    fn connect_to_foreign_game_host(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        self.active().connect_to_foreign_game_host(g, hostname, port)
    }

    fn nat_punch(&mut self, g: &mut Game, host: u32, port: u16) -> bool {
        self.active().nat_punch(g, host, port)
    }
}
