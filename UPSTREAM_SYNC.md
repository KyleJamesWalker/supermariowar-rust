# Upstream sync: a7f7e25 to 5693918f

**Bottom line:** bringing the port up to upstream `5693918f` (72 commits past `a7f7e25`) is about 45 engineer-hours. 8 of those hours are done on this branch. Seven ported upstream changes and one latent port fix already make all 43 game replays match the new C++ on every frame, sound record and screenshot, and make all 12 editor dumps match. The rest of the work is editor pixels and saved files, `servers.toml`, error paths that no replay reaches, and optional structural alignment with the upstream gfx and tileset refactors.

| Category | Done | Remaining |
|---|---|---|
| Reference merge, build and goldens (one conflict, CMake flag change) | 2 h | 0 |
| Behaviour ports with parity evidence (7 changes, below) | 6 h | 0 |
| Behaviour changes not yet ported: editor surfaces and files, tileset loading, donut block, map foreground, tour-stop settings fallback, error paths (class b, todo) | 0 | 20 h |
| Infrastructure: `servers.yml` to `servers.toml`, binary string edge cases (class c) | 0 | 4 h |
| Structural alignment with refactors, no behaviour change (class a*) | 0 | 8 h |
| Coverage and harness: new replays for unreached fixes, map dump rerun, editor nondeterminism, `gfx_smoke` twin, docs and committed goldens | 0 | 5 h |
| **Total** | **8 h** | **37 h** |

Estimates assume the tooling on this branch. Agent wall-clock time for the done column was about 1 h 45 min.

## Live plan

The sync is approved to finish on `upstream-sync`. This section is the source of truth for who owns what. The coordinator updates it as items land.

How work flows:

1. Owners edit only their own files in this worktree and commit only their own paths. If `.git/index.lock` exists, wait and retry.
2. After each commit, the owner sends the coordinator the SHA. The coordinator runs `tools/sync_gate.sh <sha>`: `cargo test`, 43 game replays and 12 editor sessions against `~/work/smw-upstream-goldens`, diffed against the last accepted baseline. A regression gets a follow-up commit, never an amend.
3. Only the coordinator builds the C++ reference or writes goldens. To add a replay or editor session, send the coordinator the script. The coordinator generates goldens serially and generates editor goldens twice.
4. No pushes, no background runs left behind, and each agent cleans its own scratch.

| Item | Upstream | Owner | Status |
|---|---|---|---|
| Menu and world surfaces, thumbnails, skins | `c2e87df8` (world part), `5c979393`, `e3bab591` | gfx | `c2e87df8` (`c1df8f2`), `5c979393` (`3b512ff`) done: the world editor stage thumbnail now matches, so every editor shot matches. `e3bab591` skins todo |
| `ImageLoader`, editor surfaces, wrapping draws | `a4a6140d`, `0124c1eb`, `e0bcaf30`, `5c865fe5`, `3ad06ca7` (check what `dacabe1c` already covers), `6b6bdb7a` | gfx | `a4a6140d` level editor half (`fc564f2`, player), `3ad06ca7` (`165a44f`), `e0bcaf30` (`1d10612`), `0124c1eb` + `545fd077` (`f2f8763`, player), `5c865fe5` (`c351624`) done. Remaining: the `SpriteBuilder` to `ImageLoader` rename and `6b6bdb7a`, both structural |
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
| 290-map sweep goldens, fresh-worktree verification, final summary | | coordinator | sweep: 290/290 at `b249b58`. Done since: harness patches regenerated (`8f458d4`, verified to rebuild `harness-latest` exactly), build docs and module map (`be622a4`), C++ `build-reference.sh`/`REFERENCE.md` (`2e1e5f64`), networking-on C++ build (`build-latest-net`). Fresh-worktree verification and final summary last |

Gate baseline: `956e820`. 20/20 tests pass. 44/44 game replays match on every frame and shot. 13/13 editor sessions match on every dump, shot and saved file. The 290-map sweep and the 504-map dump match

## Branches and outputs

| What | Where |
|---|---|
| Rust port | branch `upstream-sync` (worktree `~/work/smw-rust-upstream-sync`), based on `main` `12ccbdf` |
| C++ reference | branch `harness-latest` (worktree `~/work/smw-cpp-harness-latest`), `harness` merged with `master` `5693918f` |
| C++ build | `~/work/supermariowar-cpp-reference/build-latest/{smw,smw-leveledit,smw-worldedit}` |
| New goldens, logs, diff crops | `~/work/smw-upstream-goldens` (scratch, not committed) |

Nothing is pushed. The committed `tools/golden*` still hold the `a7f7e25` outputs.

## Parity

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

## Ported changes and evidence

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

## Remaining editor differences

| Session | Difference | Cause |
|---|---|---|
| All 5 world editor sessions | Saved world files drop the bonus text (`Toad's House,1,p6,...` instead of `Toad's House,1,\|\|Pick an item...\|\|,p6,...`) | `1eaed535`, not ported |
| `stages_vehicles` frames 130, 145 | Stage thumbnail draws platforms tinted, colours differ | Thumbnail surfaces 16 bpp to 32 bpp, `5c979393` |
| `types_backgrounds` frame 70 | Background picker colours differ on the whole screen | Picker thumbnails 16 bpp to 32 bpp, `0124c1eb` |
| `platforms_save` 96, `types_backgrounds` 45 | Tile palette markers differ (about 1,480 px) | Likely `a4a6140d`: the tile-type overlay lost its magenta colour key. Not confirmed |
| `screenshot_find` | 9 saved PNGs differ in bytes, pixels identical | Screenshots saved with an alpha channel, `0124c1eb` |

## Commit triage

Classes: **a** no Rust impact. **a\*** refactor with no observable change on any exercised path (replays match without porting it), ported only to keep the one-module-per-file map. **b** behaviour change to mirror. **c** infrastructure the port replaced. Rust modules are under `src/`.

| Commit | Subject | Class | Rust module | Status | Note |
|---|---|---|---|---|---|
| `f56607a1` | Tileset manager no longer a directory iterator | b | `common/tileset_manager.rs`, `common/map/map_reader1{5,6,7}xx.rs` | todo | Finds `Classic` after sorting: fixes the kept "index taken before sorting" bug. Affects only pre-1.8 map conversion. Rerun the map dump |
| `a2541fcc` | Do not assume the classic tileset exists | a | `common/tileset_manager.rs`, map readers | | Pointer return. Matches only once `f56607a1` is ported |
| `00cffef9` | Tileset rects computed at compile time | a* | `common/tileset_manager.rs` | | |
| `c130b990` | Lazy load tileset textures | a* | `common/tileset_manager.rs`, `common/file_io.rs` | | |
| `3ad06ca7` | Direct blitting utilities on gfxSprite | b | `common/gfx/gfx_sprite.rs` | partly ported | `blank`, `draw_to`, `blit` in their `dacabe1c` form. Wrap test uses post-clip x plus shake |
| `b6f985ec` | Tile draw call simplifications | a* | `common/tileset_manager.rs`, `common/map.rs`, `common/movingplatform.rs` | | |
| `e0bcaf30` | Fewer direct SDL blits | b | `common/eyecandy.rs`, `common/map.rs`, `smw/world.rs`, `leveleditor/` | todo | Editor path dots wrap after `takescreenshot()` leaves wrap set |
| `fa3e6a22` | More tileset refactoring | a* | `common/map.rs`, `common/movingplatform.rs` | | |
| `981b56c3` | Store tilesets directly | a* | `common/tileset_manager.rs` | | |
| `4c6d805a` | Minor refactoring | a* | `common/tileset_manager.rs` | | |
| `53e65af9` | Removed path helpers | a* | `common/path.rs` | | |
| `5c865fe5` | gfxSprite in movingplatform | b | `common/movingplatform.rs` | todo | Platform side-wrap goes through the sprite wrap. Port with `dacabe1c` semantics |
| `4c336b4c` | gfxSprites in the map preview | a* | `common/ui/mi_map_preview.rs` | **ported** | With `c2e87df8`. Preview rect no longer clipped in place. Rust commit `2e0850a` cites only this hash |
| `c2e87df8` | Fewer surface allocations in menu elements | b | `common/ui/mi_map_preview.rs`, `smw/ui/mi_world*.rs` | partly ported | Adds `blank()`: map preview 16 bpp to 32 bpp (ported). World map surfaces gain alpha and wrap (todo, no visible change in replays) |
| `5c979393` | Fewer surface allocations in map code | b | `common/map.rs`, editors | todo | Editor thumbnails |
| `a4a6140d` | Image loading moved out of gfxSprite | b | `common/gfx/gfx_sprite.rs`, editors | todo | `SpriteBuilder` becomes `ImageLoader`. The level editor tile-type overlay loses its magenta colour key: likely the tile palette marker diffs |
| `559a4401` | Tileset rects const | a* | `common/tileset_manager.rs` | | |
| `0124c1eb` | Manual surface creation removed in editors | b | `leveleditor/`, `worldeditor/` | todo | Background picker 16 bpp to 32 bpp, screenshots saved with alpha, platform tooltip wraps |
| `e3bab591` | Skin processing surfaces | b | `common/gfx.rs` | todo | `blank()` blend mode none (ported). Skin surfaces become `blank()` with alpha, RGB unchanged |
| `b9bb1a85` | Removed manual `IMG_Load` calls | a | `common/gfx/gfx_palette.rs`, `smw/ui/mi_map_browser.rs` | | Same pixels for valid files. A broken file now throws |
| `6b6bdb7a` | More gfx refactoring | a* | `common/gfx.rs`, hazards | | |
| `545fd077` | Last manual blits in the world editor | a | `leveleditor/leveleditor.rs` | | Same pixels: the sheets have no transparency chunk |
| `05e1d52b` | Limit cloned submodules | a | | | CMake |
| `02664496` | Log typo | a | | | |
| `37c4eaec` | Font handling rewrite | b | `common/gfx/gfx_font.rs` | **ported** | |
| `cfcbbcb2` | Tileset loading more lazy | b | `common/tileset_manager.rs` | todo | `.tls` read on first use. Port with `f56f8ed1`, which fixes a `.tls` truncation it introduces |
| `b5fbfffc` | SDL exception message format | a | | | Message text |
| `3f18878f` | Removed CMake platform files | a | | | `-DDISABLE_DEFAULT_CFLAGS` gone |
| `34a03d48` | Dropped Mixer X | c | | none | The port never used Mixer X |
| `cb42e76c` | FetchContent find_package | a | | | |
| `220bbd95` | Emscripten warnings | a | | | |
| `c27115b5` | Downscale tileset when sizes missing | b | `common/tileset_manager.rs` | todo | Custom packs only |
| `d2ed0cd1` | Palette BMP fallback on PNG error | b | `smw/gs_menu.rs`, `smw/main.rs` | todo | Error path |
| `df71e796` | ci: GME in Windows release | a | | | |
| `f91700da` | Uniform separators in error messages | a | | | Message text |
| `b80e755e` | Audio constructors accept paths | a* | `smw/gs_gameplay.rs`, `smw/gs_menu.rs` | | |
| `5a636882` | Zlib package case | a | | | |
| `3575237f` | Bundled Mixer build args | a | | | |
| `f8f71743` | ENet hash | a | | | |
| `6e36bd76` | Leftover SDL1 code | a* | `leveleditor/leveleditor.rs` | | |
| `fc938877` | Editors catch errors | b | editors, `common/gfx.rs` | todo | Error dialog instead of crash |
| `f56f8ed1` | Save only loaded, modified tilesets | b | `common/tileset_manager.rs` | todo | Editor `.tls` writes. Current sessions match |
| `eaaea5e3` | Error on tile index out of bounds | b | `common/tileset_manager.rs` | todo | Superseded by `19dcc293` |
| `19dcc293` | Auto-extend the tile types array | b | `common/tileset_manager.rs` | todo | Out-of-range types read `NonSolid` |
| `5bb5459b` | Disappearing capes | b | `smw/player_components/player_cape.rs` | **ported** | |
| `cdcd9c02` | Double path resolution for world maps | b | `common/map_list.rs` | **ported** | |
| `5683b58a` | Log missing tour maps | a | | | stdout only |
| `d3ad2cbb` | Level editor screenshot crash | b | `leveleditor/leveleditor.rs` | todo | PNG bytes differ |
| `a7bc5276` | World music enum looping | b | `worldeditor/worldeditor.rs`, `common/file_list.rs` | todo | Cycles through all categories |
| `d546c05b` | Tour stops cleared after world load | b | `smw/world.rs` | **ported** | |
| `1eaed535` | Toad House text when saving worlds | b | `common/world_tour_stop.rs` | todo | Saved world files |
| `d70e4dc3` | Skip reloading present skins | b | `common/resource_manager.rs` | todo | Low risk: skips `gfx_loadmenuskin` when path and colour are unchanged |
| `df5a85a6` | Readability refactor | a* | `smw/ui/mi_tournament_scoreboard.rs` | | |
| `01e5fa7e` | World editor stage map field stuck | b | `common/map_list.rs` | todo | `fInCurrentFilterSet` defaults to true, so `next(true)` no longer loops on one map |
| `3b9ca6dd` | Bug report template | a | | | |
| `dacabe1c` | Clipped destination on wrapped blits | b | `common/gfx/gfx_sprite.rs` | **ported** | |
| `11bb5fba` | Donut block's own falling graphic | b | `common/movingplatform.rs`, `smw/objects/blocks/donut_block.rs` | todo | `donutblock.png` instead of Classic tile (29,15). No replay shows a falling donut on a shot frame |
| `efe2e390` | yaml-cpp replaced with toml11 | c | `smw/network/net_config_manager.rs` | todo | See below |
| `116a5324` | README toml11 | a | | | |
| `ceb5ddb9` | README SDL3 | a | | | |
| `277ee170` | Detailed file errors | b | `common/file_io.rs` | todo | Message text on IO errors |
| `8faf76bc` | Mode settings serialization moved | b | `common/world_tour_stop.rs` | todo | Hidden change: missing tour-stop settings fall back to `gamemodesettings`, not `gamemodemenusettings`. Differs with a non-default options.bin |
| `65a81608` | Serialization tests | a | | | |
| `70706d2b` | ci: MSVC package | a | | | |
| `1dbcbaf3` | ci: MSVC static runtime | a | | | |
| `28e9e673` | Binary text reads return a string | c | `common/file_io.rs` | todo | Port with `4b965424` |
| `4b965424` | Text serialization null bytes | c | `common/file_io.rs` | todo | Same bytes up to 254 chars. 255 chars now get a NUL, longer strings throw, embedded NULs are kept |
| `2934b282` | String serialization tests | a | | | |
| `051723d0` | File writing tests | a | | | |
| `6713b94a` | README screenshots | a | | | |
| `9663cc66` | README history | a | | | |
| `5693918f` | Map foreground not cleared before load | b | `common/map.rs` | todo | Lock the RLE foreground surface around the fill. Without it the previous map foreground shows through |

Totals: 25 **a**, 14 **a\***, 30 **b** (5 ported, 2 partly ported, 23 todo), 4 **c** (3 todo). Three triage agents read every diff. Their notes are folded into this table.

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

## Recommended process

`tools/upstream_sync.sh [upstream-ref]` runs steps 1 to 4 and prints both parity summaries. It never pushes.

1. Merge the upstream ref into a new `harness-<sha>` worktree of the reference repo. Resolve conflicts by keeping the harness hooks, then rerun.
2. Build `smw`, `smw-leveledit` and `smw-worldedit` with `-DNO_NETWORK=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"`.
3. Check determinism: run `start_classic` twice. Make the editor goldens serially, twice, and require identical output.
4. Generate goldens into a scratch `GOLDEN_ROOT` and run `parity.sh` and `editor_parity.sh` against them.
5. For each divergence, match the first differing record to a commit in `commits.txt`. A shift in RNG count points at data loading or randomness. A shift in pixels points at gfx.
6. Port one upstream commit per Rust commit, with the upstream hash in the subject. Rerun the full suite after each port.
7. Run dense screenshots (`SMW_SHOT_FRAMES` every 25 frames) on five replays, and rerun the 504-map dump against the new C++. Those cover pixel-only and map-loading changes that the sampled suite misses.
8. When the suites match, regenerate the committed goldens, update the commit pin in `README.md`, the build flags in `REPLAY.md`, and the C++ bug list in `PROGRESS.md`.

## Follow-up

- Port the 23 **b** todo rows. Start with `1eaed535` and the editor surface commits, which close the remaining editor differences.
- Add replays or editor sessions for fixes that no current script reaches: a falling donut block, world music cycling, a save of a world with bonus houses, and a pre-1.8 map in the level editor.
- `TourStopVec` (`common/game_values.rs`) emulates reads past `clear()`. After `d546c05b` the game no longer needs it, but the world editor's `erase` still does. Check before removing it.
- The other C++ bugs the port keeps on purpose (AI out-of-bounds map reads, the boss-minigame Frenzy cast, the `getBoolean` assert) are in files these 72 commits do not touch.
