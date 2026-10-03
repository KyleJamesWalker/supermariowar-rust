# Editor differential replay

The level and world editors are checked the same way as the game (`REPLAY.md`). Scripted keyboard and mouse sessions edit and save maps or worlds. The C++ reference and the Rust port must then produce byte-identical per-frame dumps, screenshots and saved files.

- Harness: `src/common/EditorHarness.{h,cpp}` in `tools/cpp-harness.patch`, ported as `src/common/editor_harness.rs`. Both editors share it.
- Scripts: `tools/editor_replays/<editor>/*.txt`, where `<editor>` is `worldedit` or `leveledit`.
- Golden output: `tools/editor_golden/<editor>/<script>/` contains `dump.txt.gz`, `frame_<n>.png`, `files/` and `home/`.

## Building the reference editors

Build them in a separate directory of the patched reference tree, so `build/smw` is not disturbed:

```sh
cd ~/work/smw-ref && mkdir -p build-editors && cd build-editors
cmake .. -DNO_NETWORK=ON -DBUILD_TESTS=OFF -DCMAKE_BUILD_TYPE=Release \
         -DDISABLE_DEFAULT_CFLAGS=ON -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off"
make -j smw-worldedit smw-leveledit
```

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

Without `SMW_BIN` it runs `~/work/smw-ref/build-editors/smw-<editor>`. `editor_parity.sh` points it at `target/release/<editor>or` (`worldeditor`, `leveleditor`) and the repository's `data/`.

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

`SDL_PushEvent` does not update `SDL_GetKeyboardState`. Editor code that checks held modifiers that way (Shift+S save-as, Ctrl+Delete, Shift in dialogs) therefore sees them released, on both sides.

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

## Known C++ behaviour the scripts avoid

- **Stage editing on a world that already has stages.** World loading leaves `game_values.tourstops` empty while `iNumStages` keeps the loaded count. A new stage then overwrites stale entries, and the vehicle menu reads past the vector and crashes. The stage scripts create a new world first.
- **Minigame stages (modes 25–27).** `SaveStage` reads `g_iNumGameModeSettings` past its 22 entries, into whatever globals the linker placed after it. In the patched reference those are the harness's own globals. Saving such a stage writes that many settings, or crashes. The Rust port reads 0 there. Bonus houses (24) read past the array too, but never write settings.
