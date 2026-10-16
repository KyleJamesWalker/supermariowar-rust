//! Port of src/smw/network/NetworkLayer.h

use crate::common_netplay::network_interface::NetworkEventHandler;
use crate::game::Game;

pub trait NetworkLayer {
    fn init(&mut self, _g: &mut Game) -> bool {
        true
    }
    fn cleanup(&mut self, _g: &mut Game) {}

    fn client_restart(&mut self, g: &mut Game) -> bool;
    fn client_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>);
    fn client_shutdown(&mut self, g: &mut Game);

    fn gamehost_restart(&mut self, g: &mut Game) -> bool;
    fn gamehost_listen(&mut self, g: &mut Game, handler: &mut dyn NetworkEventHandler<Game>);
    fn gamehost_shutdown(&mut self, g: &mut Game);

    fn connect_to_lobby_server(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool;
    fn connect_to_foreign_game_host(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool;
    fn nat_punch(&mut self, g: &mut Game, host: u32, port: u16) -> bool;
}
