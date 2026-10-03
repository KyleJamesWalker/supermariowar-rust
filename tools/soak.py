#!/usr/bin/env python3
"""Soak test: random games on the C++ reference and the Rust port, diffed frame by frame.

  soak.py [--cases N] [--start S] [--jobs J] [--frames F] [--out DIR] [--rust-bin BIN] [--ref-dir DIR]

Case <seed> builds a replay from python random seed <seed>: random option edits, 2-4 CPUs plus 0-2
fuzzed human players, a random game mode and map. With --flow-share P, a share P of the cases instead
start a CPU-only tournament, tour, world or minigame through the Match Selection menu. Both builds run it in run_ref.sh sandboxes (in
parallel) and the dumps are diffed. Matching cases are deleted; for each divergence or crash the
output dir keeps <seed>/replay.txt, diff.txt (first divergence with context) and both stdout logs.
The same --start/--cases always produce the same replays, so a case reruns with --start S --cases 1.
"""

import argparse
import concurrent.futures
import os
import random
import shutil
import subprocess
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
PORT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import gen_replays as g  # noqa: E402

DATA = PORT / "data"
MAPS = sorted(p.name for p in (DATA / "maps").glob("*.map"))

# Option edits known to navigate cleanly (from gen_replays.py's opt_* replays), as (submenu, fields).
OPTION_EDITS = [
    (0, [3, 1, 2, 1, 2, 1, 1, 2, 1, 1]),
    (2, [["Down", "Return", "Right", "Right", "Right", "Return"]]),
    (3, [2, 1, 1, 1, 1, "t"]),
    (4, [1, 1, 1, 2, 1, 1, 1, 1, "t", 1, 1, 1]),
    (5, [2, 1, 1, 1, 1, 1, 1, 1, 1]),
    (7, [1, 2, 1, "t", "t", "t", "t", "t"]),
    (1, [1, "t", 1]),
    (6, ["t"]),
]

# Players field deltas per slot (Down: none -> human -> CPU -> none); slots 0/1 start human, 2/3 none.
TO = {0: {"human": [], "cpu": ["Down"], "none": ["Down", "Down"]},
      1: {"human": [], "cpu": ["Down"], "none": ["Down", "Down"]},
      2: {"human": ["Down"], "cpu": ["Up"], "none": []},
      3: {"human": ["Down"], "cpu": ["Up"], "none": []}}

FUZZ_KEYS = [["Left", "Right", "Up", "Down", "Right Ctrl", "Right Shift"], ["A", "D", "W", "S", "E", "Q"]]


TOURS = len(list((DATA / "tours").glob("*.txt")))
WORLDS = len(list((DATA / "worlds").glob("*.txt")))


def make_flow_replay(rng, seed, frames):
    """Match Selection flows (see gen_replays.flow): CPU players only, since Team Select would wait on humans."""
    s = g.Script()
    g.boot_to_main(s)
    cpus = rng.randint(2, 4)
    slots = ["none"] * 4
    for i in rng.sample(range(4), cpus):
        slots[i] = "cpu"
    g.set_players(s, [TO[i][slots[i]] for i in range(4)], order=(2, 3, 0, 1))
    s.tap("Return", gap=30)   # Start -> Match Selection
    kind, rights, sub, returns = rng.choice([
        ("tournament", 1, 0, 3),
        ("tour", 2, rng.randrange(TOURS), 3),
        ("world", 3, rng.randrange(WORLDS), 3),
        ("minigame", 4, rng.randrange(5), 2),
    ])
    g.match_type(s, rights, sub)
    for _ in range(returns):
        s.tap("Return", gap=40)
    mapname = rng.choice(MAPS)
    header = (f"# soak case {seed}: players {','.join(slots)}, {kind} (match type {rights}, sub {sub}), map '{mapname}'.\n"
              f"#@ seed={rng.randrange(1, 2**31)}\n#@ map={mapname}\n#@ frames={frames}\n")
    return s, header


def make_replay(seed, frames, flow_share=0.0):
    rng = random.Random(seed)
    if flow_share > 0 and rng.random() < flow_share:
        return make_flow_replay(rng, seed, frames)
    s = g.Script()
    g.boot_to_main(s)

    edits = sorted(rng.sample(OPTION_EDITS, rng.randint(0, 3)), key=lambda e: e[0])
    if edits:
        g.edit_options(s, edits)

    # Only slots 0 and 1 have keyboard bindings, so humans go there; CPUs fill the rest.
    cpus = rng.randint(2, 4)
    humans = rng.randint(0, min(2, 4 - cpus))
    slots = ["none"] * 4
    for i in range(humans):
        slots[i] = "human"
    free = [i for i in range(4) if slots[i] == "none"]
    for i in rng.sample(free, cpus):
        slots[i] = "cpu"
    g.set_players(s, [TO[i][slots[i]] for i in range(4)], order=(2, 3, 0, 1))

    s.tap("Return", gap=30)   # Start -> Match Selection
    s.tap("Return", gap=30)   # Start -> Team Select
    if slots[1] == "human":
        s.tap("E", gap=30)
    if slots[0] == "human":
        s.tap("Return", gap=30)
    s.tap("Return", gap=30)   # -> Game Settings
    s.wait(10)
    mode = rng.randrange(len(g.GAME_MODES))
    g.game_settings_mode(s, mode)
    s.tap("Return", gap=40)   # Start

    for p in range(2):
        if slots[p] != "human":
            continue
        busy = {k: 0 for k in FUZZ_KEYS[p]}
        f = s.f
        while f < frames - 10:
            k = rng.choice(FUZZ_KEYS[p])
            if busy[k] <= f:
                n = rng.choice([1, 2, 3, 5, 8, 15, 30, 60])
                s.hold(f, k, n)
                busy[k] = f + n + 1
            f += rng.choice([1, 2, 3, 4, 6, 10])

    mapname = rng.choice(MAPS)
    header = (f"# soak case {seed}: players {','.join(slots)}, mode {g.GAME_MODES[mode]}, map '{mapname}', "
              f"option edits {[e[0] for e in edits]}.\n"
              f"#@ seed={rng.randrange(1, 2**31)}\n#@ map={mapname}\n#@ frames={frames}\n")
    return s, header


def run(bin_path, data_dir, replay, out):
    env = dict(os.environ, SMW_BIN=str(bin_path), SMW_DATA_DIR=str(data_dir), SMW_SHOT_FRAMES="")
    return subprocess.run([str(TOOLS / "run_ref.sh"), str(replay), str(out)], env=env,
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode


def case(seed, args):
    d = args.out / str(seed)
    d.mkdir(parents=True, exist_ok=True)
    s, header = make_replay(seed, args.frames, args.flow_share)
    s.write("replay", header, d)
    replay = d / "replay.txt"

    with concurrent.futures.ThreadPoolExecutor(2) as ex:
        fc = ex.submit(run, args.ref_dir / "build" / "smw", args.ref_dir / "data", replay, d / "cpp")
        fr = ex.submit(run, args.rust_bin, DATA, replay, d / "rust")
        cst, rst = fc.result(), fr.result()

    diff = subprocess.run([str(TOOLS / "diffreplay.py"), "--context", "3", str(d / "cpp/dump.txt"), str(d / "rust/dump.txt")],
                          capture_output=True, text=True)
    first = (diff.stdout or diff.stderr).splitlines()[0] if (diff.stdout or diff.stderr) else ""
    if cst == 0 and rst == 0 and diff.returncode == 0:
        shutil.rmtree(d)
        return seed, "ok", first

    (d / "diff.txt").write_text(f"cpp exit {cst}, rust exit {rst}\n" + diff.stdout + diff.stderr)
    for side in ("cpp", "rust"):
        log = d / side / "stdout.log"
        if log.exists():
            shutil.copy(log, d / f"{side}_stdout.log")
        if (d / side / "dump.txt").exists():
            subprocess.run(["gzip", "-f", str(d / side / "dump.txt")])
    kind = "cpp-crash" if cst != 0 and rst == 0 else "rust-crash" if rst != 0 and cst == 0 else "both-crash" if cst and rst else "diverge"
    (d / "kind").write_text(kind + "\n")
    return seed, kind, first


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cases", type=int, default=100)
    ap.add_argument("--start", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2), help="cases at once (2 processes each)")
    ap.add_argument("--frames", type=int, default=5000)
    ap.add_argument("--flow-share", type=float, default=0.0, help="share of cases that run a tournament/tour/world/minigame flow")
    ap.add_argument("--out", type=Path, default=Path(os.environ.get("TMPDIR", "/tmp")) / "smw-soak")
    ap.add_argument("--rust-bin", type=Path, default=PORT / "target/release/smw")
    ap.add_argument("--ref-dir", type=Path, default=Path.home() / "work/supermariowar-cpp-reference")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    counts = {}
    seeds = range(args.start, args.start + args.cases)
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as ex:
        for seed, kind, first in ex.map(lambda sd: case(sd, args), seeds):
            counts[kind] = counts.get(kind, 0) + 1
            if kind != "ok":
                print(f"{seed}: {kind}: {first}", flush=True)
    print("summary: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())), flush=True)
    return 0 if counts.get("ok", 0) == args.cases else 1


if __name__ == "__main__":
    sys.exit(main())
