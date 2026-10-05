#!/usr/bin/env python3
"""Render a replay to an H.264 MP4, alone or side by side for two builds. See docs/REPLAY_VIDEO.md.

  replay_video.py <replay.txt|.smwrp> --after <bin|git-rev> [--before <bin|git-rev>] [-o out.mp4]
                  [--frames a-b] [--speed x] [--diff] [--scale n]
                  [--before-env K=V ...] [--after-env K=V ...] [--label-before TXT] [--label-after TXT]

Each build runs headless through run_ref.sh and streams raw frames (SMW_SHOT_STREAM) through a pipe,
so nothing but the dumps touches the disk. Exit status: 0 done, 1 a game or ffmpeg failed, 2 usage.
"""

import argparse
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
from fractions import Fraction
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
PORT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
from replay_clip import read_recording, recording_stem  # noqa: E402
W, H = 640, 480
FRAME = W * H * 4
FPS = Fraction(125, 2)
HEADER = 20
BG = bytes((32, 24, 24, 255))
WHITE = bytes((255, 255, 255, 255))
GREEN = bytes((96, 220, 96, 255))
RED = bytes((64, 64, 255, 255))

# 5x7 bitmap font; lower case prints as upper case and unknown characters as '?'.
FONT = {
    "A": "01110 10001 10001 11111 10001 10001 10001", "B": "11110 10001 10001 11110 10001 10001 11110",
    "C": "01110 10001 10000 10000 10000 10001 01110", "D": "11110 10001 10001 10001 10001 10001 11110",
    "E": "11111 10000 10000 11110 10000 10000 11111", "F": "11111 10000 10000 11110 10000 10000 10000",
    "G": "01110 10001 10000 10111 10001 10001 01111", "H": "10001 10001 10001 11111 10001 10001 10001",
    "I": "01110 00100 00100 00100 00100 00100 01110", "J": "00111 00010 00010 00010 00010 10010 01100",
    "K": "10001 10010 10100 11000 10100 10010 10001", "L": "10000 10000 10000 10000 10000 10000 11111",
    "M": "10001 11011 10101 10101 10001 10001 10001", "N": "10001 10001 11001 10101 10011 10001 10001",
    "O": "01110 10001 10001 10001 10001 10001 01110", "P": "11110 10001 10001 11110 10000 10000 10000",
    "Q": "01110 10001 10001 10001 10101 10010 01101", "R": "11110 10001 10001 11110 10100 10010 10001",
    "S": "01111 10000 10000 01110 00001 00001 11110", "T": "11111 00100 00100 00100 00100 00100 00100",
    "U": "10001 10001 10001 10001 10001 10001 01110", "V": "10001 10001 10001 10001 10001 01010 00100",
    "W": "10001 10001 10001 10101 10101 10101 01010", "X": "10001 10001 01010 00100 01010 10001 10001",
    "Y": "10001 10001 10001 01010 00100 00100 00100", "Z": "11111 00001 00010 00100 01000 10000 11111",
    "0": "01110 10001 10011 10101 11001 10001 01110", "1": "00100 01100 00100 00100 00100 00100 01110",
    "2": "01110 10001 00001 00010 00100 01000 11111", "3": "11111 00010 00100 00010 00001 10001 01110",
    "4": "00010 00110 01010 10010 11111 00010 00010", "5": "11111 10000 11110 00001 00001 10001 01110",
    "6": "00110 01000 10000 11110 10001 10001 01110", "7": "11111 00001 00010 00100 01000 01000 01000",
    "8": "01110 10001 10001 01110 10001 10001 01110", "9": "01110 10001 10001 01111 00001 00010 01100",
    " ": "00000 00000 00000 00000 00000 00000 00000", ":": "00000 01100 01100 00000 01100 01100 00000",
    "-": "00000 00000 00000 11111 00000 00000 00000", ".": "00000 00000 00000 00000 00000 01100 01100",
    ",": "00000 00000 00000 00000 01100 00100 01000", "/": "00000 00001 00010 00100 01000 10000 00000",
    "_": "00000 00000 00000 00000 00000 00000 11111", "=": "00000 00000 11111 00000 11111 00000 00000",
    "+": "00000 00100 00100 11111 00100 00100 00000", "(": "00010 00100 01000 01000 01000 00100 00010",
    ")": "01000 00100 00010 00010 00010 00100 01000", "#": "01010 01010 11111 01010 11111 01010 01010",
    "@": "01110 10001 10111 10101 10111 10000 01110", "~": "00000 00000 01000 10101 00010 00000 00000",
    "^": "00100 01010 10001 00000 00000 00000 00000", "?": "01110 10001 00001 00010 00100 00000 00100",
    "!": "00100 00100 00100 00100 00100 00000 00100", "x": "00000 10001 01010 00100 01010 10001 00000",
}
ADVANCE = 12
_glyphs = {}


def glyph(ch, color):
    """The 2x-scaled glyph as 14 rows of ADVANCE pixels each."""
    ch = ch if ch == "x" else ch.upper()
    ch = ch if ch in FONT else "?"
    key = (ch, color)
    if key not in _glyphs:
        rows = []
        for bits in FONT[ch].split():
            row = b"".join((color if b == "1" else BG) * 2 for b in bits) + BG * (ADVANCE - 10)
            rows += [row, row]
        _glyphs[key] = rows
    return _glyphs[key]


def draw_text(buf, width, x, text, color=WHITE, right=False):
    """Draws `text` into the header strip `buf` (width pixels wide) at x, or ending at x."""
    if right:
        x -= len(text) * ADVANCE
    for ch in text:
        if 0 <= x and x + ADVANCE <= width:
            for i, row in enumerate(glyph(ch, color)):
                off = ((3 + i) * width + x) * 4
                buf[off:off + len(row)] = row
        x += ADVANCE


def fail(msg, status=2):
    print(f"replay_video: {msg}", file=sys.stderr)
    sys.exit(status)


def directive(replay, key):
    value = None
    for line in read_recording(replay).splitlines():
        if line.startswith("#@") and "=" in line:
            k, v = line[2:].strip().split("=", 1)
            if k.strip() == key:
                value = v.strip()
    return value


def segment_start(replay):
    """The first frame a segment replay or clip plays (docs/REPLAY.md, "Markers, checkpoints and clips"), else 0."""
    match = os.environ.get("SMW_SEGMENT") or directive(replay, "segment")
    if match:
        for line in read_recording(replay).splitlines():
            if line.startswith("#@ checkpoint ") and f" match={match} " in line + " ":
                return int(re.search(r" frame=(\d+)", line).group(1))
    return 0


def find_data(binary):
    for d in list(Path(binary).resolve().parents)[:4]:
        if (d / "data" / "gfx").is_dir():
            return d / "data"
    ref = Path(os.environ.get("SMW_REF_DIR", "~/work/supermariowar-cpp-reference")).expanduser() / "data"
    return ref if (ref / "gfx").is_dir() else None


class Builds:
    """Resolves a binary path or a git rev of this repository to (binary, data dir)."""

    def __init__(self, cache, tmp):
        self.cache = cache
        self.tmp = tmp
        self.worktrees = []

    def git(self, *args, cwd=PORT, check=True):
        r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
        if check and r.returncode != 0:
            fail(f"git {' '.join(args)}: {r.stderr.strip()}", 1)
        return r

    def resolve(self, spec, data):
        path = Path(spec).expanduser()
        if path.is_file() and os.access(path, os.X_OK):
            data = Path(data).expanduser() if data else find_data(path)
            if data is None:
                fail(f"no data directory found above {path} or in SMW_REF_DIR; pass --before-data/--after-data")
            return path.resolve(), data
        r = self.git("rev-parse", "--verify", "--quiet", f"{spec}^{{commit}}", check=False)
        if r.returncode != 0:
            fail(f"{spec} is neither an executable nor a git rev")
        sha = r.stdout.strip()
        return self.build(sha), Path(data).expanduser() if data else self.data_at(sha)

    def build(self, sha):
        binary = self.cache / "bin" / sha / "smw"
        if binary.is_file():
            return binary
        src = Path(tempfile.mkdtemp(prefix=f"src-{sha[:10]}-", dir=self.tmp))
        self.git("worktree", "add", "--detach", str(src), sha)
        self.worktrees.append((PORT, src))
        print(f"replay_video: building {sha[:10]}", file=sys.stderr)
        env = dict(os.environ, CARGO_TARGET_DIR=str(self.cache / "target"))
        r = subprocess.run(["cargo", "build", "--release", "--quiet", "--bin", "smw"], cwd=src, env=env,
                           capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stderr, file=sys.stderr)
            fail(f"cargo build of {sha[:10]} failed", 1)
        binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(self.cache / "target" / "release" / "smw", binary)
        return binary

    def data_at(self, sha):
        repo = PORT / "data"
        want = self.git("rev-parse", f"{sha}:data").stdout.strip()
        have = self.git("rev-parse", "HEAD", cwd=repo, check=False).stdout.strip()
        if want == have:
            return repo
        if not have:
            fail(f"{repo} is not checked out; run git submodule update --init")
        dest = Path(tempfile.mkdtemp(prefix=f"data-{want[:10]}-", dir=self.tmp))
        self.git("worktree", "add", "--detach", str(dest), want, cwd=repo)
        self.worktrees.append((repo, dest))
        return dest

    def cleanup(self):
        for repo, path in reversed(self.worktrees):
            subprocess.run(["git", "worktree", "remove", "--force", str(path)], cwd=repo, capture_output=True)
        self.worktrees = []


class Side:
    """One headless run whose frames arrive on a pipe (SMW_SHOT_STREAM)."""

    def __init__(self, name, replay, binary, data, extra_env, first, last, every, out):
        self.name = name
        self.out = out
        self.out.mkdir()
        rfd, wfd = os.pipe()
        env = dict(os.environ, SMW_BIN=str(binary), SMW_DATA_DIR=str(data), SMW_SHOT_STREAM=f"/dev/fd/{wfd}",
                   SMW_SHOT_RANGE=f"{first}-{last}", SMW_SHOT_EVERY=str(every), SMW_FRAMES=str(last + 1))
        env.update(extra_env)
        self.proc = subprocess.Popen(
            [str(TOOLS / "run_ref.sh"), str(replay), str(out)],
            env=env, pass_fds=(wfd,), stdout=subprocess.DEVNULL, start_new_session=True)
        os.close(wfd)
        self.pipe = os.fdopen(rfd, "rb", buffering=FRAME)
        self.last = None
        self.done = False
        self.ended = None

    def read(self):
        """The next frame, or the last one again once the stream has ended."""
        if not self.done:
            data = self.pipe.read(FRAME)
            if len(data) == FRAME:
                self.last = data
            else:
                self.done = True
        return self.last

    def finish(self):
        self.pipe.close()
        return report(self.name, self.proc.wait(), self.out)

    def kill(self):
        kill(self.proc)


class BmpSide:
    """A binary without SMW_SHOT_STREAM: reruns the replay per chunk of SMW_SHOT_FRAMES and deletes each BMP once read."""

    CHUNK = 240

    def __init__(self, name, replay, binary, data, extra_env, first, last, every, out):
        self.name, self.replay, self.out = name, replay, out
        self.env = dict(os.environ, SMW_BIN=str(binary), SMW_DATA_DIR=str(data))
        self.env.update(extra_env)
        self.todo = list(range(first, last + 1, every))
        self.queue = []
        self.proc = None
        self.status = 0
        self.last = None
        self.done = False
        self.ended = None

    def run_chunk(self):
        chunk, self.todo = self.todo[:self.CHUNK], self.todo[self.CHUNK:]
        shutil.rmtree(self.out, ignore_errors=True)
        env = dict(self.env, SMW_SHOT_FRAMES=",".join(map(str, chunk)), SMW_FRAMES=str(chunk[-1] + 1))
        self.proc = subprocess.Popen([str(TOOLS / "run_ref.sh"), str(self.replay), str(self.out)],
                                     env=env, stdout=subprocess.DEVNULL, start_new_session=True)
        self.status = report(self.name, self.proc.wait(), self.out) or self.status
        self.proc = None
        self.queue = [self.out / f"frame_{n}.bmp" for n in chunk]

    def read(self):
        if not self.done and not self.queue and self.todo and not self.status:
            self.run_chunk()
        if not self.done:
            path = self.queue.pop(0) if self.queue else None
            if path is not None and path.exists():
                self.last = read_bmp(path)
                path.unlink()
            else:
                self.done = True
                self.queue = []
        return self.last

    def finish(self):
        shutil.rmtree(self.out, ignore_errors=True)
        return self.status

    def kill(self):
        if self.proc is not None:
            kill(self.proc)


def read_bmp(path):
    """The pixels of a 32-bit 640x480 SDL_SaveBMP file, top row first."""
    data = Path(path).read_bytes()
    offset = int.from_bytes(data[10:14], "little")
    height = int.from_bytes(data[22:26], "little", signed=True)
    if int.from_bytes(data[18:22], "little") != W or abs(height) != H or data[28] != 32:
        fail(f"{path} is not a 32-bit {W}x{H} BMP", 1)
    pixels = data[offset:offset + FRAME]
    if height < 0:
        return pixels
    row = W * 4
    return b"".join(pixels[y:y + row] for y in range(FRAME - row, -1, -row))


def report(name, status, out):
    if status != 0:
        log = out / "stdout.log"
        tail = log.read_text(errors="replace").splitlines()[-15:] if log.exists() else []
        print(f"replay_video: {name} exited with {status}", *tail, sep="\n", file=sys.stderr)
    return status


def kill(proc):
    if proc.poll() is None:
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        proc.wait()


def streams(binary):
    with open(binary, "rb") as f:
        return b"SMW_SHOT_STREAM" in f.read()


def parse_env(items):
    env = {}
    for item in items:
        if "=" not in item:
            fail(f"expected KEY=VALUE, got {item}")
        k, v = item.split("=", 1)
        env[k] = v
    return env


def label(spec, env, given):
    if given:
        return given
    base = Path(spec).name if os.sep in spec else spec
    return " ".join([base, *(f"{k}={v}" for k, v in env.items())])


def ranges(frames, step):
    """Collapses sorted frame numbers `step` apart into 'a-b' strings."""
    out, start, prev = [], None, None
    for f in frames:
        if start is None:
            start = prev = f
        elif f - prev > step:
            out.append((start, prev))
            start = f
        prev = f
    if start is not None:
        out.append((start, prev))
    return [f"{a}" if a == b else f"{a}-{b}" for a, b in out]


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("replay")
    ap.add_argument("--after", required=True, help="binary path or git rev (built with cargo)")
    ap.add_argument("--before", help="binary path or git rev; enables the side-by-side layout")
    ap.add_argument("-o", "--output", help="output MP4 (default <replay name>.mp4)")
    ap.add_argument("--frames", help="frame range a-b to render (default: the whole replay)")
    ap.add_argument("--speed", type=float, default=1.0, help="playback speed; 2 and up also drop frames")
    ap.add_argument("--diff", action="store_true", help="add a panel with the differing pixels and list them")
    ap.add_argument("--scale", type=int, default=2, help="nearest-neighbour scale factor (default 2)")
    ap.add_argument("--before-env", action="append", default=[], metavar="K=V")
    ap.add_argument("--after-env", action="append", default=[], metavar="K=V")
    ap.add_argument("--before-data", help="data dir for --before (default: found next to the binary)")
    ap.add_argument("--after-data", help="data dir for --after")
    ap.add_argument("--label-before")
    ap.add_argument("--label-after")
    ap.add_argument("--cache-dir", default=os.environ.get("SMW_VIDEO_CACHE", "~/.cache/smw-replay-video"))
    args = ap.parse_args()

    replay = Path(args.replay).resolve()
    if not replay.is_file():
        fail(f"no replay {replay}")
    if args.diff and not args.before:
        fail("--diff needs --before")
    if args.speed <= 0 or args.scale < 1:
        fail("--speed and --scale must be positive")
    if not shutil.which("ffmpeg"):
        fail("ffmpeg not found")

    every = max(1, int(args.speed)) if args.speed >= 2 else 1
    total = int(os.environ.get("SMW_FRAMES") or directive(replay, "frames") or 600)
    first, last = segment_start(replay), total - 1
    if args.frames:
        a, _, b = args.frames.partition("-")
        if not a.isdigit() or not b.isdigit() or int(b) < int(a):
            fail("--frames takes a-b")
        first, last = int(a), min(last, int(b))
        if first > last:
            fail(f"--frames starts after the replay's last frame {last}")

    out = Path(args.output or f"{recording_stem(replay)}.mp4").resolve()
    cache = Path(args.cache_dir).expanduser()
    tmp = Path(tempfile.mkdtemp(prefix="smw-video-"))
    builds = Builds(cache, tmp)
    sides, ffmpeg = [], None

    def on_signal(signum, _frame):
        raise SystemExit(128 + signum)

    signal.signal(signal.SIGTERM, on_signal)
    signal.signal(signal.SIGHUP, on_signal)
    status = 1
    try:
        specs = [("after", args.after, args.after_env, args.after_data, args.label_after)]
        if args.before:
            specs.insert(0, ("before", args.before, args.before_env, args.before_data, args.label_before))
        resolved = []
        for name, spec, env, data, given in specs:
            binary, data_dir = builds.resolve(spec, data)
            extra = parse_env(env)
            resolved.append((name, binary, data_dir, extra, f"{name}: {label(spec, extra, given)}"))
        for name, binary, data_dir, extra, _ in resolved:
            kind = Side if streams(binary) else BmpSide
            if kind is BmpSide:
                print(f"replay_video: {binary} has no SMW_SHOT_STREAM; rerunning it per chunk of BMPs", file=sys.stderr)
            sides.append(kind(name, replay, binary, data_dir, extra, first, last, every, tmp / name))

        panels = len(sides) + (1 if args.diff else 0)
        width = W * panels
        fps = FPS * Fraction(args.speed).limit_denominator(1000) / every
        filters = []
        if args.diff:
            filters.append(
                f"[0:v]split=2[base][d];[d]crop={2 * W}:{H}:0:{HEADER},split=2[l][r];"
                f"[l]crop={W}:{H}:0:0[a];[r]crop={W}:{H}:{W}:0,split=2[b][bb];"
                f"[a][b]blend=all_mode=difference,colorchannelmixer=rr=1:rg=1:rb=1,"
                f"lutrgb=r='if(val,255,0)':g=0:b=0[mask];"
                f"[bb]hue=s=0,lutrgb=r=val*0.35:g=val*0.35:b=val*0.35[dim];"
                f"[dim][mask]blend=all_mode=lighten[diff];"
                f"[base][diff]overlay={2 * W}:{HEADER}[v];[v]")
        else:
            filters.append("[0:v]")
        filters.append(f"scale=iw*{args.scale}:ih*{args.scale}:flags=neighbor,format=yuv420p[out]")
        cmd = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
               "-f", "rawvideo", "-pix_fmt", "bgra", "-s", f"{width}x{H + HEADER}",
               "-framerate", f"{fps.numerator}/{fps.denominator}", "-i", "-",
               "-filter_complex", "".join(filters), "-map", "[out]",
               "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-tune", "animation",
               "-movflags", "+faststart", str(out)]
        ffmpeg = subprocess.Popen(cmd, stdin=subprocess.PIPE)

        header = bytearray(BG * width * HEADER)
        for i, (*_, text) in enumerate(resolved):
            draw_text(header, width, i * W + 12, text[: (W - 150) // ADVANCE])
        blank = bytes(W * 4)
        differ, count = [], 0
        while True:
            frames = [s.read() for s in sides]
            if all(s.done for s in sides):
                break
            if any(f is None for f in frames):
                break
            n = first + count * every
            for s in sides:
                if s.done and s.ended is None:
                    s.ended = n - every
            buf = bytearray(header)
            for i in range(len(sides)):
                draw_text(buf, width, (i + 1) * W - 12, f"F {n}", right=True)
            if args.diff:
                same = frames[0] == frames[1]
                if not same:
                    differ.append(n)
                draw_text(buf, width, 2 * W + 12, "diff: same" if same else "diff: differs",
                          GREEN if same else RED)
            if len(sides) == 1:
                body = frames[0]
            else:
                rows = [memoryview(f) for f in frames]
                parts = []
                for y in range(0, FRAME, W * 4):
                    parts += [r[y:y + W * 4] for r in rows]
                    if args.diff:
                        parts.append(blank)
                body = b"".join(parts)
            ffmpeg.stdin.write(buf)
            ffmpeg.stdin.write(body)
            count += 1
        ffmpeg.stdin.close()
        fstatus = ffmpeg.wait()
        ffmpeg = None
        statuses = [s.finish() for s in sides]
        if count == 0:
            fail("no frames arrived", 1)
        if fstatus != 0 or any(statuses):
            return 1
        for s in sides:
            if s.ended is not None:
                print(f"replay_video: {s.name} stopped after frame {s.ended}; its last frame is held", file=sys.stderr)
        print(f"wrote {out}: {count} frames from frame {first}, every {every}, at {float(fps):g} fps")
        if args.diff:
            if differ:
                print(f"differing frames ({len(differ)}): {', '.join(ranges(differ, every))}")
            else:
                print("no differing frames")
        status = 0
    except BrokenPipeError:
        print("replay_video: ffmpeg exited early", file=sys.stderr)
    except KeyboardInterrupt:
        status = 130
    finally:
        if ffmpeg is not None:
            ffmpeg.kill()
            ffmpeg.wait()
        for s in sides:
            s.kill()
        builds.cleanup()
        shutil.rmtree(tmp, ignore_errors=True)
        if status != 0:
            out.unlink(missing_ok=True)
    return status


if __name__ == "__main__":
    sys.exit(main())
