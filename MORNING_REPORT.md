# Morning report: web, multiplayer and controls (2026-10-04)

**Bottom line:** the browser build now plays online through your relay, works on phones with touch controls, and assigns gamepads automatically. Netplay powerup blocks now match between players. Everything is merged to `main` (last merge `b631c97`) and pushed, the site and relay image are deployed, and `dist/Super Mario War.app` is rebuilt. Your relay at `smw-relay.vps.pocketsquirrel.com` passed a live two-browser game.

## What landed

| Merge | Change |
|---|---|
| `7c2d131` | On-screen touch controls for phones and tablets, plus a web app manifest so Add to Home Screen runs fullscreen |
| `3d2ac31` | Browser multiplayer: `smw_relay` (lobby server plus WebSocket relay), the browser network layer, Docker image, deploy examples. See `RELAY.md` |
| `75b3cec` | Backspace no longer navigates the page back while typing a name |
| `7a0a43d` | Net games: the host decides what powerup blocks release (an upstream C++ bug, fixed as a deliberate deviation) |
| `533ca89` | Gamepads: stick and D-pad drive the same controls on every build; pads go to the first players at each launch |
| `9343588` | Watching gamepad recordings in the browser; a launch without pads undoes the last pad assignment; touch keeps player 1 when pads are connected |
| `b631c97` | The touch-controls test pins its seed and map, so it no longer fails at random |

Every deliberate difference from the C++ is listed in `PROGRESS.md`.

## Verification

| Check | Result |
|---|---|
| `cargo test`, relay tests | pass (relay: 12 tests) |
| `tools/parity.sh`, `tools/editor_parity.sh` | 44/44 game replays, 13/13 editor sessions. `joy_game` and `joy_menu` are compared against Rust goldens because pad assignment deliberately differs from the C++ |
| Browser replay of `start_classic` | 2530 frames identical to the C++ golden |
| Native net games (`net_game_interop.sh`, 4 pairings × 2 scenarios) | 8/8 synced; Rust–Rust powerup spawns match |
| Two-browser game through a local relay (`web_netplay_test.mjs`) | synced, powerups match |
| Two-browser game against the live site and your relay | synced (606/628 and 610/627 frames within 2 px) |
| Relay image | 49 MB distroless; healthy; refuses foreign origins (403); CI built amd64 and arm64 |
| Touch test on an emulated phone (`web_touch_test.mjs`) | 10/10 consecutive passes |

## Try it

- **Two players online:** open https://www.kylejameswalker.com/supermariowar-rust/ in two browsers, choose Multiplayer, the relay server, then create a room in one and join from the other. If the room isn't listed yet, refresh the room list.
- **Gamepad (Q36 in XInput mode):** reload, press a pad button until the page shows "1 gamepad ready", click Play. P1 is the pad (A jump, B turbo, X item, Y pause); arrows/Return drive P2 and WASD/E drive P3. Set extra players to None in the main menu's Players row to play alone.
- **Phone:** open the site in landscape; the controls appear on touch screens. On iPhone, use Share → Add to Home Screen for fullscreen.

## Needs a real-device check

- iPhone: Add to Home Screen opens fullscreen; controls clear the notch and home indicator; long-press, magnifier and double-tap zoom stay blocked; keys release when switching apps.
- Holding the D-pad plus two buttons at once on real hardware.
- Android: the fullscreen button and the landscape lock.
- Backspace in the name field on the browser where you saw the bug (Chrome no longer goes back on Backspace, so the test couldn't reproduce it).
- Native `--replay` with a real pad plugged in (pads were unplugged when this was tested).

## Known gaps

- **Native and browser players can't share a game.** The relay serves browsers only.
- **Mixed C++/Rust net games keep the upstream powerup desync.** The C++ client ignores the new block packages.
- **Other random outcomes can still differ between net players:** frenzy cards, coin placement in the coin modes, stomp/survival enemies, random shell deaths. The same host-decides approach would fix each.
- **Touch on a solo phone:** player 2 defaults to human, so set it to CPU or None in the Players row before starting.
- **Touch controls send player 1's default keys.** If player 1's keyboard keys are rebound, touch won't follow.
- **SDL's browser backend drops the first stick movement on each axis** after Play. The D-pad is unaffected.
- **WebSocket runs over TCP**, so a lost packet can briefly stall a net game. WebRTC would fix it if it matters in practice.

---

# Earlier report: the initial port


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

The parity tools need the C++ reference; see `REPLAY.md` for building it (now [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference), branch `harness-latest`). As first written: Rebuild it from `~/work/supermariowar` plus `tools/cpp-harness.patch` using the cmake flags in `REPLAY.md`: `-DNO_NETWORK=ON -DDISABLE_DEFAULT_CFLAGS=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"`. The editors use `~/work/smw-ref-editors` (plus `tools/editor-harness.patch`).

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
