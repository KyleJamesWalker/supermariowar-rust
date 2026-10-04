# Fixed four out of bounds accesses and a multi star ranking regression

This PR has five fixes. Four are memory errors that AddressSanitizer or UBSan flags on current `master`, two in the game and two in the world editor. The fifth is a ranking bug in multi star mode from the same commit as fix 4.

Each fix is its own commit, so you can take or drop any of them separately. I am also happy to split them into separate PRs. Four fixes change output, but only in cases that were broken. These are multi star mode (fixes 4 and 5), deleting a world stage with vehicles, and saving a minigame stage.

## What changed

### 1. Frenzy cards in the boss minigame

The fire boss minigame spawns frenzy cards (`MiniBoss.cpp:82`). Picking one up always cast `game_values.gamemode` to `CGM_Frenzy` and called `setFrenzyOwner` (`MO_FrenzyCard.cpp:44`), which wrote past the end of the `CGM_Boss_MiniGame` object. The call now only runs in Frenzy mode, using the same `gamemode ==` check as `GSGameplay.cpp`. Frenzy mode itself does not change.

```
ERROR: AddressSanitizer: heap-buffer-overflow ... WRITE of size 8
    #0 CGM_Frenzy::setFrenzyOwner(CPlayer*) Frenzy.cpp:38
    #1 MO_FrenzyCard::collide(CPlayer*) MO_FrenzyCard.cpp:44
0x... is located 0 bytes after 752-byte region
allocated by thread T0 here:
    ...
    #1 create_gamemodes() main.cpp:258
```

Line 258 is `new CGM_Boss_MiniGame()`. With `-fsanitize=vptr`, UBSan reports the downcast itself.

### 2. Deleting a stage that has vehicles (world editor)

When the editor deletes a stage, the vehicle loop (`worldeditor.cpp:4148`) calls `RemoveVehicleFromTile`, which erases from `vehiclelist` while the loop keeps its old iterators. The loop skips the vehicle after each removed one and then reads past the end of the list. In my test, two of the five vehicles of the deleted stage survived. Those survivors kept the old index, so they pointed at the next stage. The loop now erases through its own iterator.

```
ERROR: AddressSanitizer: container-overflow ... READ of size 8
    #0 editor_stage() worldeditor.cpp
```

### 3. Saving minigame stages (world editor)

`SaveStage` (`worldeditor.cpp:3650`) looks up `g_iNumGameModeSettings[iMode]`, which has 22 entries. Bonus houses (24), the minigames (25-27) and loaded minigame stages (1000-1002) read past it. A saved boss stage then gets an arbitrary number of settings: 7 in my build instead of 3. The count now comes from `serializeGMS`. I checked that it returns the table's value for all 22 battle modes, 3 for the boss minigame and 0 for the others.

```
runtime error: index 24 out of bounds for type 'int[22]'
ERROR: AddressSanitizer: global-buffer-overflow ... in SaveStage(short)
```

### 4. Multi star mode loop bounds

e63375d7 (2024-04-21) changed `iStar < list_players_cnt - 1` to `iStar + 1 <= players.size()` (`Star.cpp:104`, `:184`). That is one iteration more than there are stars.

- With four players, the loop reads `starPlayer[3]`, past the end of the array.
- With two or three players, the extra slot is always empty. `think()` then reassigns all stars and places them again on every frame, so star stealing has been broken since e63375d7.

This commit restores the original bounds, matching `isplayerstar()`, and with them star stealing for 2 and 3 players. It changes how those games play, so it is the one most worth a look. With this fix alone, four-player games play exactly as before (fix 5 changes them). In scripted CPU games on Wacky Woods, a stolen star stayed with the stealer this long with fix 4 alone:

| Players | Before: steals, frames held (median, range) | After: steals, frames held (median, range) |
|---|---|---|
| 2 | 35, 3 (1-16) | 11, 151 (60-493) |
| 3 | 36, 1.5 (1-6) | 1, 392 |
| 4 | 17, 599 (60-1813) | identical |

```
Star.cpp:104: runtime error: index 3 out of bounds for type 'CPlayer *[3]'
```

I mapped the line number to `master`.

### 5. Multi star ranking

e63375d7 also renamed the output parameter of `GetScoreRankedPlayerList` (`GameMode.cpp:307`) to `outPlayers`, but the bubble sort still swaps the entries of the global `players` vector. Only multi star mode calls it. The returned list stays in the current `players` order, so stars go to players regardless of score. The global player order also changes mid-game. That changes update and draw order and leaves `localID` out of step with the vector index.

The sort now works on `outPlayers` again. No sanitizer flags this one. Star mode asks for ascending order (`fGetHighest = false`). A trace of the second ranking call in the 3-player game shows the effect, with each entry as `player:score`. The two runs had already diverged, so the scores differ:

```
before: out=2:5,0:5,1:4  global=1,2,0    (not ascending; global order changed)
after:  out=2:4,0:5,1:5  global=0,1,2    (ascending; global order unchanged)
```

## Testing

- **Sanitizers:** I built `smw` and `smw-worldedit` with `-fsanitize=address,undefined` (Apple clang 21, arm64), with and without the fixes. I ran each build through about 60 scripted games and world editor sessions. The four reports above go away, and the boss, star and editor scripts that used to abort now run to the end. The remaining reports are unrelated (see below).
- **No other changes:** with all five fixes, 42 scripted games and 13 level and world editor sessions give the same output as before. That covers per-frame game state, screenshots and saved files for every game mode, three minigames, tours, tournaments and world mode. None of them uses multi star mode, which fixes 4 and 5 change on purpose.
- **Build:** the default CMake build compiles with no new warnings in the changed files.
- **CI:** all workflows pass on my fork ([run](https://github.com/KyleJamesWalker/supermariowar/actions/runs/37227044509)). Android is disabled in the workflow (`if: false`).

The same runs also report a few unrelated issues, which I left out of this PR. These are map reads at index -1 and -19 in `map.h`, an uninitialized `bool` in `MiniPipe.h` and a shift overflow in `input.cpp`.

## Background

I am working on a Rust port of the game ([source](https://github.com/KyleJamesWalker/supermariowar-rust), [playable in the browser](https://www.kylejameswalker.com/supermariowar-rust/)). Claude Opus 5.5 wrote most of it. It tests how well an AI model handles a migration this size when a golden set of tests keeps it on track.

Those tests are a harness that compares the port with this C++ code frame by frame ([supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference)). It replays scripted input and compares state dumps. Where the C++ silently read or wrote out of bounds, the Rust code panicked, which is how the four memory errors surfaced. The ranking bug turned up while checking the multi star fix. I reviewed each fix and its evidence before sending this. I can send a few smaller findings later as separate PRs, if they are welcome. Thanks for keeping the game alive!
