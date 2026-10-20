#!/usr/bin/env python3
"""Renders the title screen through each shader presenter and compares the window capture with
tools/shader_golden/<name>.png. Linux CI runs it under xvfb on Mesa's llvmpipe; the comparison allows small
per-pixel differences, since llvmpipe's rounding can change with the Mesa build.

Usage: tools/shader_shot_test.py [work_dir]
  SMW_BIN       the game, built with --features shaders (default: target/release/smw)
  SMW_DATA_DIR  the data folder (default: data)
  UPDATE=1      write the captures as the new goldens
"""
import os
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from diffreplay import load_image  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
GOLDEN = REPO / "tools" / "shader_golden"
SHOT = 90
# A channel may differ by up to TOLERANCE, and at most MAX_OFF of the pixels by more.
TOLERANCE = 8
MAX_OFF = 0.002
# Name, preset, window scale; CRT Easymode draws scanlines only from 2x up.
CASES = [
    ("crt_easymode_builtin", "builtin:CRT Easymode", 2),
    ("crt_lottes_builtin", "builtin:CRT Lottes", 1),
    ("crt_lottes", str(REPO / "shaders" / "presets" / "CRT Lottes.slangp"), 1),
]


def write_png(path, w, h, rows):
    raw = b"".join(b"\0" + bytes(c for px in row for c in px) for row in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    Path(path).write_bytes(png)


def capture(name, preset, scale, work):
    out = work / name
    out.mkdir(parents=True, exist_ok=True)
    env = dict(
        os.environ,
        HOME=str(out / "home"),
        SMW_SHADER=preset,
        SMW_WINDOW_SCALE=str(scale),
        SMW_SHADER_SHOT=str(SHOT),
        SMW_SHOT_DIR=str(out),
        SMW_FRAMES=str(SHOT + 10),
        SMW_SEED="1",
        SMW_NOLIMIT="1",
        SDL_AUDIODRIVER="dummy",
    )
    smw = os.environ.get("SMW_BIN", str(REPO / "target" / "release" / "smw"))
    data = os.environ.get("SMW_DATA_DIR", str(REPO / "data"))
    log = out / "stdout.log"
    with open(log, "w") as f:
        subprocess.run([smw, "--datadir", data], env=env, stdout=f, stderr=subprocess.STDOUT, timeout=300)
    text = log.read_text(errors="replace")
    shot = out / f"shader_{SHOT}.bmp"
    if "Shader preset unavailable" in text or not shot.exists():
        print(f"{name}: no capture; last output:")
        print("".join(line for line in text.splitlines(True) if "[gfx]" in line)[-2000:])
        return None
    return load_image(shot)


def compare(name, actual):
    golden = GOLDEN / f"{name}.png"
    if not golden.exists():
        print(f"{name}: no golden at {golden}")
        return False
    we, he, exp = load_image(golden)
    wa, ha, act = actual
    if (we, he) != (wa, ha):
        print(f"{name}: size {wa}x{ha}, golden {we}x{he}")
        return False
    off = sum(
        1
        for y in range(he)
        for x in range(we)
        if max(abs(a - b) for a, b in zip(exp[y][x], act[y][x])) > TOLERANCE
    )
    ok = off <= MAX_OFF * we * he
    print(f"{name}: {'OK' if ok else 'DIFF'} {off} pixels off by more than {TOLERANCE} of {we * he}")
    return ok


def main():
    work = Path(sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="smw-shader-shot-")).resolve()
    status = 0
    for name, preset, scale in CASES:
        actual = capture(name, preset, scale, work)
        if actual is None:
            status = 1
        elif os.environ.get("UPDATE") == "1":
            GOLDEN.mkdir(parents=True, exist_ok=True)
            write_png(GOLDEN / f"{name}.png", *actual)
            print(f"{name}: golden written")
        elif not compare(name, actual):
            status = 1
    print(f"captures in {work}")
    return status


if __name__ == "__main__":
    sys.exit(main())
