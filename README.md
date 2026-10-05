# Super Mario War, in Rust

A faithful Rust + rust-sdl2 port of [Super Mario War](https://github.com/mmatyas/supermariowar) (upstream commit [`5693918f`](https://github.com/mmatyas/supermariowar/commit/5693918f5e3ec8ef50ff4f86cfe07c8d2c5a247b)). It covers the game, menus, sound, netplay, the lobby server, and the level and world editors. Given the same scripted input, the Rust build produces the same per-frame game state, sound events and screenshots as the C++ original. `docs/MORNING_REPORT.md` has the verification results.

`docs/GAME_MANUAL.md` is the player's manual: menus, controls, all 21 game modes, blocks and items.

## Downloads

Each [release](https://github.com/KyleJamesWalker/supermariowar-rust/releases) carries ready-to-run packages and a `SHA256SUMS` file:

| Package | Runs on | Notes |
|---|---|---|
| `SuperMarioWar-<version>-linux-x86_64.tar.gz`, `-linux-aarch64.tar.gz` | Linux with glibc 2.35 or newer (Ubuntu 22.04, Debian 12) | Needs the system SDL2 libraries: `sudo apt install libsdl2-2.0-0 libsdl2-image-2.0-0 libsdl2-mixer-2.0-0`. Run `SuperMarioWar/smw`. |
| `SuperMarioWar-<version>-macos-universal.zip` | macOS 14 or newer (Apple silicon), 15 or newer (Intel) | Ad-hoc signed, not notarized: after unzipping, run `xattr -dr com.apple.quarantine "Super Mario War.app"` once. |
| `SuperMarioWar-<version>-windows-x86_64.zip` | Windows 10 or newer, x64 | SDL2 DLLs included. Settings go to `%USERPROFILE%\.smw\`. |
| `SuperMarioWar-<version>-web.zip` | Any static web server | The site published at GitHub Pages. |
| `SuperMarioWar-<version>.muxapp` | Anbernic handhelds on muOS | See [Anbernic (muOS)](#anbernic-muos). |
| `SuperMarioWar-<version>.apk` | Android 5.0 or newer, with a gamepad or keyboard | Debug-signed. See [Android](#android). |

Every CI run also uploads these packages as workflow artifacts (`smw-linux-x86_64`, `smw-macos-universal`, `smw-windows-x86_64`, ...). `docs/RELEASING.md` describes how a release is cut.

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

## Anbernic (muOS)

Each `v*` release carries `SuperMarioWar-<version>.muxapp`, a one-file install for Anbernic handhelds running muOS: copy it to the SD card's `ARCHIVE/` folder and install it with Applications > Archive Manager. [`device/muos/README.md`](device/muos/README.md) covers installing, controls, where settings, recordings and logs go, troubleshooting and building the package.

## Android

`.github/workflows/build_android.yml` builds a debug-signed APK for arm64-v8a, armeabi-v7a and x86_64 following upstream's SDL-template Android port, and plays replays on an x86_64 emulator against the goldens. It needs a gamepad or keyboard; there are no on-screen controls yet. [`android/README.md`](android/README.md) covers installing, data and settings paths, controls and building.

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

`tools/web_replay.mjs` runs a replay in the browser build the way `run_ref.sh` runs a native binary, and writes the same `dump.txt` and screenshots for `diffreplay.py`. In the browser, settings and session recordings are kept in the site's IndexedDB storage. The start screen lists the last session's matches, each to watch or download as a clip, downloads the whole session, or loads a replay file, and the page footer downloads the current recording. A recording made in the browser replays on the native builds and with `tools/replay_compare.sh`.

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
tools/replay_clip.py <file>.txt --list                  # the session's matches
tools/replay_clip.py <file>.txt --match 2 -o clip.txt   # match 2 alone, as a clip that plays like any replay
```

Each match in a recording is marked and checkpointed, so `--replay <file> --segment <k>` watches match k alone, and a clip shares a single match without the menus before it. `--replay-speed <n>` watches faster. The web page downloads recordings and clips as `.smwrp`, the text gzipped (`gunzip -c f.smwrp` reads it), or as `.txt`; every tool and `--replay` reads either. `docs/REPLAY.md` ("Recordings") has the details. `tools/replay_video.py` renders a replay to an MP4, or two builds side by side (`docs/REPLAY_VIDEO.md`).

## Verifying against the C++ original

The C++ original, plus the replay and state-dump hooks used to compare it with this port, lives in [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference) (branch `harness-latest`; the hooks are also kept here as `tools/cpp-harness.patch` and `tools/editor-harness.patch`). `docs/REPLAY.md` specifies the harness, and `tools/parity.sh`, `tools/parity_sweep.sh` and `tools/editor_parity.sh` run the comparisons. `docs/ARCHITECTURE.md` documents the porting conventions, and `docs/PROGRESS.md` maps each C++ file to its Rust module. `docs/UPSTREAM_BACKLOG.md` tracks C++ bugs and port changes to offer upstream.

## CI

`.github/workflows/ci.yml` runs on every pull request and every push to `main`:

- clippy (fails on errors) and cargo-deny (licenses, advisories, sources)
- `cargo test`, debug and release, on Linux, macOS and Windows
- replay parity: `tools/parity.sh`, `tools/parity_sweep.sh`, `tools/editor_parity.sh` and `tools/segment_check.py` against the committed goldens. macOS, where the goldens were made, gates on dumps and screenshots. Linux replays on a FAT32 image so data directories list in a fixed order, gates on dumps and saved files, and also gates against goldens from the C++ reference built on the same runner.
- the web build's clip and page tests in headless Chrome
- the Linux, macOS, Windows, web and muOS packages (`build_*.yml`, the same reusable workflows `release.yml` calls)

`web.yml` deploys the web build to GitHub Pages and `relay.yml` pushes the relay image to GHCR, both from `main`.

## Credits

Game design, code and assets are from the Super Mario War project and its contributors; see `CREDITS`, `data/` and the upstream repository. This repository contains only the Rust translation and its test tooling.
