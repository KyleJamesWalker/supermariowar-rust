#!/usr/bin/env python3
"""Compare two replay dumps (and optionally screenshot sets). See REPLAY.md.

  diffreplay.py expected/dump.txt[.gz] actual/dump.txt [--context 2] [--ignore R]
  diffreplay.py --shots expected_dir actual_dir [--max-diff 0]

Exit status: 0 identical, 1 divergence, 2 usage or I/O error.
"""

import argparse
import difflib
import gzip
import re
import struct
import sys
import zlib
from pathlib import Path


def parse_frames(path):
    """Split a dump into [(frame_no, [lines])], one entry per `F` block."""
    frames = []
    data = Path(path).read_bytes()
    if data[:2] == b"\x1f\x8b":
        data = gzip.decompress(data)
    for raw in data.decode().splitlines():
        if raw.startswith("F "):
            frames.append((int(raw.split()[1]), [raw]))
        elif frames:
            frames[-1][1].append(raw)
        elif raw.strip():
            frames.append((-1, [raw]))
    return frames


NUM = re.compile(r"-?\d+\.\d+")


def lines_equal(a, b, float_tol):
    if a == b:
        return True
    if float_tol <= 0:
        return False
    if NUM.sub("#", a) != NUM.sub("#", b):
        return False
    return all(abs(float(x) - float(y)) <= float_tol
               for x, y in zip(NUM.findall(a), NUM.findall(b)))


def block_equal(a, b, float_tol):
    return len(a) == len(b) and all(lines_equal(x, y, float_tol) for x, y in zip(a, b))


def filtered(lines, ignore):
    return [l for l in lines if not any(l.startswith(p + " ") for p in ignore)]


def diff_dumps(args):
    exp = parse_frames(args.expected)
    act = parse_frames(args.actual)
    ignore = args.ignore or []

    for i in range(min(len(exp), len(act))):
        (fe, le), (fa, la) = exp[i], act[i]
        le, la = filtered(le, ignore), filtered(la, ignore)
        if fe == fa and block_equal(le, la, args.float_tol):
            continue

        print(f"DIVERGED at frame {fe} (block {i}; actual frame {fa})")
        for j in range(max(0, i - args.context), i):
            print(f"--- context frame {exp[j][0]} (identical)")
            for line in filtered(exp[j][1], ignore):
                print(f"    {line}")
        print(f"--- {args.expected}")
        print(f"+++ {args.actual}")
        for line in difflib.unified_diff(le, la, lineterm="", n=len(le) + len(la)):
            if not line.startswith(("---", "+++", "@@")):
                print(line)
        return 1

    if len(exp) != len(act):
        shorter = "actual" if len(act) < len(exp) else "expected"
        n = min(len(exp), len(act))
        nxt = (exp if len(exp) > n else act)[n][0]
        print(f"DIVERGED: {shorter} ends after {n} frames (next frame {nxt} missing)")
        return 1

    print(f"OK: {len(exp)} frames identical")
    return 0


def read_bmp(data):
    if data[:2] != b"BM":
        raise ValueError("not a BMP")
    off, = struct.unpack_from("<I", data, 10)
    w, h, _, bpp, comp = struct.unpack_from("<iiHHI", data, 18)
    if bpp not in (24, 32) or comp not in (0, 3):
        raise ValueError(f"unsupported BMP ({bpp} bpp, compression {comp})")
    step = bpp // 8
    stride = (w * step + 3) & ~3
    rows = []
    for y in range(abs(h)):
        src_y = abs(h) - 1 - y if h > 0 else y
        row = data[off + src_y * stride: off + src_y * stride + w * step]
        rows.append([tuple(row[x * step: x * step + 3][::-1]) for x in range(w)])
    return w, abs(h), rows


def read_png(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, w = 8, b"", 0
    while pos < len(data):
        length, kind = struct.unpack_from(">I4s", data, pos)
        body = data[pos + 8: pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or ctype not in (2, 6) or interlace:
                raise ValueError("unsupported PNG (need 8-bit RGB/RGBA, non-interlaced)")
            step = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = w * step
    prev = bytearray(stride)
    rows = []
    for y in range(h):
        ftype = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1: (y + 1) * (stride + 1)])
        for x in range(stride):
            a = line[x - step] if x >= step else 0
            b = prev[x]
            c = prev[x - step] if x >= step else 0
            if ftype == 1:
                line[x] = (line[x] + a) & 0xFF
            elif ftype == 2:
                line[x] = (line[x] + b) & 0xFF
            elif ftype == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 0xFF
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pred) & 0xFF
        rows.append([tuple(line[x * step: x * step + 3]) for x in range(w)])
        prev = line
    return w, h, rows


def load_image(path):
    try:
        from PIL import Image
        img = Image.open(path).convert("RGB")
        w, h = img.size
        px = list(img.getdata())
        return w, h, [px[y * w:(y + 1) * w] for y in range(h)]
    except ImportError:
        data = Path(path).read_bytes()
        return read_png(data) if data[:4] == b"\x89PNG" else read_bmp(data)


def diff_shots(args):
    exp_dir, act_dir = Path(args.expected), Path(args.actual)
    pat = re.compile(r"frame_(\d+)\.(bmp|png)$")

    def index(d):
        return {int(m.group(1)): p for p in d.iterdir() if (m := pat.search(p.name))}

    exp, act = index(exp_dir), index(act_dir)
    status = 0
    for frame in sorted(set(exp) | set(act)):
        if frame not in exp or frame not in act:
            print(f"frame {frame}: missing in {'actual' if frame in exp else 'expected'}")
            status = 1
            continue
        we, he, re_ = load_image(exp[frame])
        wa, ha, ra = load_image(act[frame])
        if (we, he) != (wa, ha):
            print(f"frame {frame}: size {we}x{he} vs {wa}x{ha}")
            status = 1
            continue
        diff = sum(1 for y in range(he) for x in range(we) if re_[y][x] != ra[y][x])
        verdict = "OK" if diff <= args.max_diff else "DIFF"
        print(f"frame {frame}: {verdict} {diff} differing pixels of {we * he}")
        if diff > args.max_diff:
            status = 1
    return status


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("expected")
    ap.add_argument("actual")
    ap.add_argument("--context", type=int, default=2, help="identical frames to print before the divergence")
    ap.add_argument("--ignore", action="append", metavar="PREFIX",
                    help="drop lines with this record prefix (e.g. R, O); repeatable")
    ap.add_argument("--float-tol", type=float, default=0.0,
                    help="treat %%.4f fields within this tolerance as equal")
    ap.add_argument("--shots", action="store_true", help="compare frame_<n>.{bmp,png} in two directories")
    ap.add_argument("--max-diff", type=int, default=0, help="allowed differing pixels per screenshot")
    args = ap.parse_args()
    try:
        return diff_shots(args) if args.shots else diff_dumps(args)
    except (OSError, ValueError) as err:
        print(f"error: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
