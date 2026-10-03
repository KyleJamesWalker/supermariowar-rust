# Rust port progress

Faithful port of `~/work/supermariowar` (C++/SDL2) to Rust + rust-sdl2. Logic mirrors the C++ file by file; parity is checked by differential replay (see `REPLAY.md`).

Scope order: core game + menus + sound, then netplay, level editor, world editor.

## Module map

| C++ | Rust | Status | Parity |
|---|---|---|---|
| (filled in as modules land) | | | |
| common/gfx.cpp, gfx/gfxSprite, gfxFont, gfxPalette, gfxSDL, Color.h | common/gfx.rs, common/gfx/* | done (upstream `5693918f`: SFont is gone) | `tools/gfx_smoke/compare.sh`: BMP byte-identical to C++ |
| common/util/SdlHelpers.cpp | common/util/sdl_helpers.rs | done | n/a |
| common/sfx.cpp | common/sfx.rs | done | `S` records (virtual mixer) match C++ on every replay |
| common/input.cpp | common/input.rs | done | replays (keyboard path) |
| common/ResourceManager.cpp | common/resource_manager.rs | done | `examples/rm_smoke.rs` loads every asset; replay screenshots match |
| common/eyecandy.cpp | common/eyecandy.rs | done | `O ec*` counts match on replays |
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
| smw/player.cpp | smw/player.rs | done except netplay branches | start_classic, map_blockpiles, frenzy_cpu: all frames identical, all shots match |
| smw/player_components/* | smw/player_components/*.rs | done | via the replays above |
| smw/ai.cpp | smw/ai.rs | done | frenzy_cpu: all 2780 frames identical, including RNG call counts |
| smw/GSGameplay.cpp | smw/gs_gameplay.rs | done except netplay (TODO(net)) | start_classic: not yet run |
| smw/gamemodes/* (GameMode, GameModeTimer, all 22 modes, bonus house, 3 minigames) | smw/gamemodes/*.rs | done except netplay disconnect text | via start_classic / frenzy_cpu, not yet run |
| smw/main.cpp create_gamemodes, mode globals | smw/main.rs | done | |
| smw/objects/blocks/* (IO_Block + 11 blocks) | smw/objects/blocks/*.rs | done | not yet (gameplay not reached in replays) |
| smw/objects/moving/* (MovingObject, CarriedObject, 19 MO_*) | smw/objects/moving/*.rs | done | not yet |
| smw/objects/carriable/* (11 CO_*) | smw/objects/carriable/*.rs | done | not yet |
| smw/objects/powerup/* (MO_Powerup + 25 PU_*) | smw/objects/powerup/*.rs | done | not yet |
| smw/objects/walkingenemy/* | smw/objects/walkingenemy/*.rs | done | not yet |
| smw/objects/overmap/* (IO_OverMapObject + 10 OMO_*) | smw/objects/overmap/*.rs | done | not yet |
| smw/objects/IO_BulletBillCannon, IO_FlameCannon, MysteryMushroomTempPlayer, SwitchColor.h, ThrowBlockType.h | smw/objects/*.rs | done | not yet |
| smw/ObjectContainer.cpp, objectgame.cpp, objecthazard.cpp | smw/object_container.rs, objectgame.rs, objecthazard.rs | done | not yet |
| server/* (smw-server) | server/*.rs, bin/smw_server.rs | done; rooms and players use a libc++-order unordered_map (unit test vs `tools/ref/unordered_map_order.cpp`) | `tools/ref/net_interop.sh`: every C++/Rust client pairing on both servers; `net_game_interop.sh` SYNCED with `SMW_SERVER` = Rust server |
| screenshot/screenshot.cpp | | intentionally not ported: dead code upstream (not in CMake, does not compile against current sources); the level editor's `takescreenshot` covers the feature | n/a |
| worldeditor/worldeditor.cpp | worldeditor/worldeditor.rs, bin/worldeditor.rs | done | `tools/editor_parity.sh`: 5 scripted sessions (paint, stages/vehicles/warps/items, navigate/resize, stage menus incl. delete) match C++ dump, screenshots and saved world files byte for byte |
| leveleditor/leveleditor.cpp | leveleditor/leveleditor.rs, bin/leveleditor.rs | done | `tools/editor_parity.sh`: 6 scripted sessions (tiles, blocks/items/warps/no-spawn/eyecandy, platforms, hazards/move/copy/mode items/Save As/Find, tile and animated tile types/backgrounds/clear, smoke) match C++ dump, screenshots and saved .map/.tls files byte for byte |
| (harness) common/EditorHarness.cpp | common/editor_harness.rs | done | shared by both editors; see EDITOR_REPLAY.md |
| common/GameModeSettingsSerialization.cpp | common/game_mode_settings_serialization.rs | done | tour-stop settings in every replay that loads a tour or world |
| smw/network/NetConfigManager.cpp (servers.toml) | smw/network/net_config_manager.rs | done | `tools/ref/net_config_interop.sh`: 300 fuzzed files written byte-identically to toml11 |

## Known C++ issues

- `RandomNumberGenerator::getBoolean(scaleMax, positiveThreshold)` (`get_boolean_threshold`) asserts `positiveThreshold < scaleMax && positiveThreshold >= 0` (`random_number_generator.rs:43`, a `debug_assert!` port of the C++ `assert`). Replay `opt_gameplay` trips it at frame 1209 in a release build with debug assertions. The C++ has the same assert, compiled out in the Release reference, so a debug C++ build would fail it too; the caller passing the bad range is not yet traced.
