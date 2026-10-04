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

- Upstream `5c979393` gave hazard and platform drawing an explicit destination, but the hazards and platform shadows drawn by position in map thumbnails still go to `blitdest`, so thumbnails lack them. The port reproduces this.
- `RandomNumberGenerator::getBoolean(scaleMax, positiveThreshold)` (`get_boolean_threshold`) asserts `positiveThreshold < scaleMax && positiveThreshold >= 0` (`random_number_generator.rs:43`, a `debug_assert!` port of the C++ `assert`). Replay `opt_gameplay` trips it at frame 1209 in a release build with debug assertions. The C++ has the same assert, compiled out in the Release reference, so a debug C++ build would fail it too; the caller passing the bad range is not yet traced.

## Deliberate deviations from the C++

Replays without joysticks still match the C++ goldens. `joy_menu` and `joy_game` exercise these deviations and are checked against Rust goldens (`tools/golden_rust/`, `make_golden.sh` with `RUST_GOLDEN=1`); `parity.sh` marks them "(Rust golden)".

- Stick and D-pad are one control: on a joystick, a binding to a stick 1 direction also fires from the same hat direction and vice versa, in game and in menus (`CPlayerInput::update`). Upstream reads only the bound one, and its defaults bind the stick, so a D-pad-only pad did nothing.
- Players get inputs at every launch (`assign_inputs`, `main.rs`): connected joysticks first, then the right keyboard set (player 1's keyboard bindings), then the left one (player 2's); those players become human. 1 pad: P1 pad, P2 right, P3 left; 2 pads: P1-2 pads, P3 right, P4 left; 3: P4 right; 4: all pads. While the page's touch controls are on, the right keyboard set (the keys they press) goes to player 1 ahead of the joysticks. The first launch with joysticks saves the players' `playercontrol` to `pad_players.txt` in the settings directory; the next launch without joysticks restores it, sets every player back to their own keyboard set, saves the settings and deletes the file. Without that file (no pad history) a launch with no joystick keeps the saved assignment, as upstream. A joystick's bindings belong to it (`inputConfiguration[pad][1]`), so they follow it to whichever player has it; switching a player to the keyboard in Controls takes the first keyboard set no other player has.
- Menus: the first keyboard player may drive menus even when it is not player 1 (upstream: only player 1), and player 1's keyboard fallback in menus is off while another player has those keys.
- Web: binding a control in the Controls menu does not block. Upstream's `MI_InputControlField::SendInput` skips `SDL_WaitEvent` under `__EMSCRIPTEN__` but keeps looping, which hangs the page (an upstream bug). The field reads each frame's polled events instead and binds the first that fits.
- Web: a standard-mapping gamepad's D-pad (browser buttons 12-15) also sends hat 0, as desktop SDL reports it. See `REPLAY.md`, Recordings.
- Touch controls (`web/touch.js`) are page-side: they dispatch player 1's default key events, so the game is unchanged and a touch session records and replays as keyboard input. Rebinding player 1's keys leaves the touch controls on the old keys.
- Replays ignore connected pads: natively, `--replay` and the harness set the `SDL_JOYSTICK_HIDAPI`, `SDL_JOYSTICK_MFI` and `SDL_JOYSTICK_IOKIT` hints to 0, leaving only the replay's virtual joysticks. The browser's SDL has no virtual joysticks, so a browser replay opens no pads and takes the joystick count from the replay; its events carry the device index the game binds, so pad sessions can be watched in the browser.
- The Rust harness records and replays axes and buttons 0-63; a browser recording that uses axis 6+ or button 16+ does not replay on the C++ harness.
- Netplay works in the browser (upstream's web build is `NO_NETWORK` and disables Multiplayer). A WebSocket network layer carries ENet's connections to `smw_relay` (`RELAY.md`), the game host skips NAT punching, and skins and maps compress with Emscripten's zlib port because libz-sys builds a `Z_SOLO` zlib for wasm32. Browser and native clients cannot play together.

## Netplay deviations

- The game host decides every random outcome that changes a net game. Upstream, every client rolls them from its own copy of the shared RNG. The copies stay in step only while every client makes the same draws; one missed draw (a joiner whose replay of the host's map collision misses a block, a death one client did not see) and every later block, enemy, card, coin spot, hazard timer and respawn point differs. `tools/ref/net_game_blocks` shows it between two C++ clients. `src/smw/net_random.rs` lists the sites: block contents, frenzy cards, stomp and survival enemies, collection cards, greed coin drops, relocations (coins, cards, eggs, flags, flag bases, phanto keys, domination areas, hill zones, yoshis, secret powerups), flag base, race goal and phanto wandering, pirhana and cannon timers, respawn points, warp exits, podobo rain, POW kill order, mystery mushroom swaps, bullet bill powerups, held shells breaking, secrets, throw box items, star and tag reassignment, star timeouts, bombs and random-style boomerangs.
  - The host runs each site with the RNG recording its draws and sends `NET_G2P_RANDOM_EVENT` (93: site, object ID context, arguments, draws; placements also carry the result). A joiner that got `NET_G2P_HOST_DECIDES_RANDOM` (92) after the sync never runs a site itself: it replays the host's with the host's draws a few frames later. Objects made during setup and inside events get matching `iNetworkID`s (unused upstream), so events can name them. A block waits until the joiner's copy stops bouncing, and a block the joiner never saw hit turns used. A joiner's player that waits 90 frames for a respawn or warp exit the host never sends picks its own. Hazards wait, and relocated objects wait out of reach, until the host's result arrives.
  - Both package types are new and the protocol version is unchanged: a C++ joiner ignores them and rolls as before, and a Rust joiner of a C++ host never hears from the host and rolls as before, so only Rust-to-Rust games (native or browser) match. The block-only packages 90 and 91 of the previous build are gone; both clients need this build.
  - Still per client, as upstream: scores and game over (each client counts its own collisions), which players die from hazards and enemies, eyecandy, and the item a view block shows while it cycles (cosmetic: the host still decides what it releases). CPU players do not exist in net games.
- Game packets name players by global ID, not `players` index (upstream indexes the vector, which a client that ended its game first has already shortened).
