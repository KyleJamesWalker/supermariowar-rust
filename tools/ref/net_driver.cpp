// Twin of examples/net_interop.rs built from the original C++ netplay code (net.cpp, ENet layer).
// Usage: net_driver <host|join> <datadir> [server] [name]   Prints `EV ...` lines.
#include <chrono>
#include <thread>
#include <string>
#include <vector>
#include "net.h"
#include "GameValues.h"
#include "GameMode.h"
#include "FileList.h"
#include "ResourceManager.h"
#include "TilesetManager.h"
#include "Game.h"
#include "gfx.h"
#include "map.h"
#include "eyecandy.h"
#include "ObjectContainer.h"
#include "Score.h"
#include "gamemodes/BonusHouse.h"
#include "gamemodes/MiniBoss.h"
#include "gamemodes/MiniBoxes.h"
#include "gamemodes/MiniPipe.h"
#include <cstdio>
#include <unistd.h>

class CPlayer;
SDL_Surface* screen = nullptr;
SDL_Surface* blitdest = nullptr;
short x_shake = 0;
short y_shake = 0;
std::vector<CPlayer*> players;
CGameMode* gamemodes[GAMEMODE_LAST];
short currentgamemode = 0;

// Globals main.cpp / GSGameplay.cpp define; net.cpp and the linked object code reference them.
CScore* score[4];
short score_cnt;
short g_iSwirlSpawnLocations[4][2][25];
CGM_Bonus* bonushousemode = NULL;
CGM_Pipe_MiniGame* pipegamemode = NULL;
CGM_Boss_MiniGame* bossgamemode = NULL;
CGM_Boxes_MiniGame* boxesgamemode = NULL;

extern std::string RootDataDirectory;
extern CGameValues game_values;
extern CMap* g_map;
extern CTilesetManager* g_tilesetmanager;
extern FiltersList* filterslist;
extern SkinList* skinlist;
extern GraphicsList* menugraphicspacklist;
extern GraphicsList* worldgraphicspacklist;
extern GraphicsList* gamegraphicspacklist;
extern CResourceManager* rm;
void create_gamemodes_for_driver();

static const unsigned FRAMES = 30;

static COutputControl pattern(unsigned seed, unsigned frame) {
    COutputControl c {};
    for (unsigned k = 0; k < 8; k++) {
        unsigned bits = (seed * 31 + frame * 7 + k * 3) % 4;
        c.keys[k].fDown = bits & 1;
        c.keys[k].fPressed = bits & 2;
    }
    return c;
}

static std::string bits(const COutputControl& c) {
    std::string s;
    for (int k = 0; k < 8; k++) { s += c.keys[k].fDown ? '1' : '0'; s += c.keys[k].fPressed ? '1' : '0'; }
    return s;
}

int main(int argc, char** argv) {
    std::string role = argv[1];
    std::string data = argv[2];
    std::string server = argc > 3 ? argv[3] : "127.0.0.1";
    bool is_host = role == "host";
    std::string name = argc > 4 ? argv[4] : (is_host ? "CppHost" : "CppJoin");

    RootDataDirectory = data;
    ensureSettingsDir();
    gfx_init(640, 480, false);
    blitdest = screen;
    rm = new CResourceManager();
    g_tilesetmanager = new CTilesetManager();
    filterslist = new FiltersList();
    skinlist = new SkinList();
    menugraphicspacklist = new GraphicsList();
    worldgraphicspacklist = new GraphicsList();
    gamegraphicspacklist = new GraphicsList();
    menugraphicspacklist->setCurrentIndex(0);
    game_values.init();
    create_gamemodes_for_driver();

    if (!net_init()) { printf("EV net_init failed\n"); return 2; }
    netplay.myPlayerName = name;
    netplay.savedServers.clear();
    netplay.savedServers.push_back(ServerAddress {server});
    netplay.selectedServerIndex = 0;
    netplay.mapfilepath = data + "/maps/0smw.map";
    netplay.newroom_name = "InteropRoom";

    if (!net_startSession() || !netplay.client.sendConnectRequestToSelectedServer()) { printf("EV connect failed\n"); return 2; }

    auto start = std::chrono::steady_clock::now();
    auto elapsed = [&] { return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count(); };
    int stage = 0;
    unsigned frame = 0;
    unsigned seen_players = 0;
    std::vector<std::string> received;
    int other = is_host ? 1 : 0;

    while (elapsed() < 20) {
        netplay.client.update();

        if (stage == 0 && netplay.connectSuccessful) {
            printf("EV connected\n");
            if (is_host) netplay.client.sendCreateRoomMessage(); else netplay.client.requestRoomList();
            stage = 1;
        } else if (stage == 1 && is_host && netplay.joinSuccessful) {
            printf("EV room created\n");
            stage = 2;
        } else if (stage == 1 && !is_host && !netplay.currentRooms.empty()) {
            printf("EV room listed: %s (%d/4)\n", netplay.currentRooms[0].name.c_str(), netplay.currentRooms[0].playerCount);
            netplay.selectedRoomIndex = 0;
            netplay.client.sendJoinRoomMessage();
            stage = 2;
        } else if (stage == 2 && netplay.currentRoom.playerCount() >= 2) {
            printf("EV room %s players: %s | %s | %s | %s me=%d host=%d\n", netplay.currentRoom.name.c_str(),
                netplay.currentRoom.playerNames[0].c_str(), netplay.currentRoom.playerNames[1].c_str(),
                netplay.currentRoom.playerNames[2].c_str(), netplay.currentRoom.playerNames[3].c_str(),
                netplay.remotePlayerNumber, netplay.hostPlayerNumber);
            seen_players = netplay.currentRoom.playerCount();
            if (is_host) {
                netplay.client.sendChatMessage("hello from host");
                netplay.client.local_gamehost.sendStartRoomMessage();
            }
            stage = 3;
        } else if (stage == 3 && netplay.gameRunning) {
            printf("EV game running, players in room %u\n", seen_players);
            stage = 4;
        } else if (stage == 4) {
            if (frame < FRAMES) {
                game_values.playerInput.outputControls[0] = pattern(is_host ? 1 : 2, frame);
                netplay.client.storeLocalInput();
                netplay.client.sendLocalInput();
                frame++;
            }
            while (!netplay.remote_input_buffer[other].empty()) {
                received.push_back(bits(netplay.remote_input_buffer[other].front().second));
                netplay.remote_input_buffer[other].pop_front();
            }
            if (frame >= FRAMES && (received.size() >= FRAMES || elapsed() > 8))
                stage = 5;
        } else if (stage == 5) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }

    std::vector<std::string> expected;
    for (unsigned f = 0; f < FRAMES; f++) expected.push_back(bits(pattern(is_host ? 2 : 1, f)));
    size_t e = 0;
    bool in_order = true;
    for (auto& r : received) {
        while (e < expected.size() && expected[e] != r) e++;
        if (e == expected.size()) { in_order = false; break; }
        e++;
    }
    bool matched = in_order && received.size() + 3 >= expected.size();
    printf("EV inputs received %zu matching %s\n", received.size(), matched ? "true" : "false");
    printf("EV final stage %d\n", stage);

    if (netplay.gameRunning) netplay.client.sendLeaveGameMessage();
    for (int i = 0; i < 20; i++) { netplay.client.update(); std::this_thread::sleep_for(std::chrono::milliseconds(5)); }
    net_close();
    fflush(stdout);
    _exit(stage == 5 && matched ? 0 : 1);
}
