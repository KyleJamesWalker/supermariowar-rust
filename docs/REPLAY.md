# Differential replay

The C++ game is rebuilt as a deterministic, headless, scriptable reference that writes one text block of game state per frame. The Rust port must read the same replay scripts and emit byte-identical dumps; `tools/diffreplay.py` reports the first frame where they diverge.

- Reference build: [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference), branch `harness-latest` (upstream `c7056790` plus the game and editor harness), checked out at `~/work/supermariowar-cpp-reference` and built into `build/`. `tools/cpp-harness.patch` and `tools/editor-harness.patch` are the same hooks as diffs against upstream.
- Scripts: `tools/replays/*.txt`. Golden output: `tools/golden/<script>/dump.txt` plus `frame_<n>.png` screenshots. A replay that tests a deliberate deviation from the C++ (`PROGRESS.md`) has a Rust golden in `tools/golden_rust/<script>/` instead (`RUST_GOLDEN=1 make_golden.sh`), which `parity.sh` uses and marks "(Rust golden)".

## Building the reference

```sh
git clone --branch harness-latest git@github.com:KyleJamesWalker/supermariowar-cpp-reference.git ~/work/supermariowar-cpp-reference
~/work/supermariowar-cpp-reference/build-reference.sh   # NO_NETWORK build of smw, smw-leveledit and smw-worldedit into build/
```

`tools/run_ref.sh`, `tools/run_editor.sh`, `tools/replay_compare.sh` and `tools/soak.py` default to that checkout (`SMW_REF_DIR`, `SMW_EDITOR_REF_DIR`, `SMW_CPP_BIN` or `--ref-dir` override it). The networking checks in `tools/ref/` expect a networking-on build of the same tree in `build-net/`.

Homebrew dependencies: `sdl2-compat`, `sdl2_image`, `sdl2_mixer`, `zlib`; CMake fetches toml11 (tested with sdl2-compat 2.32.72, Apple clang 21).

- `-DSMW_NO_RLE=ON` (passed by `build-reference.sh`; branch `harness-rle` until merged) builds without `SDL_SetSurfaceRLE`, like the port's native default. The goldens come from this build.
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
| `SMW_SHOT_EVERY=<n>` | Also save every nth frame, counting from the start of `SMW_SHOT_RANGE` (default 0). |
| `SMW_SHOT_RANGE=<a>-<b>` | Limit `SMW_SHOT_EVERY` to frames `a..=b` (`a-` runs to the end); alone, it saves every frame in the range. `SMW_SHOT_FRAMES` still applies outside it. |
| `SMW_SHOT_STREAM=<file>` | Append the frames `SMW_SHOT_EVERY`/`SMW_SHOT_RANGE` select to this file (or pipe) as raw 640x480 ARGB8888 rows, 1,228,800 bytes each, instead of writing BMPs. `SMW_SHOT_FRAMES` still writes BMPs. |
| `SMW_MAP=<name>` | Select the start map when the menu is created (see below). |

Rust only (see Recordings and "Markers, checkpoints and clips"): `SMW_SEGMENT=<k>` replays only match k from its checkpoint, `SMW_RECORD_TO=<file>` writes a replay's events, markers and checkpoints to a new recording, `SMW_NO_RECORD=1` turns off session recording, `SMW_LIVE_SCRIPT=<file>` feeds a replay-format script into the live input path of a recorded session, `SMW_AUDIBLE=1` plays real sound alongside the virtual mixer, `SMW_REPLAY_SPEED=<n>` scales the frame-limiter sleep, and `SMW_RLE=0`/`1` turns SDL surface RLE off or on (default: off natively, on in the web build).

## Frame loop

The frame counter `N` starts at 0 and counts iterations of the main loop in `gameloop()`:

1. Seeded runs: `sfx_virtual_advance()` (see Sound). Then push every replay event whose frame is `N` with `SDL_PushEvent`, in file order.
2. `currentState->update()`.
3. Write the dump block for `N`.
4. If `N` is in `SMW_SHOT_FRAMES` (or selected by `SMW_SHOT_EVERY`/`SMW_SHOT_RANGE`), `SDL_SaveBMP(screen)`: the 640x480 ARGB8888 surface as drawn by this frame, before `gfx_flipscreen()`. With `SMW_SHOT_STREAM`, the periodic frames go to the stream instead.
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

`<frame> jadd <dev>` and `<frame> jremove <dev>` (Rust only, not in the C++) connect and disconnect joystick `<dev>` before frame `<frame>` runs: a pad hot-plugged in a session with `SMW_PAD_TRANSLATE=1` (`PROGRESS.md`, Deliberate deviations). A device whose first line is `jadd` is left out of the joysticks attached at launch; `jadd` attaches a virtual joystick as index `<dev>` (in the browser, it raises `joystickcount`), `jremove` closes it, and once no match is on the players are reassigned as at launch, as in the session. A segment replay attaches every joystick the replay uses at launch and ignores these lines, since a match never reassigns players. `replay_compare.sh` refuses recordings with them (exit 2).

`#@ touch=1` (Rust only) marks a session recorded with Android's touch controls, which change how `assign_inputs` gives out players (`android/README.md`, Touch controls); a replay of it assigns them the same way. The touch controls themselves record as ordinary key lines.

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

A `.smwrp` ("Super Mario War replay") is a recording gzipped, nothing more; `gunzip -c f.smwrp` or `zcat < f.smwrp` reads it, and `.txt.gz` is the same bytes. The web page downloads `.smwrp`, and `smw --replay`, `SMW_REPLAY`, the web page's Load replay and the tools (`run_ref.sh`, `run_rust.sh`, `replay_compare.sh`, `replay_clip.py`, `segment_check.py`, `replay_video.py`, `web_replay.mjs`) read plain or gzipped files, told apart by the gzip magic bytes. `run_ref.sh` gunzips into its sandbox, so the C++ harness only sees text.

- Header: `#@ seed=` (a random seed, applied exactly as `SMW_SEED`, so the live session is a seeded run), `#@ netrec=1` (see "Online matches"), then `#@ options_b64=`, `#@ controls_b64=` and `#@ servers_b64=` (`servers.toml`, the player name and server list) when those files existed at startup. `#@ frames=` is appended at exit (normal quit, the window closing, or a caught crash); a killed process or a closed browser tab leaves it out. The recording is flushed every 60 frames, so a killed session keeps what it recorded.
- Input capture: an SDL event filter holds back every OS input event. At each frame start the held keyboard and joystick events become replay lines (`down`/`up` with `SDL_GetKeyName`, `jaxis`/`jbutton`/`jhat` with the joystick's open index) and are pushed back exactly as a replay pushes them, so the game sees `mod = KMOD_NONE` and `repeat = 0` both live and in replay. Input the format cannot express is dropped while recording: mouse, touch, game-controller and text events, key-repeat flags, and modifiers (so Alt+Enter and Alt+F4 do nothing in a recorded session).
- With `SMW_PAD_TRANSLATE=1` (the muOS launcher), a game controller's input is recorded after `pad.rs` rewrites it as joystick events (`PROGRESS.md`, Deliberate deviations), so a replay pushes the translated events and never translates again.
- Frame 0 gets one `0 jhat <dev> 0 0` line per joystick open at startup, so a replay attaches the same number of joysticks. A pad connected or disconnected later (`SMW_PAD_TRANSLATE=1` only) gets a `jadd`/`jremove` line at the frame the game noticed it, so the replay connects it at the same frame and reassigns the players at the same point (`pad_hotplug` checks this).
- In the browser, a standard-mapping gamepad reports its D-pad as buttons 12-15 (up, down, left, right) and has no hat, where desktop SDL reports hat 0. Each of those button lines is preceded by the `jhat <dev> 0 <value>` line the four buttons imply, so binding a control with the D-pad takes the hat ("Pad Up" and so on) as it does natively.
- A blocking wait (control binding) returns the next held input event and writes it with the next frame number, which is how a replay's `waitEvent` consumes it.
- Window close: `#@ frames=` is the frame the close arrived in, so the replay ends where the live session stopped processing input.
- Sound: the virtual mixer drives every game-visible sound state, as in replays, and SDL_mixer also plays the same commands as output only (results ignored, no finished callbacks). The device is opened at 44100 Hz, S16, stereo with no format changes allowed, so sound lengths match the headless dummy driver; with no usable device the game falls back to the dummy driver.
- `SMW_LIVE_SCRIPT=<file>` pushes a replay script's events into SDL's queue at their frames, with the `KMOD_NUM` modifier and some repeat flags a real keyboard sends, so they take the live capture path. A scripted live session's `SMW_DUMP` equals its recording's dump under `run_rust.sh`.

### Online matches

Rust only. A recording with `#@ netrec=1` also holds everything its client received over the network, so a net match replays offline, with no lobby server, relay or other client. Each client's recording replays its own view of the match.

- `#@ net frame=<n> side=<c|h> poll=<k> peer=<host>:<port>:<player id>:<peer key> <event>`: one event a listener got in frame n, in order. `side=c` is the client's listener (lobby server or foreign game host), `side=h` the game host it runs when it hosts; `poll=k` is the side's k-th poll in that frame (from 0), since a frame may poll a listener more than once. The event is `connect`, `disconnect` or `recv=<base64 of the message>`. The peer key tells peers apart as `NetPeer::same_peer` does.
- `#@ netv frame=<n> <key>=<value>`: a value the game read from outside it: `client_restart`, `gamehost_restart`, `connect_lobby` and `connect_game_host` (the network layer's results, `1` or `0`), `rtt` (a peer's round-trip time), `time` (the host's sync seed, from `time()`) and `relay` (the web build's relay URL, `-` for none).

A replay of such a recording never touches the network layer: no restarts, connects, NAT punches or shutdowns. Each listener poll gets the recorded events of that frame, side and poll, and each value read gets the next recorded value of its key; a missing value stops the replay. Sends go nowhere. Recordings without `#@ netrec=1` replay with the live network, as before. Net timestamps (the start barrier, the last sent and received messages) use the virtual clock in seeded runs, so the replay reads the times the session read. `crates/smw-core/src/smw/network/net_record.rs` has the code.

Watching: `smw --replay <file> [--replay-speed <n>] [--segment <k>]` plays a recording (or only its match k, see "Markers, checkpoints and clips") in a normal window at normal speed (or `n` times faster) with sound. It writes the embedded settings into a throwaway HOME, removed at exit, so the user's settings are untouched, and quits after `#@ frames=`, or, when that line is missing, 188 frames (about 3 s) after the last input. In the app bundle: `open "dist/Super Mario War.app" --args --replay /absolute/path/to/recording.txt`.

Bug check: `tools/replay_compare.sh <file> [out_dir]` runs a recording headless on the C++ reference (`SMW_CPP_BIN`, default `~/work/supermariowar-cpp-reference/build/smw`) and on the Rust build, prints the first divergent frame with `diffreplay.py` context, and saves C++ and Rust screenshots of the frames around it.

## Markers, checkpoints and clips

Rust only. A recording marks every change of game state, and saves a checkpoint where each match starts, so one match can be replayed, watched or shared without the frames before it. `SMW_RECORD_TO=<file>` makes a replay write the same recording: its own events, markers and checkpoints, under a header taken from the sandbox HOME (`seed`, `netrec`, `options_b64`, `controls_b64`, `servers_b64`, `map`), and its net events and values.

Markers are `#@ mark frame=<n> state=<state> ...` lines, written after frame n's events when the state differs from the previous frame's. States: `splash`, `menu`, `worldmap` (the World menu), `gameplay`, `scoreboard` (gameplay after the game is over) and `other`. Fields are `key=value`; a value with a space is double-quoted.

| Marker | Fields |
|---|---|
| a match starts (`gameplay` after anything else) | `match=<k>` (1, 2, ... in the session), `type` (`single`, `tournament`, `tour`, `minigame`, `world`, `quick`, `online`), `mode` (the mode's name in lower case with `_` for spaces), `style` (Star only: `ztar`, `shine`, `multi`, `random`), `goal`, `map`, `file` (the map file relative to the data directory, or to the settings directory as `~settings/<file>`, as for a net game's joiner), Tournament `game` and `wins_needed`, Tour or World `tour`/`world` and `stop=<i>/<n>`, `players`, and per playing slot `p<i>=<control>,team<t>,<skin>`, where control is `keys<s>` (keyboard set s), `pad<d>` (joystick d) or `cpu-<difficulty>` |
| `scoreboard`, and the marker that leaves the match | `match=<k>`, `scores` (the team scores), and once the game is over `winner=team<t>` or `winner=tie` |

```
#@ mark frame=2286 state=gameplay match=1 type=single mode=star style=ztar goal=5 map=2skyfight file=maps/2skyfight.map players=4 p1=pad0,team1,BubBob p2=cpu-moderate,team2,BlackMage p3=cpu-moderate,team3,0smw p4=cpu-moderate,team4,0smw
#@ checkpoint match=1 frame=2286 z64=eNrtWFtv...
#@ mark frame=3931 state=menu match=1 scores=5,5,5,5
```

A checkpoint follows its match's start marker: `#@ checkpoint match=<k> frame=<n> z64=<data>`, the base64 of the zlib stream of a versioned binary (`SMWC`, version 1, or version 2 for an online match; `crates/smw-core/src/smw/checkpoint.rs`). Older recordings hold the binary uncompressed as `b64=<data>`, which every reader still accepts; any other field is an error. It is saved in `MenuState::enter_gameplay` once the map and its music are loaded, and holds everything the match reads from earlier frames:

- the settings, as the options.bin and controls.sdl2.bin bytes they would be written as now; which input configuration each player reads; the stick and hat directions held;
- `game_values` outside those files: match type, teams, tournament, tour and world state, stored and world powerups, the mode settings in effect, colors, timers and flags the menus set;
- the game mode object, the boss type and the minigame goals; the tour or world stops; the score boards; the carry-over fields of the gameplay state;
- the map file, the map list and music list positions, every music track and sound (its file, the channels it plays on and its retrigger time), the virtual mixer;
- the RNG state, index, call count and last value, and the sound commands frame n has logged so far;
- online (version 2, first): the files the host sent (the joiner's `net_last.map`, the other players' `net_skin<k>.bmp`), the session (`netplay`), the client's and its game host's message state and peers as addresses, and the `net_random` and `net_outcomes` state. A segment replay writes those files into the settings directory, restores the peers as replayed peers and plays the match's `#@ net` events.

Matches without one get `#@ checkpoint match=<k> frame=<n> unsupported=<reason>`: `bonus_house` (a World's bonus house, which no replay covers yet). Recordings made before online checkpoints have `online`; the web page shows it as "online: no clip".

Segment replays: `SMW_SEGMENT=<k>`, `smw --replay <file> --segment <k>`, or a `#@ segment=<k>` line in the file, play match k only. The frame counter starts at the checkpoint's frame n, the events of frames up to n are skipped (the menu read them before the checkpoint), and the replay ends before the first marker after n whose state is neither `gameplay` nor `scoreboard` (a lower `SMW_FRAMES` still applies). Frame n loads the game data as the splash screen does, initializes the menus, restores the checkpoint and runs the rest of `enter_gameplay`. Its dump is byte-identical to the full replay's frames n up to the end, frame numbers and `R` counters included; only the screen of frame n differs (the full replay still shows the faded-out menu). A segment replay never records.

Clips: `tools/replay_clip.py <recording> --match <k> -o clip.txt` writes the recording's header, `#@ segment=<k>`, match k's markers and checkpoint, a `0 jhat <d> 0 0` line per joystick the recording used (so the same joysticks attach), match k's input lines, its `#@ net` and `#@ netv` lines and `#@ frames=<end>`. It plays with `smw --replay`, `run_rust.sh`, `web_replay.mjs`, `replay_video.py` and the web page, and dumps the same frames as the full recording. `--list` prints the session's matches. The web page's start screen lists the last session's matches from its markers, each with Watch and a clip download (`web/shell.html` cuts clips the same way). A clip only plays on this port: the C++ harness skips every `#` line, so it would start a clip from frame 0.

`tools/segment_check.py [--web] <file> ...` checks the exactness: it replays each file whole (recording checkpoints with `SMW_RECORD_TO` when it has none), then for every checkpoint compares the full dump from the match's first frame to the frame before it left gameplay with `SMW_SEGMENT=k` on the recording, with the clip, and with `--web` the clip in the browser build. `tools/segment_replays/` holds multi-match sessions for it (`gen_replays.py segments`) and a browser recording of a gamepad Ztar game. `tools/checkpoint_fixtures/` holds a recording of that game with its checkpoint in each format.

Compatibility: the C++ harness and `run_ref.sh` ignore `#@ mark` and `#@ checkpoint` lines, so a recording with markers replays on the reference as before, and recordings without markers replay as before but offer no segments. Replays never record unless `SMW_RECORD_TO` is set, so the parity goldens are unaffected. A checkpoint names sounds, tracks and maps by their data-relative files, so it needs the same data tree, and a build refuses a checkpoint of another format version.

## Dump format

Plain ASCII, `\n` line endings, one block per frame. Fields are separated by single spaces. Lines appear in exactly this order:

```
F <frame> <state>
M <menu> focus=<i> modifying=<0|1>
G mode=<m> gameover=<0|1> winner=<w>
P id=<id> team=<t> ix=<ix> iy=<iy> fx=<f> fy=<f> velx=<f> vely=<f> state=<s> score=<n> powerup=<p> inair=<0|1>
T type=<t> holders=<id>,<id>,<id>
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
| `T` | state `gameplay`, Star mode | `type` = `(int)CGM_Star::iCurrentModeType` (`StarStyle`: Ztar 0, Shine 1, Multi 2). `holders` = the `globalID` of `starPlayer[0..2]`, -1 for an empty slot. Ztar and Shine use slot 0; Multi uses one slot per star (players - 1). |
| `O` | state `gameplay` | element counts of `noncolcontainer`, `objectcontainer[0..2]`, `eyecandy[0..2]` at the end of the frame (dead objects not yet cleaned count). |
| `S` | seeded runs | one line per sound command or virtual-mixer event since the previous block, in the order they happened (see Sound). Events before frame 0 land in frame 0. |
| `C` | net games, Rust only | one line per random outcome, death or score change since the previous block. `powerup type=<t> x=<x> y=<y>`: a `createpowerup` call, with the position passed in (a powerup block's top-left corner). `<event> args=<a,...> out=<...>`: one `net_random::Ev` (`crates/smw-core/src/smw/net_random.rs`), its arguments, the objects it added (`<objectType>/<movingObjectType>@<x>,<y>`, `powerup<t>`) and its result (a position, a timer, a player's spot and state). `death p=<id> style=<s> removed=<0|1>`: a `CPlayer::die` call, or (`removed=1`) a player a team removal took out. `scores <team scores>` and `gameover winner=<team>`: the team scores and the end of the game, written at the end of a frame when they changed. `net_game_compare.py --spawns` compares them across clients. Never written outside a net game, so single-player dumps match the C++ goldens. |
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

Under `SMW_SEED`, `sfx` runs a virtual mixer instead of SDL_mixer playback, so channel allocation, `sfxSound::isPlaying()`, `sfxMusic::isPlaying()` and the music-finished hook depend only on the frame counter. Nothing reaches the audio device. The Rust game runs the virtual mixer in every run, live ones included: the audio service (`smw_sdl2::Sdl2Audio`) plays the same commands as output only, on the channel the virtual mixer chose, and a headless replay sends it nothing.

- Clock: `sfx_ticks()`, the virtual `1000 + N * 16` ms (`SDL_GetTicks()` in an unseeded run).
- Channels: 16. `Mix_PlayChannel(-1, ...)` takes the lowest free channel, or returns -1 when all are busy or the chunk is NULL. A channel ends at `start + alen * 1000 / bytes_per_second * (loops + 1)` ms (integer division; `bytes_per_second` = frequency x channels x sample bytes from `Mix_QuerySpec`); `loops = -1` never ends. The port reads that length, and each track's `Mix_MusicDuration`, from `crates/smw-core/src/common/sfx_durations.txt` (SDL2_mixer at 44100 Hz, S16, stereo; a unit test keeps it equal to SDL2_mixer). It asks the audio service only for files the table does not list. `Mix_HaltChannel` frees the channel and calls `sfxSound::onChannelFinished` at once, as SDL_mixer does. Freeing a chunk frees its channels without the callback.
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
| Screenshot file names (`Insert` key) and netplay timestamps | `gfxSDL.cpp`, `net.cpp` | Unreachable in the reference: no script presses `Insert`, and the build uses `NO_NETWORK`. The port's net code reads the virtual clock in seeded runs, counts player data stores instead of timing them, and records what the network delivers (see "Online matches"). |
| FMA contraction on arm64 | compiler | `-ffp-contract=off`. |
| `sinf`, `cosf` and `atan2f` round differently per libm (macOS, glibc, Emscripten's musl; clang may also merge a sin and cos into `__sincosf_stret`) | libm | Both builds use CORE-MATH's correctly rounded functions: `src/common/core-math/` in the reference, `crates/smw-core/src/common/math/trig.rs` in the port. |
| A thrown flag's `owner_throw` still points at a player `CleanDeadPlayers` deleted, and a flag base reads it | `CO_Flag`, `MO_FlagBase` | The Rust port clears it when the player is removed (see `PROGRESS.md`, Deliberate deviations). |

Map list order is deterministic: maps live in a `std::multimap` keyed by name.

## C++ changes

`tools/cpp-harness.patch` (apply with `patch -p1` in the repo root):

- `src/smw/Harness.{h,cpp}`: env parsing, event injection, dump, screenshots, frame cap.
- `main.cpp`: `harness::init()` at the top of `main_game()`, `harness::frameStart()`/`frameEnd()` around `update()`, seeded `srand`.
- `FPSLimiter.cpp`: early return under `SMW_NOLIMIT`.
- `RandomNumberGenerator`: `callCount()`, `lastValue()`, `resetCallCount()`.
- `sfx`: `sfx_ticks` clock hook, `sfx_ignore_channel_failure`, the virtual mixer (`sfx_virtual_mixer`, `sfx_virtual_advance()`), `sfx_events` for `S` lines, and the data-relative file name on `sfxSound`/`sfxMusic`.
- `CMakeLists.txt`, `gfx.cpp`, `gfxFont.cpp`, `gfxSprite.cpp`: the `SMW_NO_RLE` option.
- `Star.cpp`, `GameMode.cpp`: the multi star fixes from upstream PR branch `fix/memory-safety` (`PROGRESS.md`, Deliberate deviations).
- `common/core-math/`, `CMakeLists.txt` and every game `sin`/`cos`/`atan2` call: CORE-MATH's `cr_sinf`, `cr_cosf` and `cr_atan2f` (see Nondeterminism sources).
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
| `star_multi_2p`, `_3p`, `_4p` | 3600, 3600, 4800 | 501-503 | 2, 3 and 4 CPUs, Star mode with Star Type Multi Star, `Wacky Woods`. Regression replays for the multi star fixes (upstream `a272c93f` and `c7056790`, `UPSTREAM_BACKLOG.md` M2 and L5): 2p and 3p include steals whose holder holds for hundreds of frames in the `T` line |

Generated by `tools/gen_replays.py` (rerun it, then `make_golden.sh`, after editing):

| Scripts | Frames | Content |
|---|---|---|
| `cpu_<mode>` (22) | 3600 | All four players set to CPU in the Players field, the mode picked in the Mode field, one per game mode, maps rotating through 12 maps with platforms, hazards and warps |
| `flow_tournament`, `flow_tour`, `flow_world` | 6000, 5000, 6000 | Tournament (2 wins), the first Tour and the first World, reached through the Match field; CPUs play the stages |
| `mini_pipe`, `mini_hammerboss`, `mini_boxes` | 3000 | Minigames picked in the Match and Game fields |
| `fuzz_a`, `fuzz_b`, `fuzz_c` | 5000 | Two human players with seeded random presses and holds of every game key |

Every flow is reachable through the menus, so no extra environment override is needed. The bonus house only
occurs inside World maps and is not covered yet.
