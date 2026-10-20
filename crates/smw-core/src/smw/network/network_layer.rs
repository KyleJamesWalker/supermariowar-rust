//! The transport a game reaches the lobby with.

use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::game::Game;

pub trait NetworkLayer {
    fn init(&mut self, _g: &mut Game) -> bool {
        true
    }
    fn cleanup(&mut self, _g: &mut Game) {}

    /// Prepares the transport for a new session; false when it cannot.
    fn client_restart(&mut self, g: &mut Game) -> bool;
    /// Hands every pending connect, receive and disconnect event to `handler`, without blocking.
    fn client_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>);
    fn client_shutdown(&mut self, g: &mut Game);

    /// Starts connecting to the lobby at `hostname`; a later `client_listen` reports the connection. False when it cannot start.
    fn connect_to_lobby_server(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool;
}
