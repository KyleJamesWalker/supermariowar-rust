#!/usr/bin/env python3
"""Compare the host and joiner dumps of one net game (see net_game_interop.sh).

The host simulates every player; the joiner predicts its own player and interpolates the others
from the host's game state packets, so the two trajectories agree up to a frame offset and a little
interpolation lag. Each player's (fx, fy) track is aligned at the offset with the most exact matches, up to each
client's game over: after it, a client that ended the match removes players.

--spawns also compares the random outcomes both clients recorded (`C` records, Rust only, see docs/REPLAY.md): for each
kind, the joiner's records must be the host's, in any order. The joiner may still be missing the host's records from
the last SPAWN_TAIL gameplay frames. --min KIND=N fails when the host recorded fewer than N of KIND (--min-spawns N
is --min powerup=N). With --spawns, a game over must also come within GAMEOVER_SLACK gameplay frames on both
clients. --min-close overrides MIN_CLOSE for scenarios whose tracks drift for a few frames per block bump,
as they do between two C++ clients.
"""
import argparse
import sys
from collections import Counter

MIN_FRAMES = 300
SLACK = 4
TOLERANCE = 2.0
MIN_CLOSE = 0.95
SPAWN_TAIL = 120
GAMEOVER_SLACK = 10


def parse(path):
    menus, frames, spawns, gameover = [], [], [], None
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
        elif f[0] == 'G' and gameover is None and 'gameover=1' in f:
            gameover = len(frames) - 1
        elif f[0] == 'C' and frames:
            spawns.append((len(frames) - 1, f[1], ' '.join(f[2:])))
    return menus, frames, spawns, gameover


def compare_spawns(host, hspawns, jspawns, minimums):
    ok = True
    kinds = sorted({s[1] for s in hspawns} | {s[1] for s in jspawns} | set(minimums))
    for kind in kinds:
        h = [s for s in hspawns if s[1] == kind]
        j = [s for s in jspawns if s[1] == kind]
        extra = Counter(s[2] for s in j) - Counter(s[2] for s in h)
        missing = Counter(s[2] for s in h) - Counter(s[2] for s in j)
        late = [s for s in h if s[0] >= len(host) - SPAWN_TAIL]
        unexplained = missing - Counter(s[2] for s in late)
        good = not extra and not unexplained and len(h) >= minimums.get(kind, 0)
        ok &= good
        print(f'  {kind}: host {len(h)}, join {len(j)}: ' + ('match' if good else 'MISMATCH'))
        if not good:
            for text, n in list(extra.items())[:8]:
                frames = [s[0] for s in j if s[2] == text]
                print(f'    join only (frames {frames}): {text}' + (f' x{n}' if n > 1 else ''))
            for text, n in list(unexplained.items())[:8]:
                frames = [s[0] for s in h if s[2] == text]
                print(f'    host only (frames {frames}): {text}' + (f' x{n}' if n > 1 else ''))
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
    ap.add_argument('--min', action='append', default=[], metavar='KIND=N')
    ap.add_argument('--min-close', type=float, default=MIN_CLOSE)
    args = ap.parse_args()
    hmenus, host, hspawns, hend = parse(args.host)
    jmenus, join, jspawns, jend = parse(args.join)
    print('  host menus: ' + ' > '.join(hmenus))
    print('  join menus: ' + ' > '.join(jmenus))
    print(f'  gameplay frames: host {len(host)}, join {len(join)}')
    ok = len(host) >= MIN_FRAMES and len(join) >= MIN_FRAMES
    pids = sorted(set().union(*host) if host else set())
    if len(pids) < 2:
        ok = False
    if hend is not None or jend is not None:
        print(f'  game over at gameplay frame: host {hend}, join {jend}; tracks compared until then')
    for pid in pids:
        r = align(host[:hend], join[:jend], pid)
        if r is None:
            print(f'  player {pid}: no overlap')
            ok = False
            continue
        off, exact, close, n, moving, worst = r
        print(f'  player {pid}: offset {off:+d}, exact {exact}/{n}, within {TOLERANCE:g}px (+-{SLACK} frames) {close}/{n}, '
              f'worst {worst:.1f}px, distinct positions {moving}')
        if close < args.min_close * n or moving < 10:
            ok = False
    minimums = {k: int(n) for k, n in (m.split('=', 1) for m in args.min)}
    if args.min_spawns:
        minimums['powerup'] = args.min_spawns
    if args.spawns and not compare_spawns(host, hspawns, jspawns, minimums):
        ok = False
    if args.spawns and (hend is not None or jend is not None):
        late = hend is not None and jend is None and hend >= len(host) - SPAWN_TAIL
        same = hend is not None and jend is not None and abs(hend - jend) <= GAMEOVER_SLACK
        print('  game over: ' + ('same frame' if same else 'joiner not there yet' if late else 'DIFFERENT FRAMES'))
        ok &= same or late
    print('  ' + ('SYNCED' if ok else 'NOT SYNCED'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
