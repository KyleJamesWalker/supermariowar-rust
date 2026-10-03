# Differential replay

The C++ game is rebuilt as a deterministic, headless, scriptable reference that writes one text block of game state per frame. The Rust port must read the same replay scripts and emit byte-identical dumps; `tools/diffreplay.py` reports the first frame where they diverge.

- Reference build: `~/work/smw-ref` (a copy of `~/work/supermariowar` with `tools/cpp-harness.patch` applied). The original checkout is never modified.
- Scripts: `tools/replays/*.txt`. Golden output: `tools/golden/<script>/dump.txt` plus `frame_<n>.png` screenshots.

## Building the reference

```sh
rsync -a --exclude .git ~/work/supermariowar/ ~/work/smw-ref/
cd ~/work/smw-ref && patch -p1 < <this repo>/port/tools/cpp-harness.patch
mkdir build && cd build
cmake .. -DNO_NETWORK=ON -DBUILD_TESTS=OFF -DCMAKE_BUILD_TYPE=Release \
         -DDISABLE_DEFAULT_CFLAGS=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"
make -j smw
```

Homebrew dependencies: `sdl2-compat`, `sdl2_image`, `sdl2_mixer`, `yaml-cpp`, `zlib` (tested with sdl2-compat 2.32.72, Apple clang 21).

- `-DDISABLE_DEFAULT_CFLAGS=ON` is required on Apple Silicon: `cmake/PlatformArm.cmake` matches `arm64` and adds 32-bit ARM flags (`-marm -mfpu=vfp`) that clang rejects.
- `-ffp-contract=off` stops clang fusing `a*b+c` into FMA instructions on arm64. Rust never fuses, so without this flag float positions drift by an ulp.

## Running

```sh
tools/run_ref.sh tools/replays/start_classic.txt [out_dir]    # dump.txt + frame_<n>.bmp
tools/make_golden.sh [replay.txt ...]                          # regenerate tools/golden/
tools/diffreplay.py tools/golden/start_classic/dump.txt out/dump.txt
tools/diffreplay.py --shots tools/golden/start_classic out/
```

`run_ref.sh` sets `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy`, a fresh temporary `HOME`, deletes `data/maps/cache/mapsummary.txt`, and runs with `--datadir ~/work/smw-ref/data`. The dummy video driver works: the game draws into a 640x480 software surface and the renderer falls back to `software`. A 2,500-frame replay runs in about 3 s.

`diffreplay.py` options: `--context N` prints N identical frames before the divergence, `--ignore R` drops a record type, `--float-tol 0.001` relaxes `%.4f` fields while porting, and `--shots --max-diff N` compares screenshots by differing-pixel count. It needs only the Python standard library (Pillow is used if installed). Exit status: 0 identical, 1 diverged, 2 error.

## Environment variables

Each variable is a no-op when unset or empty. The Rust binary must honour the same set.

| Variable | Effect |
|---|---|
| `SMW_SEED=<u32>` | Reseed Well512 with this seed before any game code runs, `srand(seed)`, reset the RNG call counter, and switch sound to the virtual clock (see Nondeterminism). Accepts decimal or `0x` hex. |
| `SMW_NOLIMIT=1` | Skip the frame-limiter sleeps in `FPSLimiter::afterFlip`. Logic is frame-based; dumps with and without the limiter are identical. |
| `SMW_REPLAY=<file>` | Scripted keyboard input (format below). |
| `SMW_FRAMES=<n>` | Process frames `0..n-1`, then set `appstate = Quit` and shut down normally. |
| `SMW_DUMP=<file>` | Append one state block per frame (format below). |
| `SMW_SHOT_FRAMES=1,60,300` | Save the screen surface after those frames. |
| `SMW_SHOT_DIR=<dir>` | Where screenshots go (default `.`), named `frame_<n>.bmp`. |
| `SMW_MAP=<name>` | Select the start map when the menu is created (see below). |

## Frame loop

The frame counter `N` starts at 0 and counts iterations of the main loop in `gameloop()`:

1. Push every replay event whose frame is `N` with `SDL_PushEvent`, in file order.
2. `currentState->update()`.
3. Write the dump block for `N`.
4. If `N` is in `SMW_SHOT_FRAMES`, `SDL_SaveBMP(screen)`: the 640x480 ARGB8888 surface as drawn by this frame, before `gfx_flipscreen()`.
5. `N += 1`; if `N >= SMW_FRAMES`, quit.
6. FPS limiter, `gfx_flipscreen()`.

Splash timeline with no input: frame 0 draws "Loading...", frame 1 loads all graphics and sounds, frame 2 onward shows "Press Any Key To Continue". A `menu_select`, `menu_cancel` or `menu_random` press enters the main menu on the frame it is pushed. Press on frame 2 or later: the key check runs before the frame-1 load, so an earlier press skips loading.

## Replay format

One event per line: `<frame> <down|up> <SDL key name>`. The key name is everything after the second field, passed to `SDL_GetKeyFromName` (`Up`, `Return`, `Escape`, `Right Ctrl`, `Right Shift`, `E`). Frames must be non-decreasing. Blank lines and lines starting with `#` are ignored. Unknown keys or malformed lines exit with status 2.

Pushed events are `SDL_KEYDOWN`/`SDL_KEYUP` with `keysym.sym` = the key, `keysym.scancode = SDL_GetScancodeFromKey(sym)`, `mod = KMOD_NONE`, `repeat = 0`, `windowID = 0`, `timestamp = 0`. The game reads only `sym` and `mod`.

Lines of the form `#@ key=value` are run parameters for `run_ref.sh` (and the Rust runner): `seed`, `frames`, `map`, `shots`. They are comments to the game itself.

A key press is a `down` and a later `up`. `fPressed` fires only on the down edge, so pressing the same key twice needs an `up` in between.

Default keyboard bindings from `CGameValues::init` (`controlkeys` in `GameValues.cpp`); players 1 and 2 are human, 3 and 4 are off:

| | Player 1 game | Player 1 menu | Player 2 game | Player 2 menu |
|---|---|---|---|---|
| left / right | `Left` / `Right` | `Left` / `Right` | `A` / `D` | `A` / `D` |
| jump / up | `Up` | `Up` | `W` | `W` |
| down | `Down` | `Down` | `S` | `S` |
| turbo / select | `Right Ctrl` | `Return` | `E` | `E` |
| powerup / cancel | `Right Shift` | `Escape` | `Q` | `Q` |
| start / random | `Return` | `Space` | - | - |
| cancel / scroll fast | `Escape` | `Left Shift` | - | - |

Menu path to a default 2-player Classic game, used by `start_classic.txt`: `Return` (splash), `Return` (main: Start), `Return` (match selection: Continue), `E` (player 2 ready on team select), `Return`, `Return` (to game settings), `Return` (Start). With presses on frames 10, 30, 50, 70, 100, 130, 160, gameplay begins on frame 190.

## Dump format

Plain ASCII, `\n` line endings, one block per frame. Fields are separated by single spaces. Lines appear in exactly this order:

```
F <frame> <state>
M <menu> focus=<i> modifying=<0|1>
G mode=<m> gameover=<0|1> winner=<w>
P id=<id> team=<t> ix=<ix> iy=<iy> fx=<f> fy=<f> velx=<f> vely=<f> state=<s> score=<n> powerup=<p> inair=<0|1>
O noncol=<n> obj0=<n> obj1=<n> obj2=<n> ec0=<n> ec1=<n> ec2=<n>
R calls=<n> last=<u32>
```

| Record | When | Fields |
|---|---|---|
| `F` | always | `<state>` is `splash`, `menu`, `gameplay` or `other`, from `GameStateManager::currentState` |
| `M` | state `menu` | `<menu>` is `MenuState::mCurrentMenu`, named in the table below. `focus` is the index of `UI_Menu::m_currentFocus` in `controls` (insertion order of `AddControl`/`AddNonControl`), or -1. `modifying` is `UI_Menu::fModifyingItem`. |
| `G` | state `gameplay` | `mode` = `(int)game_values.gamemode->gamemode` (`GameModeType`: classic 0, frag 1, ... ; bonus 999, minigames 1000-1002). `gameover`, `winner` = `gamemode->gameover`, `gamemode->winningteam`. |
| `P` | state `gameplay` | one line per entry of the global `players` vector, in vector order. `id` = `globalID`, `team` = `teamID`, `ix`/`iy` shorts, `fx`/`fy`/`velx`/`vely` floats, `state` = `(int)PlayerState` (Waiting 0, Spawning 1, Dead 2, Ready 3, EnteringWarpUp 4 ... LeavingWarpRight 11), `score` = `score->score` (0 if null), `powerup` short, `inair` bool. |
| `O` | state `gameplay` | element counts of `noncolcontainer`, `objectcontainer[0..2]`, `eyecandy[0..2]` at the end of the frame (dead objects not yet cleaned count). |
| `R` | always | `calls` = number of `Well512RandomNumberGenerator::getNext()` calls since the reseed. `last` = the value the most recent call returned (0 before the first call). |

Floats are printed with C `printf("%.4f", (double)value)`. Rust `format!("{:.4}", value as f64)` produces the same text, including `-0.0000` for negative zero and round-half-to-even on exact ties.

Menu names:

| Name | `MenuState` member | Name | `MenuState` member |
|---|---|---|---|
| `main` | `mMainMenu` | `match_selection` | `mMatchSelectionMenu` |
| `options` | `mOptionsMenu` | `game_settings` | `mGameSettingsMenu` |
| `gameplay_options` | `mGameplayOptionsMenu` | `map_filter_edit` | `mMapFilterEditMenu` |
| `team_options` | `mTeamOptionsMenu` | `tour_stop` | `mTourStopMenu` |
| `powerup_drop_rates` | `mPowerupDropRatesMenu` | `world` | `mWorldMenu` |
| `powerup_settings` | `mPowerupSettingsMenu` | `team_select` | `mTeamSelectMenu` |
| `projectile_limits` | `mProjectileLimitsMenu` | `tournament_scoreboard` | `mTournamentScoreboardMenu` |
| `projectile_options` | `mProjectileOptionsMenu` | `bonus_wheel` | `mBonusWheelMenu` |
| `graphics_options` | `mGraphicsOptionsMenu` | `net_servers` | `mNetServersMenu` |
| `eyecandy_options` | `mEyeCandyOptionsMenu` | `net_edit_servers` | `mNetEditServersMenu` |
| `sound_options` | `mSoundOptionsMenu` | `net_lobby` | `mNetLobbyMenu` |
| `player_controls_select` | `mPlayerControlsSelectMenu` | `net_new_room` | `mNetNewRoomMenu` |
| `player_controls` | `mPlayerControlsMenu` | `net_new_room_settings` | `mNetNewRoomSettingsMenu` |
| `mode_options` | `mModeOptionsMenu` | `net_room` | `mNetRoomMenu` |

Any other pointer prints `unknown`.

Example (frame 400 of `start_classic`):

```
F 400 gameplay
G mode=0 gameover=0 winner=-1
P id=0 team=0 ix=522 iy=102 fx=522.0000 fy=102.5000 velx=-5.5000 vely=3.9000 state=3 score=10 powerup=-1 inair=1
P id=1 team=1 ix=283 iy=38 fx=283.0000 fy=38.8000 velx=0.0000 vely=0.4000 state=3 score=10 powerup=-1 inair=0
O noncol=6 obj0=0 obj1=0 obj2=0 ec0=0 ec1=0 ec2=4
R calls=44 last=516273304
```

## SMW_MAP

Applied at the end of `MenuState::init()`, which runs when the splash screen is dismissed (after `MapList::ReadFilters()` has reset the current map to the first one). The name is matched with `MapList::findexact(name, false)`, a case-insensitive match on the map key (file name without the `creator_` prefix and `.map`, e.g. `Block Piles` for `4matsy_Block Piles.map`). If that fails, `MapList::find(name)` matches a substring of the path. Then `miMapField->LoadCurrentMap()` refreshes the game settings preview. An unknown name exits with status 2. Without `SMW_MAP` the first map in sorted order (`0smw`) is current.

## Nondeterminism sources

| Source | Where | Handling |
|---|---|---|
| Well512 seeded from `srand(time(0))` + `rand()` in its constructor | `RandomNumberGenerator.cpp` | `SMW_SEED` reseeds via `Well512::initialize(seed)` (state[0] = seed, then LCG `state[i] = (state[i-1] * 1103515245 + 12345) & 0x7fffffff`, index 0) at the start of `main_game()`, before `create_globals()`. |
| `srand(time(NULL))` | `main.cpp` | Replaced by `srand(seed)` when seeded. Nothing in the game calls libc `rand()` afterwards (`SystemRandomNumberGenerator` is unused). |
| `sfxSound::play()` refuses a retrigger within 40 ms of `SDL_GetTicks()`, and its return value drives announcer selection in `PlayerAwardEffects` | `sfx.cpp` | Seeded runs use a virtual clock: `1000 + N * 16` ms (`WAITTIME`), where `N` is the frame counter. |
| `Mix_PlayChannel` fails when all 16 channels are busy, which depends on the audio thread | `sfx.cpp` | Seeded runs treat the failure as success (the throttle timestamp still updates). The Rust port should model `play()` as succeeding unless throttled. |
| Frame limiter reads `SDL_GetTicks()` | `FPSLimiter.cpp` | Wall clock only controls sleeping and the F1 FPS overlay. `SMW_NOLIMIT` skips the sleeps. |
| `options.bin` / `controls.sdl2.bin` in `$HOME/Library/Preferences/.smw/` | `GameValues.cpp` | `run_ref.sh` points `HOME` at an empty temp directory, so defaults from `CGameValues::init` apply. The game writes `options.bin` there on exit. |
| Map summary cache `data/maps/cache/mapsummary.txt` written on exit | `MapList.cpp` | Holds only precomputed auto-filter flags. `run_ref.sh` deletes it before each run. |
| OS events (window, joystick hot-plug, real keyboard) | `SDL_PollEvent` | Not filtered. The dummy video driver generates no input and no joysticks are attached. |
| Music-finished callback on the audio thread | `GSGameplay.cpp musicfinished()` | Only restarts music. No RNG or game state, so it is left alone. |
| Screenshot file names (`Insert` key) and netplay timestamps | `gfxSDL.cpp`, `net.cpp` | Unreachable: no script presses `Insert`, and the build uses `NO_NETWORK`. |
| FMA contraction on arm64 | compiler | `-ffp-contract=off`. |

Map list order is deterministic: maps live in a `std::multimap` keyed by name.

## C++ changes

`tools/cpp-harness.patch` (apply with `patch -p1` in the repo root):

- `src/smw/Harness.{h,cpp}`: env parsing, event injection, dump, screenshots, frame cap.
- `main.cpp`: `harness::init()` at the top of `main_game()`, `harness::frameStart()`/`frameEnd()` around `update()`, seeded `srand`.
- `FPSLimiter.cpp`: early return under `SMW_NOLIMIT`.
- `RandomNumberGenerator`: `callCount()`, `lastValue()`, `resetCallCount()`.
- `sfx`: `sfx_ticks` clock hook and `sfx_ignore_channel_failure`.
- `uimenu`: `UI_Menu::currentFocusIndex()`. `GSMenu`: menu name/focus accessors and `SMW_MAP`. `player.h`: `friend struct HarnessAccess`.

## Golden outputs

| Script | Frames | Seed | Content |
|---|---|---|---|
| `menu_nav` | 816 | 1 | Splash, main menu walk, Options and Powerup Settings, Controls, Match Selection, Team Select, back out |
| `start_classic` | 2530 | 1 | Default 2-player Classic on `0smw`; player 1 runs, jumps, uses turbo and powerup keys; several stomps |
| `map_blockpiles` | 2530 | 7 | `SMW_MAP=Block Piles`, both players moving |

The scripts were produced by a small generator; edit the `.txt` directly and rerun `tools/make_golden.sh` to refresh the golden files.
