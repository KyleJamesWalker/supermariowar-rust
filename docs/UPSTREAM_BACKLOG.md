# Upstream backlog

Bugs found in the C++ [Super Mario War](https://github.com/mmatyas/supermariowar) while porting it, and port changes worth offering upstream, tracked until each lands or is dropped. Everything was found against upstream `5693918f`; confirm each item on current upstream `master` before opening a PR.

## How an item goes upstream

1. Confirm the bug on current upstream `master`, and record the commit in the item.
2. Fix it on a branch of [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference), with the replay or script that shows it.
3. Open a small upstream PR, one bug per PR, linking the evidence. Bigger behaviour changes start as an issue.
4. Once it merges, pull it in through the upstream sync (`UPSTREAM_SYNC.md`): drop the port code that reproduces the bug, regenerate the affected goldens, and check parity.
5. Update the item's status and links here.

Status: `found` (seen at `5693918f`), `confirmed` (still on `master`), `PR open`, `merged`, `synced` (the port no longer reproduces it), `dropped`.

## Memory safety

The port reproduces these so replays match the C++ byte for byte.

| ID | Bug | Port location | Status | Upstream |
|---|---|---|---|---|
| M1 | In the boss minigame, picking up a Frenzy card `static_cast`s the game mode to `CGM_Frenzy` and writes past the end of the non-Frenzy mode object | `src/smw/objects/moving/mo_frenzy_card.rs:90` | PR open | [#480](https://github.com/mmatyas/supermariowar/pull/480) |
| M2 | Star mode with four players reads `starPlayer[3]` out of bounds; the bytes after the array form a non-null pointer. With two or three players the same loop reassigns every star each frame, so steals never stick | applied here ahead of upstream (`PROGRESS.md`, Deliberate deviations) | PR open | [#480](https://github.com/mmatyas/supermariowar/pull/480) (`0f45d19e`) |
| M3 | `CMap` block lookups and AI map reads past the map edge read neighbouring fields | `src/common/map.rs:751` (`cpp_byte`), `src/smw/ai.rs` (`cmap_byte`) | found | |
| M4 | World editor: saving a tour stop in the bonus house or a minigame mode (24-27) indexes `g_iNumGameModeSettings` out of bounds | `src/worldeditor/worldeditor.rs:3303` | PR open | [#480](https://github.com/mmatyas/supermariowar/pull/480) |
| M5 | World editor: deleting a stage keeps iterating `vehiclelist` with iterators invalidated by `RemoveVehicleFromTile`, skipping elements and visiting stale slots | `src/worldeditor/worldeditor.rs:3478` | PR open | [#480](https://github.com/mmatyas/supermariowar/pull/480) |
| M6 | Reads of uninitialized values: race goal position (`placeRaceGoal`), an AI constructor field, map loader entries for IDs missing from the tile table, level editor `MapBlock::iSettings` (makes the editor's saved output vary between runs), and a game package's default constructor | `src/smw/objects/overmap/wo_race_goal.rs:52`, `src/smw/ai.rs:404`, `src/common/map/map_reader18xx.rs:43`, `src/leveleditor/leveleditor.rs:5182`, `src/smw/network/protocol_game_packages.rs:242` | found | |
| M7 | The tournament scoreboard draws skin frames that were never loaded; Release builds skip them silently | fixed in `d63bf67` | fixed upstream in `d70e4dc3` | |

## Logic

| ID | Bug | Port location | Status | Upstream |
|---|---|---|---|---|
| L1 | `NewRoom` cuts the room name at the player-name length (`NET_MAX_PLAYER_NAME_LENGTH - 1`) | `src/common_netplay/protocol_packages.rs:194` | found | |
| L2 | `getBoolean(scaleMax, positiveThreshold)` gets an out-of-range threshold; replay `opt_gameplay` trips the assert at frame 1209 in a debug build. The caller is not traced yet | `src/common/random_number_generator.rs:43` | found | |
| L3 | Map thumbnails miss hazards and platform shadows drawn by position, which still go to `blitdest` after `5c979393` | see `PROGRESS.md`, Known C++ issues | found | |
| L4 | `sdl3` branch (`1971b11c`): the map foreground layer is drawn wrong on native SDL3. `spr_frontmap` is an opaque PNG converted to the ARGB8888 screen format, which SDL3 gives `SDL_BLENDMODE_BLEND`, and it is RLE-encoded. The first map shows the foreground's magenta colour key, because the RLE encoder ignores the key on blended surfaces (X2); later maps show the previous map's foreground, because the branch's `predrawforeground` fills the surface without the lock that `5693918f` added on `master`, so the stale RLE data is drawn. With that lock merged, every map shows magenta. Fix: drop `SDL_SetSurfaceRLE` (P5), or set `SDL_BLENDMODE_NONE` on opaque converted images. Modelled with a standalone SDL 3.4.18 program (`sdl2-compat-convert-blend.md`); not run in the branch build, which needs SDL3_mixer | not in the port (SDL2) | found | file as an issue on the `sdl3` branch |
| L5 | `GetScoreRankedPlayerList` bubble-sorts the global `players` vector instead of `outPlayers` (since `e63375d7`), so multi star mode hands out stars in player order and the player order changes mid-game | applied here ahead of upstream (`PROGRESS.md`, Deliberate deviations) | PR open | [#480](https://github.com/mmatyas/supermariowar/pull/480) (`04766e2c`) |

## Web build

Upstream ships an Emscripten build, so these apply to it directly.

| ID | Bug | Port fix | Status | Upstream |
|---|---|---|---|---|
| W1 | Binding a control in the Controls menu hangs the page: `MI_InputControlField::SendInput` skips `SDL_WaitEvent` under `__EMSCRIPTEN__` but keeps looping | binds from each frame's polled events (`faa0cf4`) | found | |
| W2 | A standard-mapping gamepad's D-pad arrives as buttons 12-15, so nothing is bound to it | the D-pad also sends hat 0 (`a9ee8b3`) | found | |
| W3 | Backspace can navigate the page back while typing a name | the page cancels the browser default (`75b3cec`) | found | |

## Netplay

Both change the protocol, so open an issue first.

| ID | Bug | Port fix | Status | Upstream |
|---|---|---|---|---|
| N1 | Powerup blocks release different items on different clients: each rolls from its own RNG copy, and a joiner that misses a host's block hit falls one draw behind. Reproduced between two C++ clients with `tools/ref/net_game_blocks` (2 of 5 runs) | the host decides block contents (`7a0a43d`); `PROGRESS.md`, Netplay deviations | found | |
| N2 | Every other random outcome can differ between clients the same way: frenzy and collection cards, stomp/survival enemies, coin and objective relocations, hazard timers, respawn points and warp exits, shell breaks, and more (survey in `PROGRESS.md`) | the host decides every random outcome (`5fee1c4`, `src/smw/net_random.rs`); replaces N1's packages | found | |
| N3 | Game packets index `players` by player number; once a client's game ends and the vector shrinks, later packets index past it (undefined behaviour; the port panicked) | look players up by global ID (`3a883b4`) | found | |
| N4 | Deaths, scores, team removals and the end of the match are decided by every client from its own collisions, so a match can end on one client first with another winner, other scores or other deaths. Reproduced between two Rust clients with `tools/ref/net_game_classic` and `net_game_coins` before the fix; the C++ runs the same code | the host decides kills, removals, the score board, scores and game end (`075beae`, `src/smw/net_outcomes.rs`) | found | |

## Proposals

Deliberate behaviour changes; each needs the maintainer's agreement first. Details in `PROGRESS.md`, Deliberate deviations.

| ID | Change | Status | Upstream |
|---|---|---|---|
| P1 | A joystick's stick and D-pad drive the same bindings. Upstream's defaults bind the stick, so a D-pad-only pad does nothing | proposed | |
| P2 | Connected gamepads go to the first players at launch, keyboards fill the rest, and the next launch without pads restores the keyboard setup | proposed | |
| P3 | The first keyboard player can drive menus, not only player 1 | proposed | |
| P4 | Browser netplay through a WebSocket relay (`smw_relay`, `RELAY.md`) | proposed | |
| P5 | Make `SDL_SetSurfaceRLE` optional (the `SMW_NO_RLE` CMake option on `harness-rle`) or drop it. Through sdl2-compat RLE costs 8–16x CPU per frame (X1), on native SDL3 an RLE blit still costs 2.1–2.4 µs against 0.3 µs without, the web build runs no faster with it, and without it X2 cannot happen. The port draws without RLE on native | proposed | |

## Upstream changes to follow

Upstream work the port will need to sync, tracked so it isn't missed.

| ID | Change | Notes | Status |
|---|---|---|---|
| S1 | The `sdl3` branch ports the game to SDL 3.4.8, SDL3_image 3.4.4 and SDL3_mixer 3.2.2 (5 commits on `master`, ~50 files; `dad8a861` onwards). The maintainer plans to make it the default ([#479](https://github.com/mmatyas/supermariowar/pull/479#issuecomment-5927470541), 2026-10-01) | Fixes X1 for Super Mario War, but the branch has L4 (X2 on native SDL3): an RLE blit costs 2.1–2.4 µs on native SDL3 against 63–69 µs through sdl2-compat (0.3 µs without RLE). Port work: move the C++ harness onto the branch (the sound hooks need the most rework, since SDL3_mixer's track API replaces Mix channels), swap the port's `sdl2::sys` calls (116 files) for SDL3's, port the branch's own changes as an upstream sync, follow the mixer rewrite in the deterministic sound path, and build SDL3_image/SDL3_mixer for the web build. Wait until it lands on `master` | on hold |

## Outside Super Mario War

| ID | Issue | Where | Status |
|---|---|---|---|
| X1 | Homebrew sdl2-compat re-encodes RLE sprite sheets on every blit, so heavy scenes miss 60 fps | draft in `sdl2-compat-rle.md`, for libsdl-org/sdl2-compat. Moot for this game once S1 lands, still worth filing for other SDL2 games. CPU per frame on an idle machine (2026-10-04), RLE on → off: `flow_world` 2.5 → 0.25 ms, `gg_death_valley` 9.5 → 0.6 ms, `cpu_greed` 10.9 → 0.7 ms, `cpu_classic` 2.1 → 0.25 ms; the draft's older figures were taken under heavy load | drafted, not filed |
| X2 | `SDL_ConvertSurface` of an opaque surface to the ARGB8888 screen format returns `SDL_BLENDMODE_BLEND` (SDL2: `NONE`). With RLE, the RLE encoder then uses per-pixel alpha and ignores the colour key, so map foreground layers show their magenta key (10 of 44 replays, 121 of 290 sweep maps) | draft with repro in `sdl2-compat-convert-blend.md`, for libsdl-org/sdl2-compat. Native SDL3 behaves the same, so an SDL3 build (S1) needs RLE off (P5) or `SDL_BLENDMODE_NONE` on opaque images | drafted, not filed |
