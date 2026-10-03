# Rust port progress

Faithful port of `~/work/supermariowar` (C++/SDL2) to Rust + rust-sdl2. Logic mirrors the C++ file by file; parity is checked by differential replay (see `REPLAY.md`).

Scope order: core game + menus + sound, then netplay, level editor, world editor.

## Module map

| C++ | Rust | Status | Parity |
|---|---|---|---|
| (filled in as modules land) | | | |
| common/gfx.cpp, gfx/gfxSprite, gfxFont, SFont, gfxPalette, gfxSDL, Color.h | common/gfx.rs, common/gfx/* | done | `tools/gfx_smoke/compare.sh`: BMP byte-identical to C++ |
| common/util/SdlHelpers.cpp | common/util/sdl_helpers.rs | done | n/a |
| common/sfx.cpp | common/sfx.rs | done | not tested (no audio in harness) |
| common/input.cpp | common/input.rs | partial: types + key clearing; `update()` waits on game_values | |
| common/ResourceManager.cpp | common/resource_manager.rs | written, waits on game_values / tileset_manager / game.rs | |
| common/eyecandy.cpp | common/eyecandy.rs | written, waits on map / io_block / gs_gameplay `eyecandy` | |
