// Stand-in for smw/net.h: only what NetConfigManager.cpp uses, so it builds without the network stack.
#pragma once
#include "ProtocolDefinitions.h"
#include <string>
#include <vector>

struct ServerAddress {
    std::string hostname;
};

struct Networking {
    std::string myPlayerName;
    std::vector<ServerAddress> savedServers;
};

extern Networking netplay;
