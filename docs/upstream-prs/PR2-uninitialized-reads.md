# Fixed uninitialized reads that make Linux and macOS play differently

The same game plays differently on Linux and macOS today. Gameplay reads three class members before anything sets them, so CPU players and race goals depend on whatever the heap holds. macOS builds behave as if those values were 0, and Linux builds do not. I replayed 45 scripted games with fixed seeds and inputs:

| Build (upstream `master` plus replay hooks) | Games identical to a macOS build of the same code |
|---|---|
| Linux arm64, without this PR | 23 of 45 |
| Linux arm64, with this PR | 45 of 45 |

The exact count varies a little between runs. On an earlier `master` (5693918f), Linux x86_64 split the same way (23 or 24 of 45). There, both architectures reached 45 of 45 with equivalent default initializers ([investigation run](https://github.com/KyleJamesWalker/supermariowar-cpp-reference/actions/runs/37408856280)). For netplay, this means a Linux and a macOS client can drift apart, because each one runs its own AI and race goal placement.

This PR has six small fixes, one commit each, so you can take or drop any of them:

- Fixes 2, 4 and 6 are the three members behind the divergence.
- Fixes 3 and 5 are two more uninitialized reads that valgrind reports. Fix 5 only shows with a hand-edited map.
- Fix 1 is a plain bug that cuts new netplay room names short.

Only fix 4 changes how a game plays on macOS: race goal placement on maps without enough race goal locations.

## What changed

### 1. New room names cut at 15 characters

`NewRoom(name, password)` (`ProtocolPackages.h:150`) ends the name at `NET_MAX_PLAYER_NAME_LENGTH - 1` instead of `NET_MAX_ROOM_NAME_LENGTH - 1`. The menu field and the server both accept 31 characters, so "Saturday Night Mario Party" reached the server as "Saturday Night ".

### 2. CPU players' `iFallDanger`

The `CPlayerAI` constructor never sets `iFallDanger`, and `Think()` reads it (`ai.cpp:242`) before anything assigns it. The constructor now sets it to 0.

```
Conditional jump or move depends on uninitialised value(s)
   at CPlayerAI::Think(COutputControl*) (src/smw/ai.cpp:242)
```

### 3. New worlds saved with uninitialized music and initial items

The world editor's New World builds the world (`worldeditor.cpp:4704`) and saves it right away (`:4708`). The `WorldMap(w, h)` constructor leaves `iMusicCategory` and `iNumInitialBonuses` unset, so the new file's music category and initial item list come from the stack (`world.cpp:745`, `:888`, `:902`). Both members now default to 0.

```
Conditional jump or move depends on uninitialised value(s)
   at WorldMap::Save(...) const (src/smw/world.cpp:888)
   by savecurrentworld() (src/worldeditor/worldeditor.cpp)
 Uninitialised value was created by a stack allocation
   at new_world() (src/worldeditor/worldeditor.cpp)
```

### 4. Race goals placed with an uninitialized goal count

The `OMO_RaceGoal` constructor calls `placeRaceGoal()` before it sets `quantity`. On maps with fewer race goal locations than goals, `placeRaceGoal()` uses `quantity` for the minimum distance between random goals (`WO_RaceGoal.cpp:154`). The constructor now sets `quantity` first.

```
Conditional jump or move depends on uninitialised value(s)
   at OMO_RaceGoal::placeRaceGoal() (src/smw/objects/overmap/WO_RaceGoal.cpp:154)
   by CGM_Race::init() (src/smw/gamemodes/Race.cpp:63)
```

This is the one fix that changes play on macOS. In my macOS replays the old value behaved as 0, so placement tried to keep goals more than 250 pixels apart. Now the distance is `250 - quantity * 25` as intended, so goals can be closer together.

### 5. Tileset IDs missing from a map's table

`MapReader1800::read_tileset` sizes its translation arrays by the highest tileset ID (`MapReader18xx.cpp:73`) but only fills the IDs in the table. A tile or moving platform tile that uses a missing ID reads an uninitialized translation, width and height. Missing IDs are now treated like listed tilesets that are not installed: `TILESETUNKNOWN`, which the map draws as a red X.

No shipped map does this: 68 maps have gaps in their ID table, but none of their tiles uses a gap. I changed three tiles of `GG_Boo rondell.map` to a missing ID. valgrind then reports `read_tiles` (`MapReader18xx.cpp:111`, `:114`) and `CMap::drawPreview` before the fix, and none of these after.

### 6. Object network IDs read by the AI

`CObject::iNetworkID` is never set (`ObjectBase.h:104` still calls it unused), but the AI keys its attention list on `networkId()` (`ai.cpp:165`, `:704`, `:899`, `:962`). CPU players then ignore or keep chasing objects depending on heap contents. In a leave-one-out on 5693918f, leaving only this initializer out made 15 of the 45 games differ on Linux. The ID now starts at 0, which is how macOS builds behave, so macOS play does not change.

With every ID at 0, one ignored object makes the AI skip all objects in its target search, as macOS builds do today. Unique IDs would fix that but change play, so I left it for a separate change.

```
Conditional jump or move depends on uninitialised value(s)
   at CPlayerAI::Think(COutputControl*) (src/smw/ai.cpp:165)
 Uninitialised value was created by a heap allocation
   by CGM_Race::init() (src/smw/gamemodes/Race.cpp:63)
```

## Testing

- **Linux vs macOS:** the table above. Linux runs in an Ubuntu 24.04 container with clang 18 and libc++. With all six fixes, the Linux and macOS state dumps of all 45 games are identical frame for frame.
- **valgrind** (`--track-origins=yes`, Linux arm64, `master` plus the replay hooks, Release `-O3 -g`): a 4-CPU race, all 290 maps up to the start of a game, 13 editor sessions and the fix 5 map. Each report above goes away.
- **macOS behaviour:** with the fixes, 44 of the 45 games and all 13 editor sessions give the same per-frame state, screenshots and saved files as before. A short game on each of the 290 maps gives the same per-frame state. The exception is the race game (fix 4), which diverges at frame 464 when it places the goals.
- **Build:** the default CMake build (macOS, Apple clang) has the same 121 warnings before and after.
- **CI:** all workflows pass on my fork ([run](https://github.com/KyleJamesWalker/supermariowar/actions/runs/37411503740)). The workflow skips Android, as in upstream.

I left out a few reads that valgrind also reports and that change none of the 45 games: `MI_SelectField.cpp:227` on every start, `CGM_Pipe_MiniGame::fSlowdown`, `PlayerAwardEffects::awardangle` and the sledge brother hammers' `IO_MovingObject::iPlayerID`.

## Background

This follows PR 480. The fixes come from the same Rust port of the game ([source](https://github.com/KyleJamesWalker/supermariowar-rust), [playable in the browser](https://www.kylejameswalker.com/supermariowar-rust/)). Claude Opus 5.5 wrote most of it, as a test of how well an AI model handles a migration this size when golden tests keep it on track.

Those tests are a harness that compares the port with this C++ code frame by frame ([supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference)). Rust has no uninitialized memory, so the port had to pick a value wherever the C++ reads one. Those places became fixes 2, 4, 5 and 6. valgrind on the C++ confirmed each one and found fix 3. Running the same harness on Linux showed the divergence. Fix 1 is a plain length bug found while porting the netplay packets. I reviewed each fix and its evidence before sending this. Thanks again!
