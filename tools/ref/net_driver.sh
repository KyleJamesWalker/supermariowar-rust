#!/bin/bash
# Builds net_driver from the original C++ netplay sources (ENet from Homebrew). Unused game symbols resolve lazily.
set -e
SRC=${SMW_SRC:-$HOME/work/supermariowar-cpp-reference/src}
TOML=${SMW_TOML11:-$HOME/work/supermariowar-cpp-reference/build/_deps/toml11-src/include}
OUT=${1:-/tmp/net_driver}
C=$SRC/common
M=$SRC/smw
clang++ -std=c++20 -O1 -w -Wno-c++11-narrowing -I$C -I$SRC/common_netplay -I$M -I$TOML -I/opt/homebrew/include $(sdl2-config --cflags) \
  $(dirname $0)/net_driver.cpp $(dirname $0)/net_driver_modes.cpp \
  $M/net.cpp $M/platform/network/enet/NetworkLayerENet.cpp $SRC/common_netplay/platform_enet/NetPeerENet.cpp \
  $M/network/FileCompressor.cpp $M/network/NetConfigManager.cpp $M/gamemodes/*.cpp \
  $(find $M/objects -name '*.cpp') $M/objectgame.cpp $M/objecthazard.cpp $M/ObjectContainer.cpp $M/GSGameplay.cpp $M/player.cpp $M/player_components/*.cpp $M/ai.cpp $C/eyecandy.cpp $C/map.cpp $C/map/*.cpp $C/movingplatform.cpp $C/MovingPlatformPaths.cpp $C/ObjectBase.cpp \
  $C/FileIO.cpp $C/FileList.cpp $C/path.cpp $C/linfunc.cpp $C/util/DirIterator.cpp $C/RandomNumberGenerator.cpp $C/Game.cpp \
  $C/GameModeSettings.cpp $C/GameValues.cpp $C/input.cpp $C/global.cpp $C/gfx.cpp $C/gfx/*.cpp $C/ResourceManager.cpp $C/sfx.cpp \
  $C/util/SdlHelpers.cpp $C/TilesetManager.cpp \
  $(sdl2-config --libs) -L/opt/homebrew/lib -lSDL2_image -lSDL2_mixer -lenet -lz -Wl,-undefined,dynamic_lookup -o $OUT
