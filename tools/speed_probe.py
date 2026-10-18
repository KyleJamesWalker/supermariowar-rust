#!/usr/bin/env python3
"""Compare game speed in wall time between two builds that tick at different rates.

Each scenario is player 1's input on a timeline in seconds. The probe turns it into a replay per build at that
build's tick rate, runs both headless, and compares player 1's position (and, with --objects, every object's) as a
function of time since gameplay started. A build whose per-tick motion is scaled to its tick rate tracks the other
to within a pixel or two; an unscaled one drifts by the ratio of the rates.

Usage: speed_probe.py --base <smw_replay> --base-hz 62.5 --new <smw_replay> --new-hz 60 [--data DIR] [scenario ...]
"""

import argparse
import os
import subprocess
import sys
import tempfile

TOOLS = os.path.dirname(os.path.abspath(__file__))
PORT = os.path.dirname(TOOLS)

# Boot and start a default 2-player Classic game on the first map (start_classic.txt's menu input). Player 1
# spawns to player 2's left, so the scenarios move left to stay clear of it.
MENU = """#@ seed=1
10 down Return
12 up Return
30 down Return
32 up Return
50 down Return
52 up Return
70 down E
72 up E
100 down Return
102 up Return
130 down Return
132 up Return
160 down Return
162 up Return
164 await gameplay
"""
AWAIT_FRAME = 164

# name: (seconds of gameplay to run, [(seconds since gameplay started, event), ...])
SCENARIOS = {
    "idle": (5.0, []),
    "walk": (6.5, [(5.00, "down Left"), (6.00, "up Left")]),
    "run": (6.5, [(5.00, "down Right Ctrl"), (5.00, "down Left"), (5.80, "up Left"), (5.80, "up Right Ctrl")]),
    "skid": (6.5, [(5.00, "down Right Ctrl"), (5.00, "down Left"), (5.60, "up Left"), (5.60, "down Right"), (5.90, "up Right"), (5.90, "up Right Ctrl")]),
    "hop": (6.5, [(5.00, "down Up"), (5.05, "up Up")]),
    "jump": (7.0, [(5.00, "down Up"), (5.60, "up Up")]),
    "runjump": (6.8, [(5.00, "down Right Ctrl"), (5.00, "down Left"), (5.30, "down Up"), (5.80, "up Up"), (5.80, "up Left"), (5.80, "up Right Ctrl")]),
    "fall": (7.0, [(5.00, "down Down"), (6.00, "up Down")]),
}


def replay_text(scenario, hz):
    seconds, events = SCENARIOS[scenario]
    lines = [MENU.rstrip("\n")]
    for t, event in events:
        lines.append(f"{AWAIT_FRAME + 1 + round(t * hz)} {event}")
    lines.append(f"#@ frames={AWAIT_FRAME + 1 + round(seconds * hz) + 400}")
    return "\n".join(lines) + "\n"


def run(binary, data, text, objects):
    with tempfile.TemporaryDirectory() as tmp:
        replay = os.path.join(tmp, "probe.txt")
        with open(replay, "w") as f:
            f.write(text)
        out = os.path.join(tmp, "out")
        env = dict(os.environ, SMW_BIN=binary, SMW_DATA_DIR=data, SMW_NOLIMIT="1")
        if objects:
            env["SMW_DUMP_OBJECTS"] = "1"
        subprocess.run([os.path.join(TOOLS, "run_ref.sh"), replay, out], env=env, check=True, stdout=subprocess.DEVNULL)
        with open(os.path.join(out, "dump.txt")) as f:
            return parse(f)


def fields(line):
    return dict(kv.split("=", 1) for kv in line.split()[1:] if "=" in kv)


def parse(f):
    """[(player 1 fx, fy, state), objects] per gameplay frame, from the first gameplay frame on."""
    frames, cur, gameplay = [], None, False
    for line in f:
        if line.startswith("F "):
            gameplay = line.split()[2] == "gameplay"
            if gameplay:
                cur = {"p": None, "x": []}
                frames.append(cur)
        elif gameplay and line.startswith("P ") and cur["p"] is None:
            p = fields(line)
            cur["p"] = (float(p["fx"]), float(p["fy"]), int(p["state"]))
        elif gameplay and line.startswith("X "):
            cur["x"].append(fields(line))
    return frames


def at(frames, hz, t):
    """Player 1's (fx, fy) at time t, interpolated between ticks."""
    f = t * hz
    i = int(f)
    if i + 1 >= len(frames):
        return None
    a, b = frames[i]["p"], frames[i + 1]["p"]
    if a is None or b is None or abs(a[0] - b[0]) > 320:
        return a
    w = f - i
    return (a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w)


def compare(name, base, base_hz, new, new_hz, step=0.05):
    seconds, events = SCENARIOS[name]
    start = min((t for t, _ in events), default=1.25) - 0.25
    worst = (0.0, 0.0, start)
    rows = []
    t = start
    while t <= seconds + 1e-9:
        a = at(base, base_hz, t)
        # Within one tick of either build, so a teleport (spawn, warp, wrap) a tick apart is not a miss.
        tick = 1.0 / min(base_hz, new_hz)
        best = None
        for b in (at(new, new_hz, t + s) for s in (0.0, -tick, tick)):
            if a and b:
                dx, dy = b[0] - a[0], b[1] - a[1]
                if abs(dx) > 320:
                    dx -= 640 * round(dx / 640)
                if best is None or max(abs(dx), abs(dy)) < max(abs(best[1]), abs(best[2])):
                    best = (b, dx, dy)
        if best:
            b, dx, dy = best
            if max(abs(dx), abs(dy)) > max(abs(worst[0]), abs(worst[1])):
                worst = (dx, dy, t)
            rows.append((t, a, b, dx, dy))
        t += step
    return worst, rows


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True)
    ap.add_argument("--base-hz", type=float, default=62.5)
    ap.add_argument("--base-data", default=os.path.join(PORT, "data"))
    ap.add_argument("--new", required=True)
    ap.add_argument("--new-hz", type=float, default=60.0)
    ap.add_argument("--new-data", default=os.path.join(PORT, "data"))
    ap.add_argument("--tolerance", type=float, default=2.0, help="pixels; exit 1 if any scenario drifts further")
    ap.add_argument("--verbose", action="store_true", help="print the sampled positions")
    ap.add_argument("scenarios", nargs="*", default=list(SCENARIOS))
    args = ap.parse_args()

    failed = False
    for name in args.scenarios:
        base = run(args.base, args.base_data, replay_text(name, args.base_hz), False)
        new = run(args.new, args.new_data, replay_text(name, args.new_hz), False)
        (dx, dy, t), rows = compare(name, base, args.base_hz, new, args.new_hz)
        ok = max(abs(dx), abs(dy)) <= args.tolerance
        failed |= not ok
        print(f"{name:10s} {'ok  ' if ok else 'FAIL'} worst dx={dx:+7.2f} dy={dy:+7.2f} px at t={t:.2f} s")
        if args.verbose:
            for t, a, b, dx, dy in rows:
                print(f"    t={t:5.2f} base=({a[0]:7.2f},{a[1]:7.2f}) new=({b[0]:7.2f},{b[1]:7.2f}) d=({dx:+6.2f},{dy:+6.2f})")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
