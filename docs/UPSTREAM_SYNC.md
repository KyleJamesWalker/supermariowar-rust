# Upstream sync: a7f7e25 to 5693918f

**Bottom line:** `upstream-sync` ports all 72 upstream commits between `a7f7e25` and `5693918f`, and it matches the new C++ everywhere we can measure. On a fresh worktree at HEAD with default paths, `cargo test` passes and the replays match on every frame, sound record, screenshot and saved file:

- 44/44 game replays
- 290/290 sweep maps
- 13/13 editor sessions

The 504-map dump also matches. Networking interop with the new C++ passes too: 8/8 pairings stay in sync and 9/9 persisted-file cases are identical. The committed goldens and harness patches now come from `harness-latest` at `5693918f`. Nothing is pushed or merged.

## Result

| Check (fresh worktree, HEAD, default paths) | Result |
|---|---|
| `cargo test --release` | 20/20 pass |
| `tools/parity.sh` (game) | 44/44 match: every frame, sound record and screenshot |
| `tools/parity_sweep.sh` (4 CPUs on every map) | 290/290 match |
| `tools/editor_parity.sh` | 13/13 match: dumps, 132 screenshots, 37 saved files |
| `matches_cpp_map_dump` (504 maps x 3 read types) | match |
| `net_interop.sh`, `net_game_interop.sh` vs networking-on new C++ (at `dd74644` / `3b512ff`) | 8/8 pairings, 8/8 synced on both servers |
| `persist_interop.sh`, networking on (at `3b512ff`) | 9/9 identical, including `servers.toml` |
| `tools/gfx_smoke/compare.sh` | byte-identical BMP |
| Dense screenshots, every 25 frames on 8 replays (at `9cc30de` and `ece585c`) | 1,694/1,694 frames match |

**What changed:** 59 commits on `upstream-sync`, 31 of which port named upstream commits. They touch 43 files in `src/` and `examples/` and 30 tool files, plus the regenerated goldens. In the C++ reference, `harness-latest` merges upstream `5693918f` into `harness`. Its only source change beyond that is the level editor dump fix (`869d3c55`).

**Effort:** the experiment estimated about 45 engineer-hours. The experiment itself took one agent about 1 h 45 min. Finishing took about 2 h 30 min of wall-clock time, with five agents in parallel (coordinator, gfx, foundation, player, objects), not counting a usage-limit pause. Most of the parity confidence came from the tooling: one gate per commit, plus dense screenshots for fixes that sampled screenshots miss.

**Known gaps:**
- `8faf76bc` (tour-stop settings fallback) is ported, but no short flow on shipped data reaches it. The fallback only differs when a game plays a stop with explicit settings and a later-parsed stop of the same mode omits them. `cov_tour_options` covers a tour played with a non-default `options.bin` instead.
- `11bb5fba` (donut graphic) has no visible effect with shipped data, because Classic's `donutblock.png` is pixel-identical to tile (29,15). It was verified once with a recoloured sprite. That check is not part of the suite.
- `5693918f` (foreground lock) makes no visible difference under sdl2-compat 2.32.72. It is ported for fidelity.
- `53e65af9` is not ported. It only renames a helper.
- C++ bugs the port still reproduces on purpose:
  - AI and `CMap` out-of-bounds map reads
  - the boss-minigame Frenzy cast
  - the `getBoolean` assert compiled out in Release
  - map thumbnails missing position-drawn hazards and platform shadows (new in `5c979393`)

  The sync fixed four bugs the port used to copy: the double world map path resolution, tour stops cleared after load, the Classic tileset index taken before sorting, and the cape offset.
- The level editor's `MapBlock::iSettings` stay uninitialized in the C++ (`CMap::clearMap`). The harness now hashes only the stored settings. Without that change, the C++ dump varies from run to run.

## Live plan

The sync is approved to finish on `upstream-sync`. This section is the source of truth for who owns what. The coordinator updates it as items land.

How work flows:

1. Owners edit only their own files in this worktree and commit only their own paths. If `.git/index.lock` exists, wait and retry.
2. After each commit, the owner sends the coordinator the SHA. The coordinator runs `tools/sync_gate.sh <sha>`: `cargo test`, 43 game replays and 12 editor sessions against `~/work/smw-upstream-goldens`, diffed against the last accepted baseline. A regression gets a follow-up commit, never an amend.
3. Only the coordinator builds the C++ reference or writes goldens. To add a replay or editor session, send the coordinator the script. The coordinator generates goldens serially and generates editor goldens twice.
4. No pushes, no background runs left behind, and each agent cleans its own scratch.

| Item | Upstream | Owner | Status |
|---|---|---|---|
| Menu and world surfaces, thumbnails, skins | `c2e87df8` (world part), `5c979393`, `e3bab591` | gfx | done: `c2e87df8` (`c1df8f2`), `5c979393` (`3b512ff`), `e3bab591` (`0f61a29`). Dense shots every 25 frames on 5 replays (world map, skins, team select): 864/864 match the new C++ |
| `ImageLoader`, editor surfaces, wrapping draws | `a4a6140d`, `0124c1eb`, `e0bcaf30`, `5c865fe5`, `3ad06ca7` (check what `dacabe1c` already covers), `6b6bdb7a` | gfx | done except `6b6bdb7a`: `a4a6140d` (`fc564f2` player, `ece585c` rename), `3ad06ca7` (`165a44f`), `e0bcaf30` (`1d10612`), `0124c1eb` + `545fd077` (`f2f8763`, player), `5c865fe5` (`c351624`) |
| Map foreground clear, donut graphic | `5693918f`, `11bb5fba` | gfx | done: `9d74d6d` (gfx), `df0f56a` (objects). Donut verified with a recoloured `donutblock.png` (Classic's is pixel-identical to tile 29,15): Rust before the port matches 194/200 dense shots, after it 200/200. Foreground: no difference on 1,100 dense `flow_tour`/`flow_tournament` frames before or after, so the old missing lock is harmless under sdl2-compat |
| Tileset manager series | `f56607a1`, `a2541fcc`, `00cffef9`, `c130b990`, `b6f985ec`, `fa3e6a22`, `981b56c3`, `4c6d805a`, `559a4401`, `cfcbbcb2` + `f56f8ed1`, `c27115b5`, `eaaea5e3` + `19dcc293` | foundation | done: `41016dc`. The 504-map dump now matches the new C++ on every map |
| `servers.toml` and interop scripts | `efe2e390` | foundation | done: `6e8672e`, `dd74644`. `net_interop.sh` 8/8. Against the networking-on new C++ (`build-latest-net`): `net_game_interop.sh` 8/8 pairings synced on both servers, networking-on `persist_interop.sh` 9/9 identical, with `servers.toml` carrying fuzzed values through the game |
| Binary strings, file errors | `28e9e673` + `4b965424`, `277ee170` | foundation | done: `ee4dcb2` |
| Tour-stop settings fallback | `8faf76bc` | foundation | done: `b249b58`. The fallback only differs when a same-mode stop with explicit settings is played and then a later-parsed stop omits them, which no short flow on shipped data reaches. `cov_tour_options` (`956e820`) covers a tour played with a non-default `options.bin` instead |
| Error paths | `fc938877`, `b9bb1a85`, `d2ed0cd1` | foundation | done: `fc938877` (`b765c64` player, `52be321` objects), `b9bb1a85` + `d2ed0cd1` (`c27c340`) |
| Level editor: screenshot crash, editor-side gfx parts (with gfx) | `d3ad2cbb` | player | done: `21bdeac`. Screenshot PNG bytes still wait on `0124c1eb` |
| Skin reload cache | `d70e4dc3` | player | done: `f7f6229` |
| World editor: bonus text, stage map field, music cycling | `1eaed535`, `01e5fa7e`, `a7bc5276` | objects | done: `91c9975`, `46d8a3a`, `30eba25`. Saved world files now match in 4 of 5 sessions |
| `TourStopVec` follow-up after `d546c05b` | | objects | done: `cd28809` (now a plain `Vec`) |
| Coverage replays: falling donut, map foreground, non-default `options.bin` for `8faf76bc` | | coordinator | done: donut and foreground checked (see gfx row). `cov_tour_options` and the world editor `sync_coverage` session (objects) added with goldens (`956e820`) |
| 504-map dump against the new C++ | | coordinator | done (assigned to foundation; both ran it): `69f1ceb` updates `map_dump`. All 504 maps and the MapList dump match since `41016dc` |
| `gfx_smoke` C++ twin on the new API | | coordinator | done: `941ede6`. The twin and the Rust example write byte-identical BMPs |
| Level editor nondeterminism root cause | | coordinator | done (assigned to player; the coordinator fixed it first): `CMap::clearMap` leaves `MapBlock::iSettings` uninitialized and `g_map` is now allocated after `gfx_init`, so the harness hashed stale heap bytes. player independently found the same cause. Both dumpers hash only stored settings (`8a8d41d`, C++ `869d3c55`). 42/42 parallel runs agree |
| Small structural leftovers | `53e65af9`, `b80e755e`, `6e36bd76`, `df5a85a6` | coordinator | closed without code: `df5a85a6`, `b80e755e` and `6e36bd76` remove C++ constructs the port never had. `53e65af9` would only replace `get_filename_from_path` with `Path::file_name` in `leveleditor.rs`, with no behaviour change, so it is left |
| 290-map sweep goldens, fresh-worktree verification, final summary | | coordinator | done: sweep goldens from the new C++. Harness patches regenerated (`8f458d4`). Docs (`be622a4`, C++ `2e1e5f64`). Committed goldens replaced (`f7ac7d5`). `verify_head.sh` fixed (`9f31612`) and run on a fresh worktree at HEAD: all suites match |

Gate baseline: `956e820`. 20/20 tests pass. 44/44 game replays match on every frame and shot. 13/13 editor sessions match on every dump, shot and saved file. The 290-map sweep and the 504-map dump match

## Branches and outputs

| What | Where |
|---|---|
| Rust port | branch `upstream-sync` (worktree `~/work/smw-rust-upstream-sync`), based on `main` `12ccbdf`, plus `31ef403` from `main` cherry-picked as `9cd1819` |
| C++ reference | branch `harness-latest` (worktree `~/work/smw-cpp-harness-latest`), `harness` merged with upstream `5693918f`, head `2e1e5f64` |
| C++ builds | `~/work/supermariowar-cpp-reference/build-latest` (NO_NETWORK: game and editors) and `build-latest-net` (networking on: `smw`, `smw-server`) |
| Goldens | committed in `tools/golden`, `tools/golden_sweep` and `tools/editor_golden` (`f7ac7d5`). The scratch copy in `~/work/smw-upstream-goldens` also holds `edref/` (the editors' reference data) and the gate logs |

Neither repository is pushed. `harness-latest` exists only locally, and `README.md` and `REFERENCE.md` refer to it.

## Experiment-phase parity (before the ports)

| Comparison | Game replays (43) | Editor sessions (12) |
|---|---|---|
| `main` vs old goldens (baseline) | 43/43 match, all shots | 12/12 match: dumps, shots, files |
| `main` vs new C++ | 0/43 match: all diverge at the first Match Selection frame. 2/130 shots | 7/12 dumps match, 0/110 shots, 19/35 files |
| `upstream-sync` vs new C++ | **43/43 match, 130/130 shots** | **12/12 dumps match**, 105/110 shots, 19/35 files |
| `upstream-sync` vs old goldens | 0/43, all diverge at the first Match Selection frame (expected, see below) | 0/12 (expected: font pixels and the world map list) |

Dense check: `start_classic`, `cpu_classic`, `cpu_ctf`, `flow_world` and `fuzz_a`, with a screenshot every 25 frames, give 830/830 identical frames against the new C++.

### Why `main` diverges from the new C++

Every one of the 43 first divergences is the same record: on the frame Match Selection opens, the old code makes 13 RNG calls and the new code makes none. Upstream `cdcd9c02` stopped `MapList` from resolving the world map directories twice. The old code therefore never found the world maps, and each tour stop that names one fell back to `MapList::random()`. The frame number differs per replay only because each replay reaches Match Selection at a different time (30 to 1952).

The other pixel differences in `main` come from the font rewrite (`37c4eaec`), the map preview surface format (`4c336b4c`) and the cape offset (`5bb5459b`).

### Why `upstream-sync` diverges from the old goldens

The same 43 frames diverge in reverse (0 calls against 13), and nothing else does. This is the intended effect of `cdcd9c02`. The editor sessions also diverge on font pixels and on the world editor's map list.

## Ported during the experiment

| Upstream | Rust commit | Change | Evidence |
|---|---|---|---|
| `cdcd9c02` map dirs resolved twice (reproduced C++ bug) | `bdceedb` | `add_maps_from` takes a resolved path | All 43 dumps go from diverged to identical |
| `37c4eaec` font rewrite (large refactor) | `ed16f54` | `gfx_font.rs` rewritten: glyph areas between markers, colour key at (0,1), screen-format RLE surface, whole-glyph chop. `s_font.rs` removed | Shot mismatches drop from 51 to 3 |
| `5bb5459b` disappearing capes | `7f6262e` | `iCapeYOffset` signed | `cpu_health` frame 2014 matches (the cape was missing) |
| `4c336b4c` + `c2e87df8` map preview sprites | `2e0850a` | Preview layers are `gfxSprite::blank` (screen format, not 16 bpp). `rectDst` is no longer clipped in place | `start_classic` frame 150 and `joy_game` frame 470 match (72,659 px each before) |
| `d546c05b` tour stops cleared after world load (reproduced C++ bug) | `8ba9fe1` | `reset_tour_stops()` runs before parsing | World editor dumps show `stages=12`, as in C++. Game replays unchanged |
| (latent port bug) | `d052cf2` | `MapList` positions follow their element on insert, as `std::multimap` iterators do | The level editor opens `NMcCoy_1-3`, as in C++. Exposed only once `cdcd9c02` made `addWorldMaps` insert maps |
| `dacabe1c` wrapped blits | `9cc30de` | `gfxSprite::draw` uses the upstream `blit` | Dense shots: `flow_world` frame 2050 differs by 122 px without it and matches with it |

Supporting commits: `358777e` adds `GOLDEN_ROOT` to the four golden and parity scripts, `68dc7ee` adds `tools/upstream_sync.sh`, `58ea0b0` updates `examples/gfx_smoke.rs`. `cargo test --release` passes (20 tests).

## Editor differences found during the experiment

All of these are resolved. `1eaed535` fixed the saved world files. `5c979393` fixed the stage thumbnail. `0124c1eb` fixed the background picker and screenshot PNGs. `a4a6140d` fixed the tile palette markers.

## Commit triage

Classes: **a** no Rust impact. **a\*** refactor with no observable change on any exercised path (replays match without porting it), ported only to keep the one-module-per-file map. **b** behaviour change to mirror. **c** infrastructure the port replaced. Rust modules are under `src/`.

| Commit | Subject | Class | Rust module | Status | Note |
|---|---|---|---|---|---|
| `f56607a1` | Tileset manager no longer a directory iterator | b | `common/tileset_manager.rs`, `common/map/map_reader1{5,6,7}xx.rs` | ported (`41016dc`) | Finds `Classic` after sorting: fixes the kept "index taken before sorting" bug. Affects only pre-1.8 map conversion. Rerun the map dump |
| `a2541fcc` | Do not assume the classic tileset exists | a | `common/tileset_manager.rs`, map readers | ported (`41016dc`)
| `00cffef9` | Tileset rects computed at compile time | a* | `common/tileset_manager.rs` | ported (in `41016dc`)
| `c130b990` | Lazy load tileset textures | a* | `common/tileset_manager.rs`, `common/file_io.rs` | ported (in `41016dc`)
| `3ad06ca7` | Direct blitting utilities on gfxSprite | b | `common/gfx/gfx_sprite.rs` | ported (`165a44f`) | `blank`, `draw_to`, `blit` in their `dacabe1c` form. Wrap test uses post-clip x plus shake |
| `b6f985ec` | Tile draw call simplifications | a* | `common/tileset_manager.rs`, `common/map.rs`, `common/movingplatform.rs` | ported (in `41016dc`)
| `e0bcaf30` | Fewer direct SDL blits | b | `common/eyecandy.rs`, `common/map.rs`, `smw/world.rs`, `leveleditor/` | ported (`1d10612`) | Editor path dots wrap after `takescreenshot()` leaves wrap set |
| `fa3e6a22` | More tileset refactoring | a* | `common/map.rs`, `common/movingplatform.rs` | ported (in `41016dc`)
| `981b56c3` | Store tilesets directly | a* | `common/tileset_manager.rs` | ported (in `41016dc`)
| `4c6d805a` | Minor refactoring | a* | `common/tileset_manager.rs` | ported (in `41016dc`)
| `53e65af9` | Removed path helpers | a* | `common/path.rs` | left: rename only
| `5c865fe5` | gfxSprite in movingplatform | b | `common/movingplatform.rs` | ported (`c351624`) | Platform side-wrap goes through the sprite wrap. Port with `dacabe1c` semantics |
| `4c336b4c` | gfxSprites in the map preview | a* | `common/ui/mi_map_preview.rs` | ported (`2e0850a`) | With `c2e87df8`. Preview rect no longer clipped in place. Rust commit `2e0850a` cites only this hash |
| `c2e87df8` | Fewer surface allocations in menu elements | b | `common/ui/mi_map_preview.rs`, `smw/ui/mi_world*.rs` | ported (`2e0850a`, `c1df8f2`) | Adds `blank()`: map preview 16 bpp to 32 bpp, world map surfaces gain alpha and wrap |
| `5c979393` | Fewer surface allocations in map code | b | `common/map.rs`, editors | ported (`3b512ff`) | Editor thumbnails |
| `a4a6140d` | Image loading moved out of gfxSprite | b | `common/gfx/gfx_sprite.rs`, editors | ported (`ece585c`, `fc564f2`) | `SpriteBuilder` becomes `ImageLoader`. The level editor tile-type overlay loses its magenta colour key: likely the tile palette marker diffs |
| `559a4401` | Tileset rects const | a* | `common/tileset_manager.rs` | ported (in `41016dc`)
| `0124c1eb` | Manual surface creation removed in editors | b | `leveleditor/`, `worldeditor/` | ported (`f2f8763`) | Background picker 16 bpp to 32 bpp, screenshots saved with alpha, platform tooltip wraps |
| `e3bab591` | Skin processing surfaces | b | `common/gfx.rs` | ported (`0f61a29`) | `blank()` blend mode none (ported). Skin surfaces become `blank()` with alpha, RGB unchanged |
| `b9bb1a85` | Removed manual `IMG_Load` calls | a | `common/gfx/gfx_palette.rs`, `smw/ui/mi_map_browser.rs` | ported (`c27c340`)
| `6b6bdb7a` | More gfx refactoring | a* | `common/gfx.rs`, hazards | ported (`3839d2c`)
| `545fd077` | Last manual blits in the world editor | a | `leveleditor/leveleditor.rs` | ported (`f2f8763`)
| `05e1d52b` | Limit cloned submodules | a | | | CMake |
| `02664496` | Log typo | a | | | |
| `37c4eaec` | Font handling rewrite | b | `common/gfx/gfx_font.rs` | ported (`ed16f54`) | |
| `cfcbbcb2` | Tileset loading more lazy | b | `common/tileset_manager.rs` | ported (`41016dc`) | `.tls` read on first use. Port with `f56f8ed1`, which fixes a `.tls` truncation it introduces |
| `b5fbfffc` | SDL exception message format | a | | | Message text |
| `3f18878f` | Removed CMake platform files | a | | | `-DDISABLE_DEFAULT_CFLAGS` gone |
| `34a03d48` | Dropped Mixer X | c | | none | The port never used Mixer X |
| `cb42e76c` | FetchContent find_package | a | | | |
| `220bbd95` | Emscripten warnings | a | | | |
| `c27115b5` | Downscale tileset when sizes missing | b | `common/tileset_manager.rs` | ported (`41016dc`) | Custom packs only |
| `d2ed0cd1` | Palette BMP fallback on PNG error | b | `smw/gs_menu.rs`, `smw/main.rs` | ported (`c27c340`) | Error path |
| `df71e796` | ci: GME in Windows release | a | | | |
| `f91700da` | Uniform separators in error messages | a | | | Message text |
| `b80e755e` | Audio constructors accept paths | a* | `smw/gs_gameplay.rs`, `smw/gs_menu.rs` | nothing to port
| `5a636882` | Zlib package case | a | | | |
| `3575237f` | Bundled Mixer build args | a | | | |
| `f8f71743` | ENet hash | a | | | |
| `6e36bd76` | Leftover SDL1 code | a* | `leveleditor/leveleditor.rs` | nothing to port
| `fc938877` | Editors catch errors | b | editors, `common/gfx.rs` | ported (`52be321`, `b765c64`) | Error dialog instead of crash |
| `f56f8ed1` | Save only loaded, modified tilesets | b | `common/tileset_manager.rs` | ported (`41016dc`) | Editor `.tls` writes. Current sessions match |
| `eaaea5e3` | Error on tile index out of bounds | b | `common/tileset_manager.rs` | ported (`41016dc`) | Superseded by `19dcc293` |
| `19dcc293` | Auto-extend the tile types array | b | `common/tileset_manager.rs` | ported (`41016dc`) | Out-of-range types read `NonSolid` |
| `5bb5459b` | Disappearing capes | b | `smw/player_components/player_cape.rs` | ported (`7f6262e`) | |
| `cdcd9c02` | Double path resolution for world maps | b | `common/map_list.rs` | ported (`bdceedb`) | |
| `5683b58a` | Log missing tour maps | a | | | stdout only |
| `d3ad2cbb` | Level editor screenshot crash | b | `leveleditor/leveleditor.rs` | ported (`21bdeac`) | PNG bytes differ |
| `a7bc5276` | World music enum looping | b | `worldeditor/worldeditor.rs`, `common/file_list.rs` | ported (`30eba25`) | Cycles through all categories |
| `d546c05b` | Tour stops cleared after world load | b | `smw/world.rs` | ported (`8ba9fe1`) | |
| `1eaed535` | Toad House text when saving worlds | b | `common/world_tour_stop.rs` | ported (`91c9975`) | Saved world files |
| `d70e4dc3` | Skip reloading present skins | b | `common/resource_manager.rs` | ported (`f7f6229`) | Low risk: skips `gfx_loadmenuskin` when path and colour are unchanged |
| `df5a85a6` | Readability refactor | a* | `smw/ui/mi_tournament_scoreboard.rs` | nothing to port
| `01e5fa7e` | World editor stage map field stuck | b | `common/map_list.rs` | ported (`46d8a3a`) | `fInCurrentFilterSet` defaults to true, so `next(true)` no longer loops on one map |
| `3b9ca6dd` | Bug report template | a | | | |
| `dacabe1c` | Clipped destination on wrapped blits | b | `common/gfx/gfx_sprite.rs` | ported (`9cc30de`) | |
| `11bb5fba` | Donut block's own falling graphic | b | `common/movingplatform.rs`, `smw/objects/blocks/donut_block.rs` | ported (`df0f56a`) | `donutblock.png` instead of Classic tile (29,15). No replay shows a falling donut on a shot frame |
| `efe2e390` | yaml-cpp replaced with toml11 | c | `smw/network/net_config_manager.rs` | ported (`dd74644`, `6e8672e`) | See below |
| `116a5324` | README toml11 | a | | | |
| `ceb5ddb9` | README SDL3 | a | | | |
| `277ee170` | Detailed file errors | b | `common/file_io.rs` | ported (`ee4dcb2`) | Message text on IO errors |
| `8faf76bc` | Mode settings serialization moved | b | `common/world_tour_stop.rs` | ported (`b249b58`) | Hidden change: missing tour-stop settings fall back to `gamemodesettings`, not `gamemodemenusettings`. Differs with a non-default options.bin |
| `65a81608` | Serialization tests | a | | | |
| `70706d2b` | ci: MSVC package | a | | | |
| `1dbcbaf3` | ci: MSVC static runtime | a | | | |
| `28e9e673` | Binary text reads return a string | c | `common/file_io.rs` | ported (`ee4dcb2`) | Port with `4b965424` |
| `4b965424` | Text serialization null bytes | c | `common/file_io.rs` | ported (`ee4dcb2`) | Same bytes up to 254 chars. 255 chars now get a NUL, longer strings throw, embedded NULs are kept |
| `2934b282` | String serialization tests | a | | | |
| `051723d0` | File writing tests | a | | | |
| `6713b94a` | README screenshots | a | | | |
| `9663cc66` | README history | a | | | |
| `5693918f` | Map foreground not cleared before load | b | `common/map.rs` | ported (`9d74d6d`) | Lock the RLE foreground surface around the fill. Without it the previous map foreground shows through |

Totals: 25 **a**, 14 **a\***, 30 **b**, 4 **c**. Every **b** and **c** commit is ported (Mixer X needed nothing), and every **a\*** refactor is ported except `53e65af9`, a helper rename. Three triage agents read every diff, and their notes are folded into this table.

### `servers.yml` to `servers.toml`

Upstream now reads and writes `$HOME/.smw/servers.toml` and ignores an existing `servers.yml` (no migration). The keys are unchanged: `player_name` (string) and `servers` (array of strings). A non-string `player_name` is now an error. A non-string server entry is skipped with a warning, where YAML stopped the loop.

The port reads and writes `servers.yml` with `yaml-rust2` and emulates yaml-cpp quoting so `tools/ref/persist_interop.sh` gets byte-identical files. A toml11 v4.4.0 build on macOS writes `servers` first, then `player_name`, then a blank line. The key order comes from libc++ hashing. An array stays inline until its running length passes 60 characters, then goes multiline with a 4-space indent and a trailing comma. The Rust writer must match this, and `persist_interop.sh`, `persist_fuzz.py`, `net_driver.sh` and `net_game_interop.sh` must move to the new file name.

## Problems hit

- **CMake flags changed.** Upstream removed `cmake/Platform*.cmake`, so `-DDISABLE_DEFAULT_CFLAGS=ON` no longer exists and is no longer needed on arm64. The build now always appends `-O3` after the user flags. The dumps stay deterministic and match, because `-ffp-contract=off` still applies. toml11 v4.4.0 and zlib v1.3.2 come through FetchContent, and SDL2 still comes from Homebrew. The editor targets are now `smw-leveledit` and `smw-worldedit`. `build-reference.sh` and `REPLAY.md` need the new flags.
- **One merge conflict.** `src/worldeditor/worldeditor.cpp`: upstream moved global construction into a new `inner_main()`. The resolution keeps `editorharness::init()` and `setDumper()` right after `gfx_init`, ahead of the moved construction. The merge commit is `287e5daa`.
- **Symlinked data trees escape the sandbox.** `run_ref.sh` and `run_editor.sh` clone the data tree with `cp -R`, which copies a symlink as a link. Pointing the editor reference at a symlinked `data` made six concurrent C++ editor runs write into the same checkout. The first editor goldens were wrong. `upstream_sync.sh` now makes a real APFS clone.
- **The new C++ level editor is nondeterministic under load.** In 1 of 6 parallel runs of `leveledit/smoke`, the frame-0 map hash differs. Sequential runs agree with each other and with Rust. The cause is not found. Editor goldens are now made serially, twice, and compared.
- **A latent port bug surfaced.** `MapList` held indices where C++ holds `std::multimap` iterators. This only mattered once `cdcd9c02` made `addWorldMaps` insert anything.
- **Screenshots are sampled.** With 2 to 6 shots per replay, the suite did not see `dacabe1c`. The dense run did.
- **The `gfx_smoke` C++ twin does not build** against the new sources (`SpriteBuilder`, `drawStretch`, `gfx_drawpreview` changed). Only the Rust side is updated.

## Reference branches

In [supermariowar-cpp-reference](https://github.com/KyleJamesWalker/supermariowar-cpp-reference):

- `harness-latest` (default): upstream plus the replay, screenshot and editor hooks. The reference build and every golden come from it. Changes to the hooks land here through PRs.
- `master`: upstream, unmodified. The harness patches are `git diff master harness-latest`, and fixes meant for upstream (`UPSTREAM_BACKLOG.md`) branch from here so their PRs carry no harness code.
- Tag `port-base-a7f7e25`: the upstream commit the port was first translated from, plus the two original hook commits.

## Recommended process

`tools/upstream_sync.sh [upstream-ref]` runs steps 1 to 4 and prints both parity summaries. It never pushes.

1. Merge the upstream ref into a new `harness-<sha>` worktree branched from `harness-latest`. Resolve conflicts by keeping the harness hooks, then rerun.
2. Build `smw`, `smw-leveledit` and `smw-worldedit` with `-DNO_NETWORK=ON -DSMW_NO_RLE=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"`.
3. Check determinism: run `start_classic` twice. Make the editor goldens serially, twice, and require identical output.
4. Generate goldens into a scratch `GOLDEN_ROOT` and run `parity.sh` and `editor_parity.sh` against them.
5. For each divergence, match the first differing record to a commit in `commits.txt`. A shift in RNG count points at data loading or randomness. A shift in pixels points at gfx.
6. Port one upstream commit per Rust commit, with the upstream hash in the subject. Rerun the full suite after each port.
7. Run dense screenshots (`SMW_SHOT_FRAMES` every 25 frames) on five replays, and rerun the 504-map dump against the new C++. Those cover pixel-only and map-loading changes that the sampled suite misses.
8. When the suites match, regenerate the committed goldens, update the commit pin in `README.md`, the build flags in `REPLAY.md`, and the C++ bug list in `PROGRESS.md`.

## Next sync

Run `tools/upstream_sync.sh` against the new upstream ref. Gate each port with `tools/sync_gate.sh`, and finish with `tools/verify_head.sh` (set `SMW_MAP_DUMP` for the map comparison). Generate editor goldens serially, twice. When the suites match, replace the committed goldens and regenerate the harness patches with `git diff master harness-<sha>`. Land the C++ side as a PR from `harness-<sha>` into `harness-latest`, after updating `master` to the synced upstream commit.
