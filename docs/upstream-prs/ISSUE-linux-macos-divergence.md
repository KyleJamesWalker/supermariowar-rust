# Uninitialized members make Linux and macOS builds play the same game differently

Draft issue for mmatyas/supermariowar, not filed. Backlog item M9 (`docs/UPSTREAM_BACKLOG.md`). Evidence is in `evidence/M9/`.

## Summary

Three class members that gameplay reads are never initialized: `CObject::iNetworkID`, `CPlayerAI::iFallDanger`, and `OMO_RaceGoal::quantity` at the time `placeRaceGoal()` reads it. Their values come from whatever the heap held. On macOS they behave as 0 in every game I recorded. On Linux they hold other values, so CPU players choose different targets, race goals land elsewhere, and the whole match diverges from that frame on. The CPU architecture plays no part: x86_64 and arm64 behave the same on each OS.

For netplay this means a Linux and a macOS client of the same game can drift apart. Each client runs its own AI and race goal placement from its own heap.

## Reproduction

I replay 45 scripted games (fixed seed and inputs) and compare a per-frame state dump with one recorded on macOS arm64. The builds are upstream `5693918f` plus a replay harness, clang with libc++, `-O2 -ffp-contract=off`. On Linux the data directory is copied in macOS directory order, so map and tour lists match.
The workflow that builds and replays all of these is `investigate-x86.yml` on branch `investigate/x86-divergence` of [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference) ([run](https://github.com/KyleJamesWalker/supermariowar-cpp-reference/actions/runs/37408856280)).

| Build | Games matching macOS |
|---|---|
| macOS arm64 | 45 of 45 |
| macOS x86_64 | 45 of 45 |
| Linux arm64 (Ubuntu 24.04) | 25 of 45 |
| Linux x86_64 (Ubuntu 24.04) | 23 or 24 of 45 |
| Linux arm64 or x86_64, with the proposed fix below | 45 of 45 |

The two Linux builds share 19 of their failing games, mostly at the same frame. Across four CI runs, arm64 failed the same 20 games each time. x86_64 failed 21 in three runs and 22 in the fourth, so the garbage can change between runs too. A race game on Block Piles with four CPU players shows the effect at its first frame: macOS makes 39 random number calls to set up the match, Linux makes 221. The first 25 calls agree. Then Linux rejects random race goal positions that macOS accepts, and retries each goal until its 32-try limit (`evidence/M9/cpu_race-rng.txt`).

To see the reads, run any CPU game under valgrind on Linux:

```
valgrind --track-origins=yes ./smw
```

## Affected lines

Lines refer to `5693918f`.

| Member | Declared | Read before any assignment | Effect |
|---|---|---|---|
| `CObject::iNetworkID` | `ObjectBase.h:104` (`// TODO: remove, unused`) | `ai.cpp:165`, `:704`, `:899`, `:962` | The AI keys its ignore list (`attentionObjects`) on it. With garbage IDs it ignores or keeps chasing the wrong objects. Without this fix, 15 games still diverge |
| `CPlayerAI::iFallDanger` | `ai.h:89` | `ai.cpp:242`, `:590`, `:596` | The first frames of a CPU player's fall-danger logic. Without this fix, 7 games still diverge |
| `OMO_RaceGoal::quantity` | `WO_RaceGoal.h:24` | `WO_RaceGoal.cpp:154`, through the `placeRaceGoal()` call at `:50`, before `:53` sets it | Sets the minimum distance between random race goals (`250 - quantity * 25`). Without this fix, the race game diverges |

Those counts come from removing one fix at a time from the patch below, on Linux arm64. The other members valgrind flags change no game in my set: `CGM_Pipe_MiniGame::fSlowdown` (`MiniPipe.h:34`, read through `WO_PipeCoin.cpp:79`), `PlayerAwardEffects::awardangle` (`PlayerAwardEffects.h:23`, drawing only), and `IO_MovingObject::iPlayerID` (`MovingObject.h:41`, read by `removeifprojectile` at `objectgame.cpp:131` for sledge brother hammers). The AI death check also reads `CMap::blockdata` and `mapdatatop` out of bounds when a player wraps past the screen edge (`map.h:208`, `:212`, from `ai.cpp:555` and `:621`). Those reads land in the struct padding of `objectdata`, which nothing initializes either.

## Sanitizer output

valgrind 3.22 on Linux arm64, from a race game:

```
Conditional jump or move depends on uninitialised value(s)
   at CPlayerAI::Think(COutputControl*) (src/smw/ai.cpp:165)
   by CPlayer::move()
   by GameplayState::update_world()
 Uninitialised value was created by a heap allocation
   at operator new(unsigned long)
   by CGM_Race::init() (src/smw/gamemodes/Race.cpp:63)

Conditional jump or move depends on uninitialised value(s)
   at OMO_RaceGoal::placeRaceGoal() (src/smw/objects/overmap/WO_RaceGoal.cpp:154)
   by OMO_RaceGoal::OMO_RaceGoal(gfxSprite*, short) (src/smw/objects/overmap/WO_RaceGoal.cpp:50)
   by CGM_Race::init() (src/smw/gamemodes/Race.cpp:63)
 Uninitialised value was created by a heap allocation
   by CGM_Race::init() (src/smw/gamemodes/Race.cpp:63)

Conditional jump or move depends on uninitialised value(s)
   at CPlayerAI::Think(COutputControl*) (src/smw/ai.cpp:242)
 Uninitialised value was created by a heap allocation
   by GameplayState::createPlayers() (src/smw/GSGameplay.cpp:299)
```

`evidence/M9/valgrind-sites.txt` lists every site and the games that hit it. `-fsanitize=undefined` and `-fsanitize=float-cast-overflow` report nothing on these paths, because reading an uninitialized member is not a check UBSan has.

## Proposed fix

Give each member a default initializer:

```cpp
// ObjectBase.h
int iNetworkID = 0;
// ai.h
short iFallDanger = 0;
// WO_RaceGoal.h
short quantity = 0;
// MiniPipe.h, PlayerAwardEffects.h, MovingObject.h
bool fSlowdown = false;
float awardangle = 0.0f;
short iPlayerID = 0;
```

Also value-initialize `objectdata` and `blockdata` in the `CMap` constructor, which zeroes the padding the out-of-bounds reads land in. A bounds check in the AI death check would be the cleaner fix for those reads, but it can change CPU behaviour near the screen edges.

With these defaults every build plays as macOS does today, and the 45 per-frame dumps stay identical on macOS. Two follow-ups change play and need a decision:

- `quantity`: set it before `placeRaceGoal()` runs, as the code intends. Random race goals can then sit closer together (`250 - quantity * 25` pixels instead of 250).
- `iNetworkID`: give each object a unique ID, or key `attentionObjects` on something else. With every ID 0, one ignored object makes the AI skip every object in its target search. That is today's macOS behavior.
