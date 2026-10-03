#!/usr/bin/env bash
# Builds the C++ gfx smoke twin from the original sources (~/work/supermariowar, read only).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="${SMW_SRC:-$HOME/work/supermariowar/src}/common"
out="${1:-${TMPDIR:-/tmp}/gfx_smoke_cpp}"
clang++ -std=c++20 -O1 -I"$src" $(sdl2-config --cflags) \
    "$here/gfx_smoke.cpp" "$src/gfx.cpp" "$src/gfx/gfxSprite.cpp" "$src/gfx/gfxFont.cpp" \
    "$src/gfx/SFont.cpp" "$src/gfx/gfxPalette.cpp" "$src/gfx/gfxSDL.cpp" "$src/util/SdlHelpers.cpp" "$src/path.cpp" \
    $(sdl2-config --libs) -lSDL2_image -framework CoreFoundation -o "$out"
