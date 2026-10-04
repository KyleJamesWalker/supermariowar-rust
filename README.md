# Super Mario War, in Rust

A faithful Rust + rust-sdl2 port of [Super Mario War](https://github.com/mmatyas/supermariowar) (upstream commit [`5693918f`](https://github.com/mmatyas/supermariowar/commit/5693918f5e3ec8ef50ff4f86cfe07c8d2c5a247b)). It covers the game, menus, sound, netplay, the lobby server, and the level and world editors. Given the same scripted input, the Rust build produces the same per-frame game state, sound events and screenshots as the C++ original. `docs/MORNING_REPORT.md` has the verification results.

## Build and run

Requires Rust and the SDL2, SDL2_image and SDL2_mixer libraries (on macOS: `brew install sdl2 sdl2_image sdl2_mixer`). Game data is the upstream [supermariowar-data](https://github.com/mmatyas/supermariowar-data) submodule.

```sh
git submodule update --init                             # or clone with --recursive
cargo run --release --bin smw -- --datadir data          # the game
cargo run --release --bin leveleditor -- --datadir data
cargo run --release --bin worldeditor -- --datadir data
cargo run --release --bin smw_server                     # netplay lobby server
```

Native builds draw without SDL surface RLE, which on Homebrew's sdl2-compat costs most of the frame time and shows the map foreground's colour key (`docs/sdl2-compat-rle.md`); the web build keeps it. `SMW_RLE=1` or `SMW_RLE=0` overrides at launch.

## Web build

The port also builds for the browser as WebAssembly (`wasm32-unknown-emscripten`), mirroring upstream's Emscripten build: SDL2, SDL2_image (PNG and BMP), SDL2_mixer (WAV and OGG) and zlib come from Emscripten's ports, and `data/` is preloaded into the page. It needs emsdk 5.0.2, the version upstream's CI uses, and the Rust target:

```sh
git clone https://github.com/emscripten-core/emsdk.git ~/work/emsdk
~/work/emsdk/emsdk install 5.0.2 && ~/work/emsdk/emsdk activate 5.0.2
rustup target add wasm32-unknown-emscripten

tools/package_web.sh                          # builds and writes dist/web/ (index.html, smw.js, smw.wasm, smw.data, ...)
python3 -m http.server -d dist/web 8000       # then open http://localhost:8000
node tools/web_replay.mjs tools/replays/start_classic.txt out/   # run a replay in headless Chrome
node tools/web_touch_test.mjs out/                                # check the touch controls on an emulated phone
```

`tools/web_replay.mjs` runs a replay in the browser build the way `run_ref.sh` runs a native binary, and writes the same `dump.txt` and screenshots for `diffreplay.py`. In the browser, settings and session recordings are kept in the site's IndexedDB storage. The start screen can watch or download the last game or load a replay file, and the page footer downloads the current recording. A recording made in the browser replays on the native builds and with `tools/replay_compare.sh`.

On touch screens (`pointer: coarse`) the page shows on-screen controls for player 1: a D-pad, Jump, Run and Item, Start and Back. They press player 1's default keys, so the game sees ordinary keyboard input. The page footer forces them on or off. Player 2 defaults to a human player, so to play alone set it to CPU or None in the main menu's Players row. On iPhone, Share > Add to Home Screen runs the game fullscreen; Android can use the controls' fullscreen button or install the page.

## Browser multiplayer

Upstream's web build has no netplay. Here the browser build plays online through `smw_relay`, which runs the lobby server and relays game traffic between players over WebSocket. The page connects to `wss://smw-relay.vps.pocketsquirrel.com`, or to the relay a `?relay=ws://...` parameter names. Browser and native players cannot meet yet. `docs/RELAY.md` covers the protocol, settings and deployment.

```sh
cargo run --release --manifest-path relay/Cargo.toml -- --port 8080   # then open http://localhost:8000/?relay=ws://localhost:8080
node tools/web_netplay_test.mjs out/   # two headless browsers play a scripted net game
```

## Session recordings

Every normal launch records its input to `~/Library/Preferences/.smw/replays/` (the newest 10 are kept), together with the settings it started with and a random seed, so the session can be replayed exactly. `SMW_NO_RECORD=1` turns this off.

```sh
cargo run --release --bin smw -- --datadir data --replay ~/Library/Preferences/.smw/replays/<file>.txt   # watch it
open "dist/Super Mario War.app" --args --replay /absolute/path/<file>.txt                               # from the app
tools/replay_compare.sh ~/Library/Preferences/.smw/replays/<file>.txt   # replay on the C++ original and the port, report the first difference
```

`--replay-speed <n>` watches faster. `docs/REPLAY.md` ("Recordings") has the details. `tools/replay_video.py` renders a replay to an MP4, or two builds side by side (`docs/REPLAY_VIDEO.md`).

## Verifying against the C++ original

The C++ original, plus the replay and state-dump hooks used to compare it with this port, lives in [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference) (branch `harness-latest`; the hooks are also kept here as `tools/cpp-harness.patch` and `tools/editor-harness.patch`). `docs/REPLAY.md` specifies the harness, and `tools/parity.sh`, `tools/parity_sweep.sh` and `tools/editor_parity.sh` run the comparisons. `docs/ARCHITECTURE.md` documents the porting conventions, and `docs/PROGRESS.md` maps each C++ file to its Rust module. `docs/UPSTREAM_BACKLOG.md` tracks C++ bugs and port changes to offer upstream.

## Credits

Game design, code and assets are from the Super Mario War project and its contributors; see `data/` and the upstream repository. This repository contains only the Rust translation and its test tooling.
