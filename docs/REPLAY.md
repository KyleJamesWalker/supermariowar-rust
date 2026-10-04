# Differential replay

The C++ game is rebuilt as a deterministic, headless, scriptable reference that writes one text block of game state per frame. The Rust port must read the same replay scripts and emit byte-identical dumps; `tools/diffreplay.py` reports the first frame where they diverge.

- Reference build: [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference), branch `harness-latest` (upstream `5693918f` plus the game and editor harness), checked out at `~/work/supermariowar-cpp-reference` and built into `build/`. `tools/cpp-harness.patch` and `tools/editor-harness.patch` are the same hooks as diffs against upstream.
- Scripts: `tools/replays/*.txt`. Golden output: `tools/golden/<script>/dump.txt` plus `frame_<n>.png` screenshots. A replay that tests a deliberate deviation from the C++ (`PROGRESS.md`) has a Rust golden in `tools/golden_rust/<script>/` instead (`RUST_GOLDEN=1 make_golden.sh`), which `parity.sh` uses and marks "(Rust golden)".

## Building the reference

```sh
git clone --branch harness-latest git@github.com:KyleJamesWalker/supermariowar-cpp-reference.git ~/work/supermariowar-cpp-reference
~/work/supermariowar-cpp-reference/build-reference.sh   # NO_NETWORK build of smw, smw-leveledit and smw-worldedit into build/
```

`tools/run_ref.sh`, `tools/run_editor.sh`, `tools/replay_compare.sh` and `tools/soak.py` default to that checkout (`SMW_REF_DIR`, `SMW_EDITOR_REF_DIR`, `SMW_CPP_BIN` or `--ref-dir` override it). The networking checks in `tools/ref/` expect a networking-on build of the same tree in `build-net/`.

Homebrew dependencies: `sdl2-compat`, `sdl2_image`, `sdl2_mixer`, `zlib`; CMake fetches toml11 (tested with sdl2-compat 2.32.72, Apple clang 21).

- `-ffp-contract=off` stops clang fusing `a*b+c` into FMA instructions on arm64. Rust never fuses, so without this flag float positions drift by an ulp.

## Running

```sh
tools/run_ref.sh tools/replays/start_classic.txt [out_dir]    # dump.txt + frame_<n>.bmp
tools/run_rust.sh tools/replays/start_classic.txt [out_dir]   # same, for the Rust port (builds --release first)
tools/parity.sh [replay.txt ...]                               # Rust vs golden for every replay, with a summary
tools/make_golden.sh [replay.txt ...]                          # regenerate tools/golden/
tools/diffreplay.py tools/golden/start_classic/dump.txt out/dump.txt
tools/diffreplay.py --shots tools/golden/start_classic out/
```

`run_ref.sh` takes `SMW_BIN` and `SMW_DATA_DIR` to run another binary and data tree (this is how `run_rust.sh` runs the port against the repository's `data/`), and exits with the game's status. It sets `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy`, and runs each replay in its own sandbox: a fresh temporary `HOME` and an APFS clone (`cp -c -R`) of the data tree with `maps/cache/mapsummary.txt` removed, passed as `--datadir`. Concurrent runs therefore never share a file the game writes. The dummy video driver works: the game draws into a 640x480 software surface and the renderer falls back to `software`. A 2,500-frame replay runs in about 3 s.

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

Rust only (see Recordings): `SMW_NO_RECORD=1` turns off session recording, `SMW_LIVE_SCRIPT=<file>` feeds a replay-format script into the live input path of a recorded session, `SMW_AUDIBLE=1` plays real sound alongside the virtual mixer, and `SMW_REPLAY_SPEED=<n>` scales the frame-limiter sleep.

## Frame loop

The frame counter `N` starts at 0 and counts iterations of the main loop in `gameloop()`:

1. Seeded runs: `sfx_virtual_advance()` (see Sound). Then push every replay event whose frame is `N` with `SDL_PushEvent`, in file order.
2. `currentState->update()`.
3. Write the dump block for `N`.
4. If `N` is in `SMW_SHOT_FRAMES`, `SDL_SaveBMP(screen)`: the 640x480 ARGB8888 surface as drawn by this frame, before `gfx_flipscreen()`.
5. `N += 1`; if `N >= SMW_FRAMES`, quit.
6. FPS limiter, `gfx_flipscreen()`.

Splash timeline with no input: frame 0 draws "Loading...", frame 1 loads all graphics and sounds, frame 2 onward shows "Press Any Key To Continue". A `menu_select`, `menu_cancel` or `menu_random` press enters the main menu on the frame it is pushed. Press on frame 2 or later: the key check runs before the frame-1 load, so an earlier press skips loading.

## Replay format

One event per line: `<frame> <down|up> <SDL key name>`. The key name is everything after the second field, passed to `SDL_GetKeyFromName` (`Up`, `Return`, `Escape`, `Right Ctrl`, `Right Shift`, `E`). Frames must be non-decreasing. Blank lines and lines starting with `#` are ignored. Unknown keys or malformed lines exit with status 2.

Pushed events are `SDL_KEYDOWN`/`SDL_KEYUP` with `keysym.sym` = the key, `keysym.scancode = SDL_GetScancodeFromKey(sym)`, `mod = KMOD_NONE`, `repeat = 0`, `windowID = 0`, `timestamp = 0`. The game reads only `sym` and `mod`.

Lines of the form `#@ key=value` are run parameters for `run_ref.sh` (and the Rust runner): `seed`, `frames`, `map`, `shots`, and `options`, an `options.bin` (path relative to the replay) copied into the sandbox HOME before the run. Session recordings also carry `options_b64` and `controls_b64`, the base64 bytes of the `options.bin` and `controls.sdl2.bin` the session started with, which `run_ref.sh` writes into the sandbox HOME. They are comments to the game itself.

A key press is a `down` and a later `up`. `fPressed` fires only on the down edge, so pressing the same key twice needs an `up` in between.

### Joystick events

`<frame> jaxis <dev> <axis> <value>`, `<frame> jbutton <dev> <button> <0|1>` and `<frame> jhat <dev> <hat> <value>` push `SDL_JOYAXISMOTION`, `SDL_JOYBUTTONDOWN`/`SDL_JOYBUTTONUP` (`state` `SDL_PRESSED`/`SDL_RELEASED`) and `SDL_JOYHATMOTION`, with `which = <dev>`, interleaved with key events in file order. Ranges: `dev` 0-7, axis 0-5 with value -32768..32767, button 0-15, hat 0 with value 0-15 (`SDL_HAT_UP` 1, `RIGHT` 2, `DOWN` 4, `LEFT` 8). The game binds a device by index and compares it with `which`, so `<dev>` is both. The Rust harness accepts axes and buttons 0-63, so a browser recording can keep every pad input; one that uses axis 6+ or button 16+ does not replay on the C++ harness, and `replay_compare.sh` says so and exits 2.

When the replay has joystick lines, `harness::init` attaches `max(dev) + 1` virtual joysticks (`SDL_JoystickAttachVirtual`, game-controller type, 6 axes, 16 buttons, 1 hat (Rust: 64 axes, 64 buttons, 1 hat), named "Virtual Controller") before `init_joysticks`, so `SDL_NumJoysticks`, the Controls device list and the `controls.sdl2.bin` device clamp see them. Replays without joystick lines attach none. The Rust harness sets the `SDL_JOYSTICK_HIDAPI`, `SDL_JOYSTICK_MFI` and `SDL_JOYSTICK_IOKIT` hints to 0 in any replay (`run_ref.sh` sets them for the C++ too), so a connected pad cannot join one. The browser's SDL has no virtual joysticks: a browser replay opens no pads, sets `joystickcount` to `max(dev) + 1` and names them "Virtual Controller", and its joystick events reach the game with `which = <dev>` as natively. `web_replay.mjs` also hides the browser's pads.

Binding a control in the Controls menu blocks in `MI_InputControlField::SendInput` until input arrives. With a replay loaded, that wait calls `harness::waitEvent` instead of `SDL_WaitEvent`:

1. It returns the next unconsumed replay event in file order, whatever its frame number, built exactly as `frameStart` would push it. The frame counter does not advance, as in the real game.
2. That event counts as consumed: the `frameStart` of its frame skips it.
3. If the replay has no events left, the harness prints `blocking wait at frame N but the replay has no events left` and exits with status 2.

Without a replay it is a plain `SDL_WaitEvent`. Pressing select on a control field at frame N therefore binds the first event after that press's `up`, e.g. `jbutton 0 6 1`.

The browser cannot block, so the Rust web build does not wait: each frame the field reads the input events the menu polled that frame and binds the first that fits, staying in "(Press Button)" until one does. Upstream's web build skips the wait but still loops, which hangs the page.

Joystick defaults (`GameValues.cpp`, device index != keyboard): game Left/Right = axis 0, Jump = button 0, Down = axis 1 +, Turbo/Item/Pause/Exit = buttons 1-4; menu Up/Down = axis 1, Left/Right = axis 0, select = button 0, cancel = button 1, random = button 2. `JOYSTICK_DEAD_ZONE` is 16384. Cancel on the main menu exits the game. The Rust port deviates here (see `PROGRESS.md`, Deliberate deviations): a stick 1 binding also fires from the hat and vice versa, and at launch connected joysticks go to the first players, so a replay with joystick lines starts with player 1 on joystick 0, player 2 on the arrow-key set and player 3 on the WASD set, all human.

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

## Recordings

The Rust game records every normal launch (no `SMW_REPLAY`, no `SMW_NO_RECORD`) to `~/Library/Preferences/.smw/replays/<UTC timestamp>.txt`, the settings directory that holds `options.bin`, and keeps the newest 10. A recording is an ordinary replay script, so `run_ref.sh` runs it on either build. The editors and replays never record.

- Header: `#@ seed=` (a random seed, applied exactly as `SMW_SEED`, so the live session is a seeded run), then `#@ options_b64=` and `#@ controls_b64=` when those files existed at startup. `#@ frames=` is appended at exit (normal quit, the window closing, or a caught crash); a killed process or a closed browser tab leaves it out. The browser build flushes the recording every 60 frames.
- Input capture: an SDL event filter holds back every OS input event. At each frame start the held keyboard and joystick events become replay lines (`down`/`up` with `SDL_GetKeyName`, `jaxis`/`jbutton`/`jhat` with the joystick's open index) and are pushed back exactly as a replay pushes them, so the game sees `mod = KMOD_NONE` and `repeat = 0` both live and in replay. Input the format cannot express is dropped while recording: mouse, touch, game-controller and text events, key-repeat flags, and modifiers (so Alt+Enter and Alt+F4 do nothing in a recorded session).
- Frame 0 gets one `0 jhat <dev> 0 0` line per joystick open at startup, so a replay attaches the same number of joysticks.
- In the browser, a standard-mapping gamepad reports its D-pad as buttons 12-15 (up, down, left, right) and has no hat, where desktop SDL reports hat 0. Each of those button lines is preceded by the `jhat <dev> 0 <value>` line the four buttons imply, so binding a control with the D-pad takes the hat ("Pad Up" and so on) as it does natively.
- A blocking wait (control binding) returns the next held input event and writes it with the next frame number, which is how a replay's `waitEvent` consumes it.
- Window close: `#@ frames=` is the frame the close arrived in, so the replay ends where the live session stopped processing input.
- Sound: the virtual mixer drives every game-visible sound state, as in replays, and SDL_mixer also plays the same commands as output only (results ignored, no finished callbacks). The device is opened at 44100 Hz, S16, stereo with no format changes allowed, so sound lengths match the headless dummy driver; with no usable device the game falls back to the dummy driver.
- `SMW_LIVE_SCRIPT=<file>` pushes a replay script's events into SDL's queue at their frames, with the `KMOD_NUM` modifier and some repeat flags a real keyboard sends, so they take the live capture path. A scripted live session's `SMW_DUMP` equals its recording's dump under `run_rust.sh`.

Watching: `smw --replay <file> [--replay-speed <n>]` plays a recording in a normal window at normal speed (or `n` times faster) with sound. It writes the embedded settings into a throwaway HOME, removed at exit, so the user's settings are untouched, and quits after `#@ frames=`, or, when that line is missing, 188 frames (about 3 s) after the last input. In the app bundle: `open "dist/Super Mario War.app" --args --replay /absolute/path/to/recording.txt`.

Bug check: `tools/replay_compare.sh <file> [out_dir]` runs a recording headless on the C++ reference (`SMW_CPP_BIN`, default `~/work/supermariowar-cpp-reference/build/smw`) and on the Rust build, prints the first divergent frame with `diffreplay.py` context, and saves C++ and Rust screenshots of the frames around it.

## Dump format

Plain ASCII, `\n` line endings, one block per frame. Fields are separated by single spaces. Lines appear in exactly this order:

```
F <frame> <state>
M <menu> focus=<i> modifying=<0|1>
G mode=<m> gameover=<0|1> winner=<w>
P id=<id> team=<t> ix=<ix> iy=<iy> fx=<f> fy=<f> velx=<f> vely=<f> state=<s> score=<n> powerup=<p> inair=<0|1>
O noncol=<n> obj0=<n> obj1=<n> obj2=<n> ec0=<n> ec1=<n> ec2=<n>
S <sound event>
C <kind> <fields>
R calls=<n> last=<u32>
```

| Record | When | Fields |
|---|---|---|
| `F` | always | `<state>` is `splash`, `menu`, `gameplay` or `other`, from `GameStateManager::currentState` |
| `M` | state `menu` | `<menu>` is `MenuState::mCurrentMenu`, named in the table below. `focus` is the index of `UI_Menu::m_currentFocus` in `controls` (insertion order of `AddControl`/`AddNonControl`), or -1. `modifying` is `UI_Menu::fModifyingItem`. |
| `G` | state `gameplay` | `mode` = `(int)game_values.gamemode->gamemode` (`GameModeType`: classic 0, frag 1, ... ; bonus 999, minigames 1000-1002). `gameover`, `winner` = `gamemode->gameover`, `gamemode->winningteam`. |
| `P` | state `gameplay` | one line per entry of the global `players` vector, in vector order. `id` = `globalID`, `team` = `teamID`, `ix`/`iy` shorts, `fx`/`fy`/`velx`/`vely` floats, `state` = `(int)PlayerState` (Waiting 0, Spawning 1, Dead 2, Ready 3, EnteringWarpUp 4 ... LeavingWarpRight 11), `score` = `score->score` (0 if null), `powerup` short, `inair` bool. |
| `O` | state `gameplay` | element counts of `noncolcontainer`, `objectcontainer[0..2]`, `eyecandy[0..2]` at the end of the frame (dead objects not yet cleaned count). |
| `S` | seeded runs | one line per sound command or virtual-mixer event since the previous block, in the order they happened (see Sound). Events before frame 0 land in frame 0. |
| `C` | net games, Rust only | one line per random outcome, death or score change since the previous block. `powerup type=<t> x=<x> y=<y>`: a `createpowerup` call, with the position passed in (a powerup block's top-left corner). `<event> args=<a,...> out=<...>`: one `net_random::Ev` (`src/smw/net_random.rs`), its arguments, the objects it added (`<objectType>/<movingObjectType>@<x>,<y>`, `powerup<t>`) and its result (a position, a timer, a player's spot and state). `death p=<id> style=<s> removed=<0|1>`: a `CPlayer::die` call, or (`removed=1`) a player a team removal took out. `scores <team scores>` and `gameover winner=<team>`: the team scores and the end of the game, written at the end of a frame when they changed. `net_game_compare.py --spawns` compares them across clients. Never written outside a net game, so single-player dumps match the C++ goldens. |
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

## Sound

Under `SMW_SEED`, `sfx` runs a virtual mixer instead of SDL_mixer playback, so channel allocation, `sfxSound::isPlaying()`, `sfxMusic::isPlaying()` and the music-finished hook depend only on the frame counter. Nothing reaches the audio device.

- Clock: `sfx_ticks()`, the virtual `1000 + N * 16` ms.
- Channels: 16. `Mix_PlayChannel(-1, ...)` takes the lowest free channel, or returns -1 when all are busy or the chunk is NULL. A channel ends at `start + alen * 1000 / bytes_per_second * (loops + 1)` ms (integer division; `bytes_per_second` = frequency x channels x sample bytes from `Mix_QuerySpec`); `loops = -1` never ends. `Mix_HaltChannel` frees the channel and calls `sfxSound::onChannelFinished` at once, as SDL_mixer does. Freeing a chunk frees its channels without the callback.
- Music: one track. `play` starts it unpaused; `once=1` ends `Mix_MusicDuration * 1000` ms later (truncated), `once=0` or an unknown duration never ends. `stop` and freeing the playing `Mix_Music` halt it without the hook. `togglePause` freezes and resumes the remaining time.
- `harness::frameStart()` calls `sfx_virtual_advance()` before pushing events: every channel whose end time has passed finishes in channel order (`onChannelFinished`), then finished music calls `musicfinished()` on the main thread.

`S` lines (`<name>` is the file path after the data directory, e.g. `sfx/packs/Classic/jump.wav`; empty for a sound that never loaded):

| Line | Source |
|---|---|
| `S play <name> ch=<c>` | `sfxSound::play()` past the 40 ms throttle; `c` is the virtual channel or -1 |
| `S skip <name>` | `sfxSound::play()` refused by the throttle |
| `S loop <name> loops=<n> ch=<c>` | `sfxSound::playLoop()` |
| `S stop <name>` | `sfxSound::stop()` |
| `S haltall` | `sfx_stopallsounds()` |
| `S done ch=<c>` | channel `c` reached its end time |
| `S music <name> once=<0\|1> resume=<0\|1>` | `sfxMusic::play()` |
| `S musicstop <name>` | `sfxMusic::stop()` |
| `S musicpause <name> paused=<0\|1>` | `sfxMusic::togglePause()`; `paused` is the new state |
| `S musicdone` | the playing track ended; `musicfinished()` runs next |
| `S soundvolume <v>` / `S musicvolume <v>` | `sfx_setsoundvolume()` / `sfx_setmusicvolume()` |

## SMW_MAP

Applied at the end of `MenuState::init()`, which runs when the splash screen is dismissed (after `MapList::ReadFilters()` has reset the current map to the first one). The name is matched with `MapList::findexact(name, false)`, a case-insensitive match on the map key (file name without the `creator_` prefix and `.map`, e.g. `Block Piles` for `4matsy_Block Piles.map`). If that fails, `MapList::find(name)` matches a substring of the path. Then `miMapField->LoadCurrentMap()` refreshes the game settings preview. An unknown name exits with status 2. Without `SMW_MAP` the first map in sorted order (`0smw`) is current.

## Nondeterminism sources

| Source | Where | Handling |
|---|---|---|
| Well512 seeded from `srand(time(0))` + `rand()` in its constructor | `RandomNumberGenerator.cpp` | `SMW_SEED` reseeds via `Well512::initialize(seed)` (state[0] = seed, then LCG `state[i] = (state[i-1] * 1103515245 + 12345) & 0x7fffffff`, index 0) at the start of `main_game()`, before `create_globals()`. |
| `srand(time(NULL))` | `main.cpp` | Replaced by `srand(seed)` when seeded. Nothing in the game calls libc `rand()` afterwards (`SystemRandomNumberGenerator` is unused). |
| `sfxSound::play()` refuses a retrigger within 40 ms of `SDL_GetTicks()`, and its return value drives announcer selection in `PlayerAwardEffects` | `sfx.cpp` | Seeded runs use a virtual clock: `1000 + N * 16` ms (`WAITTIME`), where `N` is the frame counter. |
| `Mix_PlayChannel` fails when all 16 channels are busy, which depends on the audio thread | `sfx.cpp` | Seeded runs use the virtual mixer (see Sound), and still treat a failed `play()` as success (the throttle timestamp updates). |
| Channel-finished and music-finished callbacks on the audio thread drive `isPlaying()`, which gameplay reads (skid, flying, invincibility and clock sounds, music restarts) | `sfx.cpp`, `GSGameplay.cpp` | Seeded runs use the virtual mixer; the callbacks run on the main thread from `frameStart()`. |
| Frame limiter reads `SDL_GetTicks()` | `FPSLimiter.cpp` | Wall clock only controls sleeping and the F1 FPS overlay. `SMW_NOLIMIT` skips the sleeps. |
| `options.bin` / `controls.sdl2.bin` in `$HOME/Library/Preferences/.smw/` | `GameValues.cpp` | `run_ref.sh` points `HOME` at an empty temp directory, so defaults from `CGameValues::init` apply. The game writes `options.bin` there on exit. |
| Map summary cache `data/maps/cache/mapsummary.txt` written on exit | `MapList.cpp` | Holds only precomputed auto-filter flags. Each run gets its own data clone without it. |
| OS events (window, joystick hot-plug, real keyboard) | `SDL_PollEvent` | Not filtered. The dummy video driver generates no input and no joysticks are attached. |
| Screenshot file names (`Insert` key) and netplay timestamps | `gfxSDL.cpp`, `net.cpp` | Unreachable: no script presses `Insert`, and the build uses `NO_NETWORK`. |
| FMA contraction on arm64 | compiler | `-ffp-contract=off`. |

Map list order is deterministic: maps live in a `std::multimap` keyed by name.

## C++ changes

`tools/cpp-harness.patch` (apply with `patch -p1` in the repo root):

- `src/smw/Harness.{h,cpp}`: env parsing, event injection, dump, screenshots, frame cap.
- `main.cpp`: `harness::init()` at the top of `main_game()`, `harness::frameStart()`/`frameEnd()` around `update()`, seeded `srand`.
- `FPSLimiter.cpp`: early return under `SMW_NOLIMIT`.
- `RandomNumberGenerator`: `callCount()`, `lastValue()`, `resetCallCount()`.
- `sfx`: `sfx_ticks` clock hook, `sfx_ignore_channel_failure`, the virtual mixer (`sfx_virtual_mixer`, `sfx_virtual_advance()`), `sfx_events` for `S` lines, and the data-relative file name on `sfxSound`/`sfxMusic`.
- `uimenu`: `UI_Menu::currentFocusIndex()`. `GSMenu`: menu name/focus accessors and `SMW_MAP`. `player.h`: `friend struct HarnessAccess`.

## Golden outputs

`tools/golden/<script>/` holds `dump.txt` (or `dump.txt.gz`; `diffreplay.py` reads both) and `frame_<n>.png`.
`make_golden.sh` writes gzipped dumps.

Hand-written scripts:

| Script | Frames | Seed | Content |
|---|---|---|---|
| `menu_nav` | 816 | 1 | Splash, main menu walk, Options and Powerup Settings, Controls, Match Selection, Team Select, back out |
| `start_classic` | 2530 | 1 | Default 2-player Classic on `0smw`; player 1 runs, jumps, uses turbo and powerup keys; several stomps |
| `map_blockpiles` | 2530 | 7 | `SMW_MAP=Block Piles`, both players moving |
| `frenzy_cpu` | 2780 | 42 | Players 3 and 4 set to CPU in the main menu, Frenzy picked in the game settings Mode field, `Block Piles` |

Generated by `tools/gen_replays.py` (rerun it, then `make_golden.sh`, after editing):

| Scripts | Frames | Content |
|---|---|---|
| `cpu_<mode>` (22) | 3600 | All four players set to CPU in the Players field, the mode picked in the Mode field, one per game mode, maps rotating through 12 maps with platforms, hazards and warps |
| `flow_tournament`, `flow_tour`, `flow_world` | 6000, 5000, 6000 | Tournament (2 wins), the first Tour and the first World, reached through the Match field; CPUs play the stages |
| `mini_pipe`, `mini_hammerboss`, `mini_boxes` | 3000 | Minigames picked in the Match and Game fields |
| `fuzz_a`, `fuzz_b`, `fuzz_c` | 5000 | Two human players with seeded random presses and holds of every game key |

Every flow is reachable through the menus, so no extra environment override is needed. The bonus house only
occurs inside World maps and is not covered yet.
