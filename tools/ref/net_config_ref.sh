#!/bin/bash
# Builds net_config_ref (NetConfigManager.cpp with a stand-in net.h) against the C++ reference and its toml11.
set -e
SRC=${SMW_SRC:-$HOME/work/supermariowar-cpp-reference/src}
TOML=${SMW_TOML11:-$HOME/work/supermariowar-cpp-reference/build/_deps/toml11-src/include}
OUT=${1:-/tmp/net_config_ref}
HERE=$(cd "$(dirname "$0")" && pwd)
clang++ -std=c++20 -O1 -w -I"$HERE/net_config" -I"$SRC/common_netplay" -I"$SRC/common" -I"$SRC/smw" -I"$TOML" \
  $(sdl2-config --cflags) "$HERE/net_config_ref.cpp" "$SRC/smw/network/NetConfigManager.cpp" "$SRC/common/path.cpp" \
  $(sdl2-config --libs) -o "$OUT"
