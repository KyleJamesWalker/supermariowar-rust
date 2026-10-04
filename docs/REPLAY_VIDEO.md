# Replay videos

`tools/replay_video.py` renders a replay script or session recording to an H.264 MP4, either one build alone or two builds side by side, optionally with a third panel that marks the pixels that differ. Use it to show what a change does on screen. There is no audio.

## Usage

```sh
# Before/after a fix: the committed fix against main, with the differing frames listed
tools/replay_video.py tools/replays/start_classic.txt --before origin/main --after HEAD --diff -o fix.mp4

# Watch one build
tools/replay_video.py ~/Library/Preferences/.smw/replays/<file>.txt --after target/release/smw

# One match of a session: a clip renders from the match's first frame
tools/replay_clip.py ~/Library/Preferences/.smw/replays/<file>.txt --match 1 -o clip.txt
tools/replay_video.py clip.txt --after target/release/smw

# Same binary, different settings per side: the sdl2-compat RLE foreground bug
tools/replay_video.py tools/replays/cpu_greed.txt --frames 400-1000 --diff \
    --before target/release/smw --before-env SMW_RLE=1 --label-before "RLE on" \
    --after target/release/smw --label-after "RLE off"

# The C++ reference against the port
tools/replay_video.py tools/replays/start_classic.txt --diff \
    --before ~/work/supermariowar-cpp-reference/build/smw --label-before "C++" --after target/release/smw
```

| Option | Effect |
|---|---|
| `--after`, `--before` | A binary, or a git rev of this repository. Only `--after` gives a single video. |
| `-o` | Output file (default `<replay name>.mp4` in the current directory). |
| `--frames a-b` | Render only game frames `a..=b`; the game stops after `b`. |
| `--speed x` | Playback speed. At 2 and above it also keeps only every `int(x)`th frame. |
| `--diff` | Adds a panel with the differing pixels in red over the dimmed after frame, and prints the differing frame ranges. |
| `--before-env`, `--after-env K=V` | Extra environment for one side, e.g. `SMW_RLE=1`. Repeatable. |
| `--before-data`, `--after-data` | Data directory for a binary. Default: the nearest `data/` above the binary, else `$SMW_REF_DIR/data`. |
| `--label-before`, `--label-after` | Panel labels (default: the rev or binary name and the extra env). |
| `--scale n` | Nearest-neighbour scale (default 2). |

The video plays at the game's real 62.5 fps (16 ms frames), H.264 in `yuv420p`, so it opens in QuickTime and browsers. Each panel carries its label and the game frame number. `ffmpeg` (with `libx264`) is the only dependency; the Homebrew build has no `drawtext`, so the script draws the text itself.

## How it works

- A git rev is checked out into a temporary worktree and built with `cargo build --release`. The binary is cached in `~/.cache/smw-replay-video/bin/<sha>/` (`--cache-dir` or `SMW_VIDEO_CACHE` move it), and one shared `CARGO_TARGET_DIR` there keeps rebuilds incremental. `HEAD` means the commit, not uncommitted changes; pass `target/release/smw` for those. The data submodule is the checkout's `data/` when the commit matches, otherwise a temporary worktree of it.
- Each side runs headless through `tools/run_ref.sh` with `SMW_SHOT_STREAM` pointed at a pipe (`docs/REPLAY.md`, Environment variables). The script compares the two frames, composes the panels and pipes them into one `ffmpeg`, so frames never reach the disk.
- A binary built before `SMW_SHOT_STREAM` existed (an older rev, or the C++ reference before `harness-shot-stream`) falls back to BMP screenshots: the replay reruns once per 240 frames, and each BMP is deleted once read. That is about 8x slower and uses about 300 MB of temporary space.
- Temporary files, worktrees and game processes are removed on exit, including on failure or Ctrl-C.

A 3,600-frame `--diff` video (3 panels, 3840x1000) takes about 45 s on an M-series Mac with about 11 MB of temporary disk; the MP4 is about 25 MB.
