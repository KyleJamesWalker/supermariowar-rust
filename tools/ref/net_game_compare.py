#!/usr/bin/env python3
"""Compare the host and joiner dumps of one net game (see net_game_interop.sh).

The host simulates every player; the joiner predicts its own player and interpolates the others
from the host's game state packets, so the two trajectories agree up to a frame offset and a little
interpolation lag. Each player's (fx, fy) track is aligned at the offset with the most exact matches.

--spawns also compares the powerups both clients spawned (`C` records, Rust only): the same types at the same blocks,
in the same order. The joiner may still be missing the host's spawns from the last SPAWN_TAIL gameplay frames.
--min-spawns N fails when the host spawned fewer than N. --min-close overrides MIN_CLOSE for scenarios whose tracks
drift for a few frames per block bump, as they do between two C++ clients.
"""
import argparse
import sys

MIN_FRAMES = 300
SLACK = 4
TOLERANCE = 2.0
MIN_CLOSE = 0.95
SPAWN_TAIL = 120


def parse(path):
    menus, frames, spawns = [], [], []
    for line in open(path):
        f = line.split()
        if not f:
            continue
        if f[0] == 'F':
            state = f[2]
            if state == 'gameplay':
                frames.append({})
        elif f[0] == 'M' and (not menus or menus[-1] != f[1]):
            menus.append(f[1])
        elif f[0] == 'P' and frames:
            kv = dict(x.split('=', 1) for x in f[1:])
            frames[-1][int(kv['id'])] = (float(kv['fx']), float(kv['fy']), int(kv['state']))
        elif f[0] == 'C' and frames:
            kv = dict(x.split('=', 1) for x in f[1:])
            spawns.append((len(frames) - 1, int(kv['type']), int(kv['x']), int(kv['y'])))
    return menus, frames, spawns


def compare_spawns(host, hspawns, jspawns, min_spawns):
    hs = [s[1:] for s in hspawns]
    js = [s[1:] for s in jspawns]
    tail = sum(1 for s in hspawns if s[0] >= len(host) - SPAWN_TAIL)
    for i in range(max(len(hs), len(js))):
        h = f'frame {hspawns[i][0]} type {hs[i][0]} at {hs[i][1]},{hs[i][2]}' if i < len(hs) else '-'
        j = f'frame {jspawns[i][0]} type {js[i][0]} at {js[i][1]},{js[i][2]}' if i < len(js) else '-'
        mark = '' if i < len(hs) and i < len(js) and hs[i] == js[i] else '  <- differs'
        print(f'    spawn {i}: host {h}; join {j}{mark}')
    ok = js == hs[:len(js)] and len(hs) - len(js) <= tail and len(hs) >= min_spawns
    print(f'  powerup spawns: host {len(hs)}, join {len(js)}: ' + ('match' if ok else 'MISMATCH'))
    return ok


def align(host, join, pid):
    h = [fr.get(pid) for fr in host]
    j = [fr.get(pid) for fr in join]
    best = None
    for off in range(-60, 61):
        pairs = [(h[i], j[i + off]) for i in range(len(h)) if 0 <= i + off < len(j) and h[i] and j[i + off]]
        if not pairs:
            continue
        exact = sum(1 for a, b in pairs if abs(a[0] - b[0]) < 0.01 and abs(a[1] - b[1]) < 0.01)
        if best is None or exact > best[1]:
            best = (off, exact, len(pairs))
    if best is None:
        return None
    off, exact, n = best
    # Network jitter shifts the offset by a few frames, so each host frame matches the nearest joiner frame nearby.
    close, worst = 0, 0.0
    for i in range(len(h)):
        if not h[i] or not 0 <= i + off < len(j) or not j[i + off]:
            continue
        d = min(max(abs(h[i][0] - b[0]), abs(h[i][1] - b[1]))
                for b in j[max(0, i + off - SLACK):i + off + SLACK + 1] if b)
        close += d <= TOLERANCE
        worst = max(worst, d)
    moving = len({(round(a[0]), round(a[1])) for a in h if a})
    return off, exact, close, n, moving, worst


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('host')
    ap.add_argument('join')
    ap.add_argument('--spawns', action='store_true')
    ap.add_argument('--min-spawns', type=int, default=0)
    ap.add_argument('--min-close', type=float, default=MIN_CLOSE)
    args = ap.parse_args()
    hmenus, host, hspawns = parse(args.host)
    jmenus, join, jspawns = parse(args.join)
    print('  host menus: ' + ' > '.join(hmenus))
    print('  join menus: ' + ' > '.join(jmenus))
    print(f'  gameplay frames: host {len(host)}, join {len(join)}')
    ok = len(host) >= MIN_FRAMES and len(join) >= MIN_FRAMES
    pids = sorted(set().union(*host) if host else set())
    if len(pids) < 2:
        ok = False
    for pid in pids:
        r = align(host, join, pid)
        if r is None:
            print(f'  player {pid}: no overlap')
            ok = False
            continue
        off, exact, close, n, moving, worst = r
        print(f'  player {pid}: offset {off:+d}, exact {exact}/{n}, within {TOLERANCE:g}px (+-{SLACK} frames) {close}/{n}, '
              f'worst {worst:.1f}px, distinct positions {moving}')
        if close < args.min_close * n or moving < 10:
            ok = False
    if args.spawns and not compare_spawns(host, hspawns, jspawns, args.min_spawns):
        ok = False
    print('  ' + ('SYNCED' if ok else 'NOT SYNCED'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
