#!/usr/bin/env python3
"""Extend a replay by probing the C++ reference: whenever the dump stalls (game over screen,
or a menu whose M line has not changed for --stall frames), press Return there and re-run.

  grow_replay.py <replay.txt> [--frames N] [--stall 90] [--rounds 40]

Rewrites the replay in place (keeps its #@ header, updates frames=). Needs the C++ reference built (see REPLAY.md).
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent


def run(replay, frames):
    with tempfile.TemporaryDirectory(prefix="grow-") as out:
        env = dict(os.environ, SMW_FRAMES=str(frames), SMW_SHOT_FRAMES="")
        subprocess.run([str(TOOLS / "run_ref.sh"), str(replay), out], env=env, stdout=subprocess.DEVNULL, check=False)
        return (Path(out) / "dump.txt").read_text().splitlines()


def blocks(lines):
    cur = None
    for l in lines:
        if l.startswith("F "):
            if cur:
                yield cur
            cur = [l]
        elif cur is not None:
            cur.append(l)
    if cur:
        yield cur


def find_stall(lines, after, stall):
    """First frame >= after where the game sits on a game-over screen or an unchanged menu for `stall` frames."""
    run_key, run_start = None, None
    for b in blocks(lines):
        f = int(b[0].split()[1])
        state = b[0].split()[2]
        key = None
        if state == "gameplay":
            g = next((l for l in b if l.startswith("G ")), "")
            if "gameover=1" in g:
                key = "gameover"
        elif state == "menu":
            key = next((l for l in b if l.startswith("M ")), None)
        if key is None or key != run_key:
            run_key, run_start = key, f
            continue
        if f >= after and f - max(run_start, after) >= stall:
            return f
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("replay")
    ap.add_argument("--frames", type=int)
    ap.add_argument("--stall", type=int, default=90)
    ap.add_argument("--rounds", type=int, default=40)
    args = ap.parse_args()

    path = Path(args.replay)
    text = path.read_text()
    header = [l for l in text.splitlines() if l.startswith("#")]
    events = [l for l in text.splitlines() if l and not l.startswith("#")]
    frames = args.frames or int(re.search(r"#@ frames=(\d+)", text).group(1))
    header = [re.sub(r"#@ frames=\d+", f"#@ frames={frames}", l) for l in header]

    last = max((int(e.split()[0]) for e in events), default=0)
    for _ in range(args.rounds):
        path.write_text("\n".join(header + events) + "\n")
        stall = find_stall(run(path, frames), last + 1, args.stall)
        if stall is None:
            break
        events += [f"{stall} down Return", f"{stall + 2} up Return"]
        last = stall + 2
        print(f"Return at {stall}", file=sys.stderr)
    path.write_text("\n".join(header + events) + "\n")


if __name__ == "__main__":
    main()
