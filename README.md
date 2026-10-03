# Super Mario War, in Rust

A faithful Rust + rust-sdl2 port of [Super Mario War](https://github.com/mmatyas/supermariowar) (upstream commit [`5693918f`](https://github.com/mmatyas/supermariowar/commit/5693918f5e3ec8ef50ff4f86cfe07c8d2c5a247b)). It covers the game, menus, sound, netplay, the lobby server, and the level and world editors. Given the same scripted input, the Rust build produces the same per-frame game state, sound events and screenshots as the C++ original. `MORNING_REPORT.md` has the verification results.

## Build and run

Requires Rust and the SDL2, SDL2_image and SDL2_mixer libraries (on macOS: `brew install sdl2 sdl2_image sdl2_mixer`). Game data is the upstream [supermariowar-data](https://github.com/mmatyas/supermariowar-data) submodule.

```sh
git submodule update --init                             # or clone with --recursive
cargo run --release --bin smw -- --datadir data          # the game
cargo run --release --bin leveleditor -- --datadir data
cargo run --release --bin worldeditor -- --datadir data
cargo run --release --bin smw_server                     # netplay lobby server
```

## Session recordings

Every normal launch records its input to `~/Library/Preferences/.smw/replays/` (the newest 10 are kept), together with the settings it started with and a random seed, so the session can be replayed exactly. `SMW_NO_RECORD=1` turns this off.

```sh
cargo run --release --bin smw -- --datadir data --replay ~/Library/Preferences/.smw/replays/<file>.txt   # watch it
open "dist/Super Mario War.app" --args --replay /absolute/path/<file>.txt                               # from the app
tools/replay_compare.sh ~/Library/Preferences/.smw/replays/<file>.txt   # replay on the C++ original and the port, report the first difference
```

`--replay-speed <n>` watches faster. `REPLAY.md` ("Recordings") has the details.

## Verifying against the C++ original

The C++ original, plus the replay and state-dump hooks used to compare it with this port, lives in [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference) (branch `harness-latest`; the hooks are also kept here as `tools/cpp-harness.patch` and `tools/editor-harness.patch`). `REPLAY.md` specifies the harness, and `tools/parity.sh`, `tools/parity_sweep.sh` and `tools/editor_parity.sh` run the comparisons. `ARCHITECTURE.md` documents the porting conventions, and `PROGRESS.md` maps each C++ file to its Rust module.

## Credits

Game design, code and assets are from the Super Mario War project and its contributors; see `data/` and the upstream repository. This repository contains only the Rust translation and its test tooling.
