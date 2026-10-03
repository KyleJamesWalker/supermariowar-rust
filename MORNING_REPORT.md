# Morning report: Rust port of Super Mario War

**Bottom line:** the whole C++ project is ported to plain Rust + rust-sdl2: the game, menus, sound, netplay, lobby server, level editor and world editor. Run on the same scripted input, the Rust build produces the same per-frame state, sound events and screenshots as the original C++, byte for byte, on every replay we have (333 game replays and 12 editor sessions). Rust also interoperates with the C++ over the network and reads and writes the same save files.

Branch `rust-port`. The crate is at the repo root (it was developed in `port/`; the old Bevy attempt has been removed). About 87k lines of Rust in 325 files, nothing pushed.

## Final verification (fresh checkout of HEAD)

Run at 06:07–06:22 on a clean `git worktree` of `59065bc`:

| Step | Result |
|---|---|
| `cargo build --release` (default and `--features no_network`) | ok |
| `cargo test --release` | 20/20 pass, including the RNG, map-load, MapList, libc++-order and alias-audit tests |
| `tools/parity.sh` (43 replays) | 43/43 identical: every frame, every `S` sound record, every screenshot |
| `tools/editor_parity.sh` (12 sessions) | 12/12 identical: dumps, screenshots and saved files |

One commit landed afterwards: `d63bf67`, the bonus-world sprite fix below. It only changes a path that used to panic, so no passing replay can change, and the affected replay was verified separately (12,000 frames identical).

Earlier full runs, same goldens:

| Suite | What it covers | Result |
|---|---|---|
| `tools/parity.sh` | 43 replays: all 22 modes with 4 CPUs, Tour/Tournament/World flows, 3 minigames, 3 fuzzed-input games, gamepad menus and play, option/player/team setups, menu navigation | 43/43 identical on every frame, sound record and screenshot |
| `tools/parity_sweep.sh` | 4 CPUs on every one of the 290 maps | 290/290 identical |
| `tools/editor_parity.sh` | 7 level editor + 5 world editor scripted sessions | 12/12 identical: dump, screenshots and every saved `.map`/`.tls`/world file |
| `tools/soak.py` | Random games (mode, map, options, CPUs, fuzzed humans), C++ vs Rust, no goldens | 723 games of 5,000–8,000 frames: no state divergence. 25 Rust crashes where the C++ silently reads out of bounds, in 3 classes (CMap edge reads, AI edge reads, boss-minigame Frenzy cast). All fixed (06407b7, ec5f3a6, a39d9aa), and every failing case now matches the saved C++ dump |
| `tools/ref/net_interop.sh`, `net_game_interop.sh` | C++/Rust clients × C++/Rust lobby server, then a real game through the menus | 8/8 pairings pass; game sync within 2 px on ≥95% of frames, same as C++↔C++ |
| `tools/ref/persist_interop.sh` | options.bin, controls, servers.yml, map cache, filters written by one build and read by the other | identical, including 5 fuzzed seeds |
| Map tests (`cargo test`) | All 504 maps loaded, 30 maps rendered, MapList, RNG, libc++ hash order vs C++ | identical |

## How it was verified

The C++ game is built from a pristine copy plus `tools/cpp-harness.patch`, which adds a fixed seed, scripted input (keyboard and gamepad), a per-frame state dump, screenshots, and a frame-clocked sound mixer so audio state is deterministic. Rust has the same hooks. `REPLAY.md` is the spec, and `ARCHITECTURE.md` describes the porting conventions. Each replay runs in its own sandbox (data clone plus fresh HOME), so runs can't interfere.

## How to run

```sh
cargo run --release --bin smw -- --datadir data   # the game
cargo run --release --bin leveleditor -- --datadir data
cargo run --release --bin worldeditor -- --datadir data
cargo run --release --bin smw_server              # lobby server
tools/verify_head.sh                              # build + test + 43-replay parity on a clean checkout
```

The parity tools need the C++ reference at `~/work/smw-ref/build/smw`. Rebuild it from `~/work/supermariowar` plus `tools/cpp-harness.patch` using the cmake flags in `REPLAY.md`: `-DNO_NETWORK=ON -DDISABLE_DEFAULT_CFLAGS=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"`. The editors use `~/work/smw-ref-editors` (plus `tools/editor-harness.patch`).

## Known gaps

- **Bonus house is untested.** It's only reachable inside World maps, and in 12,000 frames of `Contest_Bonus World` the CPUs never entered one. That replay (`tools/repros/flow_world_bonus.txt`) has no golden yet. It did find one real bug, now fixed in `d63bf67`: the tournament scoreboard draws unloaded skin frames, and the Release C++ silently skips them. With the fix it matches the C++ for all 12,000 frames.
- **Real-time 60 fps on two heavy scenes.** Rust runs at the same CPU cost as C++ (0.95–1.02× on 5 replays). `flow_world` and the heaviest map exceed the 16.7 ms budget on *both* builds, because Homebrew's sdl2-compat (over SDL3) re-encodes RLE sprite sheets on every blit. The upstream issue is drafted (not filed) in `docs/sdl2-compat-rle.md`. A pixel-safe bypass is possible but not done. Windowed 60 fps wasn't confirmed because the machine was saturated all night.
- **Soak repro replays not yet goldens.** One replay per fixed crash class is committed in `tools/repros/`, along with `flow_world_bonus.txt`. Run them with `run_ref.sh` and `run_rust.sh` and diff the outputs. They should be trimmed and given goldens; they sit outside `tools/replays/` so `parity.sh` doesn't count them as failures. The `flow_world_bonus` generator entry in `tools/gen_replays.py` is left uncommitted for the same reason. The private soak reference `~/work/smw-ref-soak` predates the sound records; rebuild it before the next soak.
- **Duplicate C++ memory-layout model.** Rust deliberately reproduces C++ out-of-bounds `CMap` reads (the C++ reads neighbouring fields silently). `common/map.rs` (`cpp_byte`) and `smw/ai.rs` (`cmap_byte`, which adds the warp tables) each carry a copy. Merging them is cleanup with no behaviour change.
- **Cosmetic stderr.** Where the C++ throws and catches (e.g. options.bin unwritable), Rust prints Rust's default "thread 'main' panicked" line before catching. Behaviour and exit code match.
- **Not ported on purpose:** `src/screenshot` is dead upstream (not in CMake, doesn't compile).
- **C++ bugs kept for parity**, all documented in code and PROGRESS.md: the Classic-tileset index taken before sorting, stale tour stops read after `clear()`, AI out-of-bounds map reads, the boss-minigame Frenzy-card cast, and the `getBoolean` assert that a debug C++ build would trip in `opt_gameplay`.

## Reference trees

Outside the repo: `~/work/smw-ref` (game), `~/work/smw-ref-editors`, `~/work/smw-ref-net` (networking on), `~/work/smw-ref-soak`.
