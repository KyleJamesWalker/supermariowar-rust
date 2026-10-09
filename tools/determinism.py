#!/usr/bin/env python3
"""Cross-platform determinism check (docs/ARCHITECTURE_V2.md, Parity policy timeline): report-only until phase 6.

  determinism.py manifest <job> <suite> <out_root> [<suite> <out_root> ...] > manifest.txt
      One line per artifact of every replay under each out_root (a parity.sh, parity_sweep.sh, editor_parity.sh or
      web_replay.mjs output tree): "<suite>/<replay>\\t<artifact>\\t<sha256>". dump.txt and saved files hash as bytes;
      screenshots (.bmp, .png) hash their RGB pixels, since BMP headers and PNG encoders differ between SDL builds.
  determinism.py compare <summary.md> <manifest.txt> ...
      Compares the replays the manifests share and writes a Markdown summary naming each replay that differs and its
      first differing artifact. Always exits 0.
"""
import hashlib
import re
import struct
import sys
import zlib
from pathlib import Path

SKIP = {"stdout.log"}


def rgb_rows_bmp(data):
    off, = struct.unpack_from("<I", data, 10)
    w, h, _, bpp, _ = struct.unpack_from("<iiHHI", data, 18)
    step = bpp // 8
    stride = (w * step + 3) & ~3
    for y in range(abs(h)):
        src_y = abs(h) - 1 - y if h > 0 else y
        row = bytearray(data[off + src_y * stride: off + src_y * stride + w * step])
        if step == 4:
            del row[3::4]
        row[0::3], row[2::3] = row[2::3], row[0::3]  # BGR to RGB
        yield bytes(row)


def rgb_rows_png(data):
    pos, idat = 8, b""
    while pos < len(data):
        length, kind = struct.unpack_from(">I4s", data, pos)
        body = data[pos + 8: pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or ctype not in (2, 6) or interlace:
                raise ValueError("unsupported PNG")
            step = 3 if ctype == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = w * step
    prev = bytearray(stride)
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
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else (b if pb <= pc else c))) & 0xFF
        prev = line
        row = bytearray(line)
        if step == 4:
            del row[3::4]
        yield bytes(row)


def artifact_hash(path):
    data = path.read_bytes()
    try:
        if data[:2] == b"BM":
            rows = rgb_rows_bmp(data)
        elif data[:8] == b"\x89PNG\r\n\x1a\n":
            rows = rgb_rows_png(data)
        else:
            return "bytes:" + hashlib.sha256(data).hexdigest()
        h = hashlib.sha256()
        for row in rows:
            h.update(row)
        return "rgb:" + h.hexdigest()
    except (ValueError, struct.error, zlib.error):
        return "bytes:" + hashlib.sha256(data).hexdigest()


def sort_key(artifact):
    """dump.txt first, then screenshots by frame, then everything else by path."""
    m = re.fullmatch(r"frame_(\d+)\.(bmp|png)", artifact)
    if artifact == "dump.txt":
        return (0, 0, artifact)
    return (1, int(m.group(1)), artifact) if m else (2, 0, artifact)


def replay_dirs(root):
    """Directories holding a dump.txt: <root>/<name> for game, sweep and web, <root>/<editor>/<name> for editors."""
    for dump in sorted(Path(root).rglob("dump.txt")):
        yield dump.parent


def manifest(args):
    job, pairs = args[0], args[1:]
    print(f"# job {job}")
    for suite, root in zip(pairs[0::2], pairs[1::2]):
        if not Path(root).is_dir():
            continue
        for d in replay_dirs(root):
            replay = f"{suite}/{d.relative_to(root).as_posix()}"
            files = [p for p in d.rglob("*") if p.is_file() and p.name not in SKIP]
            for p in sorted(files, key=lambda p: sort_key(p.relative_to(d).as_posix())):
                print(f"{replay}\t{p.relative_to(d).as_posix()}\t{artifact_hash(p)}")


def load(path):
    job, replays = Path(path).stem, {}
    for line in Path(path).read_text().splitlines():
        if line.startswith("# job "):
            job = line[6:]
        elif line and not line.startswith("#"):
            replay, artifact, digest = line.split("\t")
            replays.setdefault(replay, {})[artifact] = digest
    return job, replays


def compare(args):
    out, paths = Path(args[0]), args[1:]
    jobs = [load(p) for p in paths]
    names = sorted({r for _, replays in jobs for r in replays})
    shared, differ = 0, []
    for replay in names:
        having = [(job, replays[replay]) for job, replays in jobs if replay in replays]
        if len(having) < 2:
            continue
        shared += 1
        common = set.intersection(*(set(a) for _, a in having))
        for artifact in sorted(common, key=sort_key):
            values = {}
            for job, arts in having:
                values.setdefault(arts[artifact], []).append(job)
            if len(values) > 1:
                groups = " vs ".join(", ".join(js) for js in sorted(values.values()))
                differ.append((replay, artifact, groups))
                break
        else:
            missing = [job for job, a in having if set(a) != set().union(*(set(b) for _, b in having))]
            if missing:
                differ.append((replay, "(artifact sets differ)", ", ".join(missing)))

    lines = ["### Determinism across jobs (report-only until phase 6)", ""]
    lines.append("| Job | Replays |")
    lines.append("|---|---|")
    for job, replays in jobs:
        lines.append(f"| {job} | {len(replays)} |")
    lines.append("")
    lines.append(f"{shared} replays ran in two or more jobs; {shared - len(differ)} are identical everywhere they ran, {len(differ)} differ.")
    if differ:
        lines += ["", "| Replay | First differing artifact | Jobs that agree with each other |", "|---|---|---|"]
        lines += [f"| {r} | {a} | {g} |" for r, a, g in differ]
    out.write_text("\n".join(lines) + "\n")
    print(out.read_text())


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in ("manifest", "compare"):
        print(__doc__, file=sys.stderr)
        return 2
    (manifest if sys.argv[1] == "manifest" else compare)(sys.argv[2:])
    return 0


if __name__ == "__main__":
    sys.exit(main())
