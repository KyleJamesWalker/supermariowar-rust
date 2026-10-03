// Twin of NetConfigManager load + save: reads $HOME/.../servers.toml with the C++ code, then writes it back.
// Build: see net_config_ref.sh. The Rust side is `smw::network::net_config_manager` (same file in the game).
#include "net.h"
#include "network/NetConfigManager.h"

Networking netplay;
std::string RootDataDirectory;

int main()
{
    netplay.myPlayerName = "Player";
    NetConfigManager().load();
    NetConfigManager().save();
}
