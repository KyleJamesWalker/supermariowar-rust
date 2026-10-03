// Dumps MapList contents and navigation results for the Rust parity test. Usage: map_list_dump <datadir>
#include <array>
#include <list>
#include <string>
#include <vector>
#include <map>
#include <memory>
#include <filesystem>
#include <optional>
#include <new>
#include <cstdlib>
#define private public
#define protected public
#include "map.h"
#include "MapList.h"
#include "GameValues.h"
#include "FileList.h"
#include "TilesetManager.h"
#include "ResourceManager.h"
#include "RandomNumberGenerator.h"
#include "path.h"
#include "SDL.h"
#include "GameMode.h"
#include <cstdio>

class CPlayer;
SDL_Surface* screen = nullptr;
SDL_Surface* blitdest = nullptr;
short x_shake = 0;
short y_shake = 0;
std::vector<CPlayer*> players;
CGameMode* gamemodes[GAMEMODE_LAST];
void musicfinished() {}

extern std::string RootDataDirectory;
extern CGameValues game_values;
extern CMap* g_map;
extern CTilesetManager* g_tilesetmanager;
extern FiltersList* filterslist;
extern MapList* maplist;
extern CResourceManager* rm;

static std::string rel(const std::string& p) {
    return p.size() > RootDataDirectory.size() ? p.substr(RootDataDirectory.size()) : p;
}

static void cur(const char* what) {
    printf("%s cur=%s file=%s\n", what, maplist->currentShortmapname().c_str(), rel(maplist->currentFilename()).c_str());
}

int main(int argc, char** argv) {
    RootDataDirectory = argv[1];
    SDL_Init(0);
    screen = SDL_CreateRGBSurface(0, 640, 480, 32, 0, 0, 0, 0);
    blitdest = screen;
    filterslist = new FiltersList();
    game_values.init();
    rm = new CResourceManager();
    g_tilesetmanager = new CTilesetManager(convertPath("gfx/packs/Classic"));
    g_map = new (calloc(1, sizeof(CMap))) CMap();
    maplist = new MapList(false);
    maplist->ReadFilters();
    printf("\n---\n");

    printf("count=%zu filtered=%zu\n", maplist->count(), maplist->filteredCount());
    for (auto& [key, node] : maplist->maps) {
        printf("map %s %s idx=%d f=", key.c_str(), rel(node.filename).c_str(), node.iIndex);
        for (bool b : node.pfFilters) printf("%d", b);
        printf("\n");
    }
    for (auto& [key, node] : maplist->worldmaps) printf("world %s %s\n", key.c_str(), rel(node.filename).c_str());
    for (size_t i = 0; i < game_values.piFilterIcons.size(); i++) printf("icon %zu %d\n", i, game_values.piFilterIcons[i]);

    RandomNumberGenerator::generator().reseed(42);
    for (int i = 0; i < 3; i++) { maplist->next(false); cur("next"); }
    maplist->prev(false); cur("prev");
    for (int i = 0; i < 5; i++) { maplist->random(false); cur("random"); }
    printf("findexact %d\n", maplist->findexact("death valley", false)); cur("after findexact");
    printf("findexact %d\n", maplist->findexact("no such map", false)); cur("after miss");
    printf("findworld %d\n", maplist->findexact("special_bonushouse", true)); cur("after findworld");
    printf("find %d\n", maplist->find("matsy")); cur("after find");
    printf("startswith %d\n", maplist->startswith('m')); cur("after startswith m");
    printf("startswithstr %d\n", maplist->startswith(std::string("bo"))); cur("after startswith bo");
    for (size_t f = 0; f < NUM_AUTO_FILTERS; f++) {
        std::vector<bool> filters(NUM_AUTO_FILTERS + filterslist->count(), false);
        filters[f] = true;
        maplist->ApplyFilters(filters);
        printf("filter %zu filtered=%zu on=%d", f, maplist->filteredCount(), game_values.fFiltersOn);
        cur("");
        for (int i = 0; i < 4; i++) { maplist->next(true); cur(" next"); }
        maplist->prev(true); cur(" prev");
        maplist->random(true); cur(" random");
        auto it = maplist->GetIteratorAt(1, true);
        printf(" at1 %s\n", it == maplist->maps.end() ? "end" : it->first.c_str());
    }
    std::vector<bool> none(NUM_AUTO_FILTERS + filterslist->count(), false);
    maplist->ApplyFilters(none);
    for (int i = 0; i < 5; i++) printf("randomFilename %s\n", rel(maplist->randomFilename()).c_str());
    return 0;
}
