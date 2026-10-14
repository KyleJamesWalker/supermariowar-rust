#!/bin/bash
# Builds map_render against the original C++ sources. Unused game symbols resolve lazily (-undefined dynamic_lookup).
set -e
SRC=${SMW_SRC:-$HOME/work/supermariowar-cpp-reference/src}
OUT=${1:-/tmp/map_render}
C=$SRC/common
OBJ=$(mktemp -d)
trap 'rm -rf "$OBJ"' EXIT
for f in atan2f cosf sinf; do
  clang -O2 -ffp-contract=off -w -c $C/core-math/$f.c -o $OBJ/$f.o
done
clang++ -std=c++20 -O2 -DNDEBUG -DSMW_NO_RLE -ffp-contract=off -w -I$C -I$SRC/smw $(sdl2-config --cflags) \
  $(dirname $0)/map_render.cpp $C/map.cpp $C/map/*.cpp $C/movingplatform.cpp $C/MovingPlatformPaths.cpp $C/TilesetManager.cpp \
  $C/FileIO.cpp $C/FileList.cpp $C/path.cpp $C/linfunc.cpp $C/util/DirIterator.cpp $C/RandomNumberGenerator.cpp \
  $C/GameModeSettings.cpp $C/GameValues.cpp $C/input.cpp $C/global.cpp $C/gfx.cpp $C/gfx/*.cpp $C/ResourceManager.cpp $C/sfx.cpp \
  $C/util/SdlHelpers.cpp $OBJ/*.o \
  $(sdl2-config --libs) -lSDL2_image -lSDL2_mixer -Wl,-undefined,dynamic_lookup -o $OUT
