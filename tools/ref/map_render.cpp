// Renders maps the way GameplayState draws them, for the Rust pixel-parity test.
// Usage: map_render <datadir> <outdir> <mapfile>...  Writes <outdir>/<n>_{f0,f40,thumb}.bmp per map.
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
#include "movingplatform.h"
#include "GameValues.h"
#include "FileList.h"
#include "TilesetManager.h"
#include "ResourceManager.h"
#include "path.h"
#include "gfx.h"
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
extern SkinList* skinlist;
extern GraphicsList* menugraphicspacklist;
extern GraphicsList* worldgraphicspacklist;
extern GraphicsList* gamegraphicspacklist;
extern CResourceManager* rm;
extern short g_iCurrentDrawIndex;
void LoadCurrentMapBackground();

static void drawMap() {
    blitdest = screen;
    rm->spr_backmap[g_iCurrentDrawIndex].draw(0, 0);
    g_map->drawPlatforms(0);
    g_map->drawPlatforms(1);
    g_map->drawPlatforms(2);
    if (game_values.toplayer)
        g_map->drawfrontlayer();
    g_map->drawWarpLocks();
    g_map->drawPlatforms(3);
    g_map->drawPlatforms(4);
}

int main(int argc, char** argv) {
    RootDataDirectory = argv[1];
    std::string outdir = argv[2];
    gfx_init(640, 480, false);
    blitdest = screen;
    rm = new CResourceManager();
    g_map = new (calloc(1, sizeof(CMap))) CMap();
    filterslist = new FiltersList();
    skinlist = new SkinList();
    menugraphicspacklist = new GraphicsList();
    worldgraphicspacklist = new GraphicsList();
    gamegraphicspacklist = new GraphicsList();
    game_values.init();
    menugraphicspacklist->setCurrentIndex(0);
    worldgraphicspacklist->setCurrentIndex(0);
    gamegraphicspacklist->setCurrentIndex(0);
    rm->loadAllGraphics();

    for (int i = 3; i < argc; i++) {
        int n = i - 3;
        g_map->loadMap(argv[i], read_type_full);
        LoadCurrentMapBackground();
        g_map->predrawbackground(rm->spr_background, rm->spr_backmap[0]);
        g_map->predrawforeground(rm->spr_frontmap[0]);
        g_map->predrawbackground(rm->spr_background, rm->spr_backmap[1]);
        g_map->predrawforeground(rm->spr_frontmap[1]);
        g_map->SetupAnimatedTiles();

        SDL_FillRect(screen, NULL, 0);
        drawMap();
        SDL_SaveBMP(screen, (outdir + "/" + std::to_string(n) + "_f0.bmp").c_str());

        for (int f = 0; f < 40; f++) {
            g_map->updatePlatforms();
            g_map->update();
        }
        g_map->lockconnection(0);
        SDL_FillRect(screen, NULL, 0);
        drawMap();
        SDL_SaveBMP(screen, (outdir + "/" + std::to_string(n) + "_f40.bmp").c_str());

        SDL_Surface* thumb = g_map->createThumbnailSurface(false);
        if (thumb) {
            SDL_SaveBMP(thumb, (outdir + "/" + std::to_string(n) + "_thumb.bmp").c_str());
            SDL_FreeSurface(thumb);
        }
    }
    printf("\n---\nrendered %d\n", argc - 3);
    return 0;
}
