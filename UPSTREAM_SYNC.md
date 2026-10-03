# Upstream sync: a7f7e25 to 5693918f

**Bottom line:** bringing the port up to upstream `5693918f` (72 commits past `a7f7e25`) is about 40 engineer-hours. 8 of those hours are done on this branch. Six ported upstream changes and one latent port fix already make all 43 game replays match the new C++ on every frame, sound record and screenshot, and make all 12 editor dumps match. The rest of the work is editor pixels and saved files, `servers.toml`, error paths that no replay reaches, and optional structural alignment with the upstream gfx and tileset refactors.

| Category | Done | Remaining |
|---|---|---|
| Reference merge, build and goldens (one conflict, CMake flag change) | 2 h | 0 |
| Behaviour ports with parity evidence (7 changes, below) | 6 h | 0 |
| Behaviour changes not yet ported: editor surfaces and files, donut block, map foreground, tile types, error paths (class b, todo) | 0 | 15 h |
| Infrastructure: `servers.yml` to `servers.toml` (class c) | 0 | 4 h |
| Structural alignment with refactors, no behaviour change (class a*) | 0 | 10 h |
| Coverage and harness: new replays for unreached fixes, map dump rerun, editor nondeterminism, `gfx_smoke` twin, docs and committed goldens | 0 | 5 h |
| **Total** | **8 h** | **34 h** |

Estimates assume the tooling on this branch. Agent wall-clock time for the done column was about 1 h 45 min.

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
| `4c336b4c` map preview sprites | `2e0850a` | Preview layers are `gfxSprite::blank` (screen format, not 16 bpp). `rectDst` is no longer clipped in place | `start_classic` frame 150 and `joy_game` frame 470 match (72,659 px each before) |
| `d546c05b` tour stops cleared after world load (reproduced C++ bug) | `8ba9fe1` | `reset_tour_stops()` runs before parsing | World editor dumps show `stages=12`, as in C++. Game replays unchanged |
| (latent port bug) | `d052cf2` | `MapList` positions follow their element on insert, as `std::multimap` iterators do | The level editor opens `NMcCoy_1-3`, as in C++. Exposed only once `cdcd9c02` made `addWorldMaps` insert maps |
| `dacabe1c` wrapped blits | `9cc30de` | `gfxSprite::draw` uses the upstream `blit` | Dense shots: `flow_world` frame 2050 differs by 122 px without it and matches with it |

Supporting commits: `358777e` adds `GOLDEN_ROOT` to the four golden and parity scripts, `68dc7ee` adds `tools/upstream_sync.sh`, `58ea0b0` updates `examples/gfx_smoke.rs`. `cargo test --release` passes (20 tests).

## Remaining editor differences

| Session | Difference | Cause |
|---|---|---|
| All 5 world editor sessions | Saved world files drop the bonus text (`Toad's House,1,p6,...` instead of `Toad's House,1,\|\|Pick an item...\|\|,p6,...`) | `1eaed535`, not ported |
| `stages_vehicles` frames 130, 145 | Stage thumbnail draws platforms tinted, colours differ | Editor thumbnail surfaces, `5c979393` / `0124c1eb` |
| `types_backgrounds` frame 70 | Background picker colours differ on the whole screen | 16 bpp editor surfaces replaced, `0124c1eb` / `545fd077` |
| `platforms_save` 96, `types_backgrounds` 45 | Tile palette markers differ (about 1,480 px) | Tileset series, likely `19dcc293`. Not root-caused |
| `screenshot_find` | 9 saved PNGs differ in bytes, pixels identical | Screenshot surface format, `d3ad2cbb` / `0124c1eb` |

## Commit triage

Classes: **a** no Rust impact. **a\*** refactor with no observable change on any exercised path (replays match without porting it), ported only to keep the one-module-per-file map. **b** behaviour change to mirror. **c** infrastructure the port replaced. Rust modules are under `src/`.

| Commit | Subject | Class | Rust module | Status | Note |
|---|---|---|---|---|---|
| `f56607a1` | Tileset manager no longer a directory iterator | b | `common/tileset_manager.rs`, `common/map/map_reader1{5,6,7}xx.rs` | todo | Finds `Classic` after sorting: fixes the kept "index taken before sorting" bug. Affects only pre-1.8 map conversion. Rerun the map dump |
| `a2541fcc` | Do not assume the classic tileset exists | b | `common/tileset_manager.rs`, map readers | todo | Null-safe when `Classic` is missing. Error path only |
| `00cffef9` | Tileset rects computed at compile time | a* | `common/tileset_manager.rs` | | |
| `c130b990` | Lazy load tileset textures | a* | `common/tileset_manager.rs`, `common/file_io.rs` | | |
| `3ad06ca7` | Direct blitting utilities on gfxSprite | a* | `common/gfx/gfx_sprite.rs` | partly ported | `blank`, `draw_to`, `blit` |
| `b6f985ec` | Tile draw call simplifications | a* | `common/tileset_manager.rs`, `common/map.rs`, `common/movingplatform.rs` | | |
| `e0bcaf30` | Fewer direct SDL blits | a* | `common/eyecandy.rs`, `common/map.rs`, `smw/world.rs` | | |
| `fa3e6a22` | More tileset refactoring | a* | `common/map.rs`, `common/movingplatform.rs` | | |
| `981b56c3` | Store tilesets directly | a* | `common/tileset_manager.rs` | | |
| `4c6d805a` | Minor refactoring | a* | `common/tileset_manager.rs` | | |
| `53e65af9` | Removed path helpers | a* | `common/path.rs` | | |
| `5c865fe5` | gfxSprite in movingplatform | a* | `common/movingplatform.rs` | | |
| `4c336b4c` | gfxSprites in the map preview | b | `common/ui/mi_map_preview.rs` | **ported** | 16 bpp surfaces to screen format |
| `c2e87df8` | Fewer surface allocations in menu elements | a* | `smw/ui/mi_world_preview_display.rs` | | World preview was already screen format |
| `5c979393` | Fewer surface allocations in map code | b | `common/map.rs`, editors | todo | Editor thumbnails |
| `a4a6140d` | Image loading moved out of gfxSprite | a* | `common/gfx/gfx_sprite.rs` | | `SpriteBuilder` becomes `ImageLoader` |
| `559a4401` | Tileset rects const | a* | `common/tileset_manager.rs` | | |
| `0124c1eb` | Manual surface creation removed in editors | b | `leveleditor/`, `worldeditor/` | todo | Editor surface formats |
| `e3bab591` | Skin processing surfaces | a* | `common/gfx.rs` | | |
| `b9bb1a85` | Removed manual `IMG_Load` calls | a* | `common/gfx/gfx_palette.rs`, `smw/ui/mi_map_browser.rs` | | Map browser not in replays. Unverified |
| `6b6bdb7a` | More gfx refactoring | a* | `common/gfx.rs`, hazards | | |
| `545fd077` | Last manual blits in the world editor | b | `leveleditor/leveleditor.rs` | todo | Background picker colours |
| `05e1d52b` | Limit cloned submodules | a | | | CMake |
| `02664496` | Log typo | a | | | |
| `37c4eaec` | Font handling rewrite | b | `common/gfx/gfx_font.rs` | **ported** | |
| `cfcbbcb2` | Tileset loading more lazy | a* | `common/tileset_manager.rs` | | |
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
| `d70e4dc3` | Skip reloading present skins | a* | `common/resource_manager.rs` | | Cache only |
| `df5a85a6` | Readability refactor | a* | `smw/ui/mi_tournament_scoreboard.rs` | | |
| `01e5fa7e` | World editor stage map field stuck | b | `common/map_list.rs` | todo | `fInCurrentFilterSet` defaults to true |
| `3b9ca6dd` | Bug report template | a | | | |
| `dacabe1c` | Clipped destination on wrapped blits | b | `common/gfx/gfx_sprite.rs` | **ported** | |
| `11bb5fba` | Donut block's own falling graphic | b | `common/movingplatform.rs`, `smw/objects/blocks/donut_block.rs` | todo | No replay reaches a falling donut on a shot frame |
| `efe2e390` | yaml-cpp replaced with toml11 | c | `smw/network/net_config_manager.rs` | todo | See below |
| `116a5324` | README toml11 | a | | | |
| `ceb5ddb9` | README SDL3 | a | | | |
| `277ee170` | Detailed file errors | b | `common/file_io.rs` | todo | Message text on IO errors |
| `8faf76bc` | Mode settings serialization moved | a* | `common/world_tour_stop.rs` | | New file `GameModeSettingsSerialization.cpp` |
| `65a81608` | Serialization tests | a | | | |
| `70706d2b` | ci: MSVC package | a | | | |
| `1dbcbaf3` | ci: MSVC static runtime | a | | | |
| `28e9e673` | Binary text reads return a string | a* | `common/file_io.rs` | | |
| `4b965424` | Text serialization null bytes | a* | `common/file_io.rs` | | Byte format equals `a7f7e25`. Strings over 254 chars now throw |
| `2934b282` | String serialization tests | a | | | |
| `051723d0` | File writing tests | a | | | |
| `6713b94a` | README screenshots | a | | | |
| `9663cc66` | README history | a | | | |
| `5693918f` | Map foreground not cleared before load | b | `common/map.rs` | todo | Pixels after a map change |

Totals: 22 **a**, 24 **a\*** (one partly ported), 24 **b** (6 ported, 18 todo), 2 **c** (1 todo).

### `servers.yml` to `servers.toml`

Upstream now reads and writes `$HOME/.smw/servers.toml` and ignores an existing `servers.yml` (no migration). The keys are unchanged: `player_name` (string) and `servers` (array of strings). The validation and fallback rules are also unchanged. The port reads and writes `servers.yml` with `yaml-rust2` and emulates yaml-cpp quoting, so that `tools/ref/persist_interop.sh` gets byte-identical files. To keep that test byte-identical, the Rust side needs a writer that matches `toml::format` from toml11 v4.4.0. Check its key order: `toml::table` is an `unordered_map`, so the order follows libc++ hashing. `src/server` already has a libc++-order helper for this.

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

- Port the 18 **b** todo rows. Start with `1eaed535` and the editor surface commits, which close the remaining editor differences.
- Add replays or editor sessions for fixes that no current script reaches: a falling donut block, world music cycling, a save of a world with bonus houses, and a pre-1.8 map in the level editor.
- `TourStopVec` (`common/game_values.rs`) emulates reads past `clear()`. After `d546c05b` the game no longer needs it, but the world editor's `erase` still does. Check before removing it.
- The other C++ bugs the port keeps on purpose (AI out-of-bounds map reads, the boss-minigame Frenzy cast, the `getBoolean` assert) are in files these 72 commits do not touch.
