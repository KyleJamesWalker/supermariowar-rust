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
| common/GlobalConstants.h, Version.h, MatchTypes.h, GameplayStyles.h, PlayerKill*.h, MovingObjectTypes.h, EyecandyStyles.h, Score.h | common/global_constants.rs etc. | done | n/a |
| common/RandomNumberGenerator.cpp | common/random_number_generator.rs | done | unit test vs C++ outputs (`tools/ref/rng_ref.cpp`) |
| common/FileIO.cpp, linfunc.cpp, path.cpp, util/DirIterator.cpp, util/Grid.h, util/ContainerHelpers.h, math/Vec2.h | same names | done | unit tests |
| common/FileList.cpp, GameModeSettings.cpp | common/file_list.rs, game_mode_settings.rs | done | unit tests on data/; GameModeSettings raw image matches clang layout (`tools/ref/layout_ref.cpp`) |
| common/TileTypes.cpp | common/tile_types.rs | done | unit test |
| common/map.cpp, map/MapReader*.cpp | common/map.rs, common/map/* | done (drawing ported, untested on screen) | all 504 maps x 3 read types vs C++ (`tools/ref/map_dump.sh`, `SMW_MAP_DUMP=... cargo test matches_cpp_map_dump`) |
| common/movingplatform.cpp, MovingPlatformPaths.cpp | common/movingplatform.rs, moving_platform_paths.rs | done | platform paths covered by the map dump (120 frames); collisions untested until CPlayer lands |
| common/TilesetManager.cpp | common/tileset_manager.rs | done | via map dump |
| common/MapList.cpp | common/map_list.rs | done | not yet |
| common/WorldTourStop.cpp | common/world_tour_stop.rs | done | strtok unit test |
| common/GameValues.cpp | common/game_values.rs | done except full options.bin IO (handed to gfx) | not yet |
| common/ObjectBase.cpp, IO_Block.h, GameMode.h, Game.cpp | common/object_base.rs, io_block.rs, game_mode.rs, game.rs | done (headers + base structs/traits) | n/a |
| smw/objects/blocks/IO_Block.cpp | smw/objects/blocks/io_block.rs | partial: all but BounceMovingObject, KillPlayersAndObjectsInsideBlock | |
| smw/player.cpp | smw/player.rs | done except netplay branches | start_classic reaches gameplay; P lines not comparable yet (RNG diverges in the menu at frame 30) |
| smw/player_components/* | smw/player_components/*.rs | done | as above |
| smw/ai.cpp | smw/ai.rs | done | frenzy_cpu: not comparable yet (same menu RNG divergence) |
| objects/moving/MovingObject.cpp, objectgame.cpp | | placeholders (foundation) awaiting their porters | |
| smw/GSGameplay.cpp | smw/gs_gameplay.rs | done except netplay (TODO(net)) and LoadMapHazards call (waits on objecthazard) | start_classic: not yet run |
| smw/gamemodes/* (GameMode, GameModeTimer, all 22 modes, bonus house, 3 minigames) | smw/gamemodes/*.rs | done except netplay disconnect text | via start_classic / frenzy_cpu, not yet run |
| smw/main.cpp create_gamemodes, mode globals | smw/main.rs | done | |
