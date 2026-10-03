// Dumps CMap state after loadMap for the Rust map-loading parity test.
// Build: see map_dump.sh. Usage: map_dump <datadir> <mapfile> <readtype>
#include <array>
#include <list>
#include <string>
#include <vector>
#include <memory>
#include <map>
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
#include <vector>

class CPlayer;

class CScore;
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
extern CResourceManager* rm;

struct MapDumper {
    static void dump(CMap& m, int readtype) {
        printf("bg=%s bgid=%d ec=%d,%d,%d music=%d races=%d flags=%d\n", m.szBackgroundFile.c_str(), m.backgroundID,
               m.eyecandy[0], m.eyecandy[1], m.eyecandy[2], m.musicCategoryID, m.iNumRaceGoals, m.iNumFlagBases);
        printf("filters");
        for (bool f : m.fAutoFilter) printf(" %d", f);
        printf("\n");
        if (readtype == 2) return;
        for (int j = 0; j < MAPHEIGHT; j++)
            for (int i = 0; i < MAPWIDTH; i++) {
                printf("t %d %d", i, j);
                for (int k = 0; k < MAPLAYERS; k++)
                    printf(" %d/%d/%d", m.mapdata[i][j][k].iID, m.mapdata[i][j][k].iCol, m.mapdata[i][j][k].iRow);
                printf(" top=%d obj=%d hid=%d", (int)m.mapdatatop[i][j], m.objectdata[i][j].iType, m.objectdata[i][j].fHidden);
                if (m.objectdata[i][j].iType == 1 || m.objectdata[i][j].iType == 15 || (m.objectdata[i][j].iType >= 11 && m.objectdata[i][j].iType <= 14)) {
                    printf(" set");
                    for (int s = 0; s < NUM_BLOCK_SETTINGS; s++) printf(" %d", m.objectdata[i][j].iSettings[s]);
                }
                printf(" warp=%d/%d/%d ns", m.warpdata[i][j].direction, m.warpdata[i][j].connection, m.warpdata[i][j].id);
                for (int t = 0; t < NUMSPAWNAREATYPES; t++) printf("%d", m.nospawn[t][i][j]);
                printf("\n");
            }
        printf("switches %d %d %d %d\n", m.iSwitches[0], m.iSwitches[1], m.iSwitches[2], m.iSwitches[3]);
        for (const MapItem& it : m.mapitems) printf("item %d %d %d\n", it.itype, it.ix, it.iy);
        for (const MapHazard& h : m.maphazards) {
            printf("hazard %d %d %d", h.itype, h.ix, h.iy);
            for (int p = 0; p < NUMMAPHAZARDPARAMS; p++) printf(" %d", h.iparam[p]);
            for (int p = 0; p < NUMMAPHAZARDPARAMS; p++) printf(" %.6f", h.dparam[p]);
            printf("\n");
        }
        for (MovingPlatform* p : m.platforms) {
            printf("platform %d %d layer=%d path=%d x=%.6f y=%.6f vx=%.6f vy=%.6f\n", p->iTileWidth, p->iTileHeight, p->iDrawLayer,
                   (int)p->pPath->typeId(), p->fx, p->fy, p->fVelX, p->fVelY);
            for (size_t i = 0; i < p->iTileData.size(); i++)
                printf(" %d/%d/%d:%d", p->iTileData[i].iID, p->iTileData[i].iCol, p->iTileData[i].iRow, (int)p->iTileType[i]);
            printf("\n");
            for (int f = 0; f < 120; f++) p->update();
            printf(" after120 x=%.6f y=%.6f vx=%.6f vy=%.6f s1=%.6f,%.6f\n", p->fx, p->fy, p->fVelX, p->fVelY, p->pPath->currentPos1().x, p->pPath->currentPos1().y);
        }
        if (readtype == 1) return;
        printf("warpexits %d maxconn %d\n", m.numwarpexits, m.maxConnection);
        for (int w = 0; w < m.numwarpexits; w++) {
            const WarpExit& e = m.warpexits[w];
            printf("we %d %d %d %d %d %d %d %d %d %d\n", e.direction, e.connection, e.id, e.x, e.y, e.lockx, e.locky, e.warpx, e.warpy, e.numblocks);
        }
        for (int t = 0; t < NUMSPAWNAREATYPES; t++) {
            printf("spawn %d n=%d total=%d", t, m.numspawnareas[t], m.totalspawnsize[t]);
            for (int a = 0; a < m.numspawnareas[t]; a++)
                printf(" [%d %d %d %d %d]", m.spawnareas[t][a].left, m.spawnareas[t][a].top, m.spawnareas[t][a].width, m.spawnareas[t][a].height, m.spawnareas[t][a].size);
            printf("\n");
        }
        printf("drawareas %d", m.numdrawareas);
        for (int d = 0; d < m.numdrawareas; d++) printf(" [%d %d %d %d]", m.drawareas[d].x, m.drawareas[d].y, m.drawareas[d].w, m.drawareas[d].h);
        printf("\n");
        for (int r = 0; r < m.iNumRaceGoals; r++) printf("race %d %d\n", m.racegoallocations[r].x, m.racegoallocations[r].y);
        for (int f = 0; f < m.iNumFlagBases; f++) printf("flagbase %d %d\n", m.flagbaselocations[f].x, m.flagbaselocations[f].y);
        m.UpdateAllTileGaps();
        printf("gaps");
        for (int j = 0; j < MAPHEIGHT; j++)
            for (int i = 0; i < MAPWIDTH; i++)
                if (m.mapdatatop[i][j] == TileType::Gap) printf(" %d,%d", i, j);
        printf("\n");
    }
};

int main(int argc, char** argv) {
    RootDataDirectory = argv[1];
    SDL_Init(0);
    screen = SDL_CreateRGBSurface(0, 640, 480, 32, 0, 0, 0, 0);
    blitdest = screen;
    filterslist = new FiltersList();
    game_values.init();
    rm = new CResourceManager();
    g_tilesetmanager = new CTilesetManager(convertPath("gfx/packs/Classic"));
    // Zeroed so members CMap leaves default-initialized compare equal to the port's zeroed CMap.
    g_map = new (calloc(1, sizeof(CMap))) CMap();
    int readtype = atoi(argv[3]);
    g_map->loadMap(argv[2], (ReadType)readtype);
    printf("\n---\n");
    MapDumper::dump(*g_map, readtype);
    return 0;
}
