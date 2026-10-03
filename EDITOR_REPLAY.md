# Editor differential replay

The level and world editors are checked the same way as the game (`REPLAY.md`). Scripted keyboard and mouse sessions edit and save maps or worlds. The C++ reference and the Rust port must then produce byte-identical per-frame dumps, screenshots and saved files.

- Harness: `src/common/EditorHarness.{h,cpp}` in `tools/editor-harness.patch`, ported as `src/common/editor_harness.rs`. Both editors share it.
- Scripts: `tools/editor_replays/<editor>/*.txt`, where `<editor>` is `worldedit` or `leveledit`.
- Golden output: `tools/editor_golden/<editor>/<script>/` contains `dump.txt.gz`, `frame_<n>.png`, `files/` and `home/`.

## Building the reference editors

The editors build from their own tree, `~/work/smw-ref-editors`, never from `~/work/smw-ref` (the game reference all game goldens come from). That tree is upstream plus `tools/cpp-harness.patch` plus `tools/editor-harness.patch`:

```sh
rsync -a --exclude .git ~/work/supermariowar/ ~/work/smw-ref-editors/
cd ~/work/smw-ref-editors
patch -p1 < <this repo>/tools/cpp-harness.patch
patch -p1 < <this repo>/tools/editor-harness.patch
mkdir -p build-editors && cd build-editors
cmake .. -DNO_NETWORK=ON -DBUILD_TESTS=OFF -DCMAKE_BUILD_TYPE=Release \
         -DDISABLE_DEFAULT_CFLAGS=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"
make -j smw-worldedit smw-leveledit
```

`editor-harness.patch` holds one `diff -uN supermariowar/src/<f> smw-ref-editors/src/<f>` per file it touches (`common/CMakeLists.txt`, `common/EditorHarness.{h,cpp}`, `common/map.h`, `leveleditor/leveleditor.cpp`, `worldeditor/worldeditor.cpp`); `~/work/smw-ref` never gets them, so the game reference stays clean upstream plus `cpp-harness.patch`. Regenerate it from `~/work` by rerunning that diff for each file, and check it with `patch -p1 --dry-run` on a clean copy that already has `cpp-harness.patch` applied.

## Running

```sh
tools/run_editor.sh worldedit tools/editor_replays/worldedit/paint_save.txt [out_dir]   # C++
tools/editor_parity.sh [script ...]          # Rust vs golden: dump, shots, saved files
tools/make_editor_golden.sh [script ...]     # regenerate tools/editor_golden/
```

`run_editor.sh` sets up each run as follows:

- It runs in a fresh directory that holds an APFS clone of the data tree as `data/`, and passes `--datadir data`. Saves never touch the reference data, and the world and map paths the editors print are identical across runs.
- It uses a fresh `HOME`, so no `worldeditor.bin` or `leveleditor.bin` from a previous run is read.
- It sets the same `SDL_VIDEODRIVER=dummy` / `SDL_AUDIODRIVER=dummy` / `SMW_*` environment as `run_ref.sh`.

Without `SMW_BIN` it runs `~/work/smw-ref-editors/build-editors/smw-<editor>` (`SMW_EDITOR_REF_DIR` overrides the tree). `editor_parity.sh` points it at `target/release/<editor>or` (`worldeditor`, `leveleditor`) and the repository's `data/`.

Script directives, given as `#@ key=value` lines:

| Key | Meaning |
|---|---|
| `seed`, `frames`, `shots` | as in `REPLAY.md` (`frames` defaults to 300) |
| `files` | comma-separated paths, relative to the run directory (e.g. `data/worlds/x.txt`), copied to `out/files/` after the run |
| `homefiles` | names in `$HOME/Library/Preferences/.smw/` copied to `out/home/` |

## Frame loop

Every editor loop ends its iteration with `SDL_Delay(delay)`. The reference build `#define`s `SDL_Delay` to `editorharness::frameDelay`, and the Rust port calls `editor_harness::frame_delay` at the same places. One call is one frame, and it runs these steps:

1. While frame `N < SMW_FRAMES`:
   - write the dump block for `N`;
   - if `N` is in `SMW_SHOT_FRAMES`, save `screen` as `frame_<N>.bmp`.
2. `N += 1`.
3. Push the events scheduled for frame `N`, or, once `N >= SMW_FRAMES`, push `SDL_QUIT`. The editors then exit through their normal shutdown, which saves `ZZworldeditor.txt` / `ZZleveleditor.map` and the settings file.
4. If the editor still has not exited 60 frames later, the harness exits with status 3.
5. Sleep `delay` ms unless `SMW_NOLIMIT` is set.

`editorharness::init()` pushes the frame-0 events. Loops that return early (a key that switches editor mode) do not end a frame. The next loop's `SDL_Delay` ends it instead.

The level editor's loops end with `FPSLimiter::beforeFlip()` instead, so the patch puts `editorharness::frameDelay(0)` before each of its five `beforeFlip` calls: the main loop, the block property page, and the nested tile, animation and tile type pickers of the platform editor. Its text dialogs (New, Save As, Find) loop on `SDL_Delay` without flipping and get the `#define`. The editor links `smw/FPSLimiter.cpp`, which calls `harness::noLimit()`, so `leveleditor.cpp` defines that to return `editorharness::noLimit()`.

## Script format

This is the `REPLAY.md` format plus mouse lines:

```
<frame> down|up <SDL key name>
<frame> mousedown|mouseup <left|middle|right> <x> <y>
<frame> mousemove <x> <y>
```

The harness builds each event as follows:

- **Key events:** `keysym.mod` is the set of scripted Shift/Ctrl/Alt keys held at that moment.
- **Mouse button events:** `clicks = 1`.
- **Motion events:** `state` is the scripted buttons held, and `xrel`/`yrel` are measured from the last scripted position.

`SDL_PushEvent` does not update `SDL_GetKeyboardState` or `SDL_GetMouseState`, so the harness keeps its own copies: a scancode array set and cleared by scripted key events at push time, and the scripted button mask. `editorharness::keyboardState()` / `mouseState()` (`editor_harness::keyboard_state()` / `mouse_state()`) return them while the harness is active and fall back to SDL otherwise.

- The level editor `#define`s `SDL_GetKeyboardState` and `SDL_GetMouseState` to these, so held Shift, Ctrl and the Z/X/C path modifiers work in scripts.
- The world editor still reads SDL directly, so it sees held modifiers released.

## Dump format

```
F <frame>
<editor lines>
R calls=<n> last=<u32>
```

The world editor writes one editor line:

`E state=<s> edit_mode=<m> set_tile=<t> auto=<0|1> col=<c> row=<r> w=<w> h=<h> stages=<n> vehicles=<n> warps=<n> world=<file name>`

| Field | Source |
|---|---|
| `state` | the main loop's editor state (`EDITOR_EDIT` 0, `EDITOR_STAGE` 21, ...) |
| `col`, `row` | `draw_offset_col`, `draw_offset_row` |
| `w`, `h` | `iWorldWidth`, `iWorldHeight` |
| `stages` | `game_values.tourstops.size()` |
| `vehicles`, `warps` | sizes of the editor's vehicle and warp lists |

The level editor writes five editor lines:

```
E state=<s> edit_mode=<m> layer=<l> mouse=<x>,<y> ignore=<0|1> only=<0|1> blocks=<0|1>
T tileset=<t> start=<x>,<y> end=<x>,<y> size=<cols>,<rows> drag=<0|1> view=<x>,<y>,<ax> block=<b>,<on> tiletype=<t> settype=<t> item=<i> warp=<dir>,<conn> nospawn=<n>
X move=<m> start=<x>,<y> offset=<x>,<y> drag=<sx>,<sy>,<ox>,<oy> replace=<0|1> nodrag=<0|1> copied=<layer>
P count=<n> edit=<i> state=<s> preview=<i> switch=<s>,<i> hazards=<n> hzstate=<s> hzedit=<i> modeitem=<m>,<drag> msg=<t> music=<t>
H map=<hex> plat=<hex> anim=<hex>
```

The fields are the editor globals of the same names (`edit_mode`, `selected_layer`, `set_tile_*`, `move_*`, `g_iNumPlatforms`, `iPlatformEditState`, `iEditState`, `modeitemmode`, `g_messagedisplaytimer`, `g_musiccategorydisplaytimer`...). `H` holds three 32-bit FNV-1a hashes (offset `0x811c9dc5`, prime `0x01000193`), each over a sequence of 32-bit little-endian ints, with floats taken by their bit pattern:

| Hash | Values, in order |
|---|---|
| `map` | for x in 0..20, y in 0..15: the 4 layers of `mapdata` (`iID`, `iCol`, `iRow`), `mapdatatop`, `objectdata` (`iType`, the stored settings, `fHidden`: all 26 settings for types 1 and 15, setting 0 for types 11-14, none otherwise, because the C++ leaves the others uninitialized), `warpdata` (`direction`, `connection`, `id`), the 6 `nospawn` flags. Then the `mapitems` count and each item (`itype`, `ix`, `iy`); the `maphazards` count and each hazard (`itype`, `ix`, `iy`, then per parameter `iparam`, `dparam`); `eyecandy[3]`; `musicCategoryID`; `iNumRaceGoals` and 8 race goal `x`, `y`; `iNumFlagBases` and 4 flag base `x`, `y`; `iSwitches[4]`; each byte of `szBackgroundFile`; `platforms.size()` |
| `plat` | for each of the first `g_iNumPlatforms` editor platforms: 300 tiles (`iID`, `iCol`, `iRow`), 300 types, `iVelocity`, `iStartX`, `iStartY`, `iEndX`, `iEndY`, `iPathType`, `fAngle`, `fRadiusX`, `fRadiusY`, `iDrawLayer` |
| `anim` | the 256 `animatedtiletypes` |

The C++ dump needs the private `CMap` members, so `editor-harness.patch` adds `friend void dumpLevelEditorState(void*)` to `map.h`; a friend declaration does not change the game binary.

## Known C++ behaviour the scripts avoid

- **Stage editing on a world that already has stages.** World loading leaves `game_values.tourstops` empty while `iNumStages` keeps the loaded count. A new stage then overwrites stale entries, and the vehicle menu reads past the vector and crashes. The stage scripts create a new world first.
- **Minigame stages (modes 25–27).** `SaveStage` reads `g_iNumGameModeSettings` past its 22 entries, into whatever globals the linker placed after it. In the patched reference those are the harness's own globals. Saving such a stage writes that many settings, or crashes. The Rust port reads 0 there. Bonus houses (24) read past the array too, but never write settings.
