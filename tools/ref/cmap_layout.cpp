// Prints the CMap member offsets that smw/ai.rs uses to reproduce out-of-bounds g_map reads.
// Build from ~/work/smw-ref/src: clang++ -std=c++17 -Wno-invalid-offsetof -I common -I smw -I . -I/opt/homebrew/include -I/opt/homebrew/include/SDL2 cmap_layout.cpp
#define private public
#define protected public
#include "map.h"
#undef private
#undef protected
#include <cstdio>
#include <cstddef>
int main() {
    printf("sizeof CMap %zu MapBlock %zu TilesetTile %zu Warp %zu\n", sizeof(CMap), sizeof(MapBlock), sizeof(TilesetTile), sizeof(Warp));
#define OFF(f) printf("%s %zu\n", #f, offsetof(CMap, f));
    OFF(mapdata) OFF(mapdatatop) OFF(objectdata) OFF(blockdata) OFF(nospawn) OFF(warpdata)
}
