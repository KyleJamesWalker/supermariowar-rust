#!/usr/bin/env python3
"""AI-vs-AI benchmark: plays the 4-CPU replays (tools/replays/cpu_*.txt) over many seeds and maps and reports how
the CPU players do per mode. Each match is a replay run with a new seed and map, so results are deterministic.

Usage: ai_bench.py [--modes classic,ctf] [--seeds 1-8] [--maps "0smw,Rubber Room"] [--frames 7200]
                   [--jobs 8] [--json out.json] [--compare before.json]
SMW_BIN selects the game binary (default target/release/smw, built unless SMW_NO_BUILD=1).
"""
import argparse
import concurrent.futures
import gzip
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
PORT = TOOLS.parent
DEFAULT_MAPS = ['0smw', '2skyfight', 'Rubber Room', 'Wacky Woods', 'Bouncy Spikes', 'Block Piles']
STUCK_FRAMES = 30
TRAP_WINDOW = 300
PLAYER = re.compile(r'^P id=(\d+) .*?ix=(-?\d+) iy=(-?\d+) .*?velx=(-?[\d.]+) .*?state=(\d+) score=(-?\d+) .*?inair=(\d)')
GAME = re.compile(r'^G mode=(-?\d+) gameover=(\d) winner=(-?\d+)')


def parse_range(text):
    out = []
    for part in text.split(','):
        lo, _, hi = part.partition('-')
        out.extend(range(int(lo), int(hi or lo) + 1))
    return out


def script_for(src, seed, map_name, frames):
    lines = []
    for line in src.read_text().splitlines():
        if line.startswith(('#@ seed=', '#@ map=', '#@ frames=', '#@ shots=')):
            continue
        lines.append(line)
    header = [f'#@ seed={seed}', f'#@ map={map_name}', f'#@ frames={frames}']
    return '\n'.join(lines[:1] + header + lines[1:]) + '\n'


def play(job):
    mode, src, seed, map_name, frames = job
    with tempfile.TemporaryDirectory(prefix='smw-aibench-') as tmp:
        script = Path(tmp) / f'{mode}.txt'
        script.write_text(script_for(src, seed, map_name, frames))
        out = Path(tmp) / 'out'
        env = dict(os.environ, SMW_NO_BUILD='1')
        subprocess.run([str(TOOLS / 'run_rust.sh'), str(script), str(out)], env=env, capture_output=True)
        dump = out / 'dump.txt'
        if not dump.exists():
            return {'mode': mode, 'seed': seed, 'map': map_name, 'error': 'no dump'}
        return summarize(mode, seed, map_name, dump.read_text())


def summarize(mode, seed, map_name, text):
    gameplay = 0
    gameover_frame = None
    winner = -1
    scores = {}
    first = {}
    last = {}
    still = {}
    stuck = {}
    boxes = {}
    trapped = 0
    frame = 0
    in_play = False
    for line in text.splitlines():
        if line.startswith('F '):
            parts = line.split()
            frame = int(parts[1])
            in_play = len(parts) > 2 and parts[2] == 'gameplay'
            if in_play:
                gameplay += 1
            continue
        if not in_play:
            continue
        m = GAME.match(line)
        if m:
            if m.group(2) == '1' and gameover_frame is None:
                gameover_frame = gameplay
                winner = int(m.group(3))
            continue
        m = PLAYER.match(line)
        if m and gameover_frame is None:
            pid, ix, velx, state, score, inair = int(m.group(1)), int(m.group(2)), float(m.group(4)), int(m.group(5)), int(m.group(6)), m.group(7)
            scores[pid] = score
            first.setdefault(pid, score)
            ready_on_ground = state == 3 and inair == '0'
            if ready_on_ground and velx == 0.0 and last.get(pid) == ix:
                still[pid] = still.get(pid, 0) + 1
                if still[pid] >= STUCK_FRAMES:
                    stuck[pid] = stuck.get(pid, 0) + 1
            else:
                still[pid] = 0
            last[pid] = ix
            iy = int(m.group(3))
            if state == 3:
                key = (pid, gameplay // TRAP_WINDOW)
                x0, x1, y0, y1, n = boxes.get(key, (ix, ix, iy, iy, 0))
                boxes[key] = (min(x0, ix), max(x1, ix), min(y0, iy), max(y1, iy), n + 1)
    trapped = sum(n for x0, x1, y0, y1, n in boxes.values() if n >= TRAP_WINDOW // 2 and x1 - x0 < 60 and y1 - y0 < 40)
    return {'mode': mode, 'seed': seed, 'map': map_name, 'gameplay_frames': gameplay, 'gameover_frame': gameover_frame, 'trapped_frames': trapped,
            'stuck_until': gameover_frame or gameplay, 'winner': winner, 'scores': scores, 'score_change': sum(abs(scores[p] - first[p]) for p in scores), 'stuck_frames': stuck}


def report(results):
    by_mode = {}
    for r in results:
        by_mode.setdefault(r['mode'], []).append(r)
    rows = {}
    for mode, rs in sorted(by_mode.items()):
        ok = [r for r in rs if 'error' not in r]
        ended = [r for r in ok if r['gameover_frame'] is not None]
        wins = [0, 0, 0, 0]
        for r in ended:
            if 0 <= r['winner'] < 4:
                wins[r['winner']] += 1
        frames = sum(r['gameplay_frames'] for r in ok) or 1
        stuck = sum(sum(r['stuck_frames'].values()) for r in ok)
        trapped = sum(r.get('trapped_frames', 0) for r in ok)
        live = sum(r.get('stuck_until', r['gameplay_frames']) for r in ok) or 1
        change = sum(r['score_change'] for r in ok)
        rows[mode] = {
            'matches': len(rs),
            'errors': len(rs) - len(ok),
            'gameover_rate': round(len(ended) / len(ok), 3) if ok else 0,
            'frames_to_gameover': round(statistics.mean(r['gameover_frame'] for r in ended)) if ended else None,
            'win_spread': round(statistics.pstdev(wins), 2) if ended else None,
            'score_per_1000': round(1000 * change / frames, 2),
            'stuck_per_1000': round(1000 * stuck / (live * 4), 2),
            'trapped_per_1000': round(1000 * trapped / (live * 4), 2),
        }
    return rows


def print_table(rows, before=None):
    cols = ['matches', 'errors', 'gameover_rate', 'frames_to_gameover', 'win_spread', 'score_per_1000', 'stuck_per_1000', 'trapped_per_1000']
    print('mode'.ljust(14) + ''.join(c.rjust(20) for c in cols))
    for mode, row in rows.items():
        cells = []
        for c in cols:
            v = row[c]
            if before and mode in before and before[mode].get(c) is not None and v is not None and c not in ('matches', 'errors'):
                cells.append(f'{v} ({v - before[mode][c]:+g})'.rjust(20))
            else:
                cells.append(str(v).rjust(20))
        print(mode.ljust(14) + ''.join(cells))


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--modes', default='')
    ap.add_argument('--seeds', default='1-4')
    ap.add_argument('--maps', default=','.join(DEFAULT_MAPS))
    ap.add_argument('--frames', type=int, default=7200)
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('--json')
    ap.add_argument('--compare')
    args = ap.parse_args()

    if not os.environ.get('SMW_NO_BUILD'):
        subprocess.run(['cargo', 'build', '--release', '--quiet', '--manifest-path', str(PORT / 'Cargo.toml'), '--bin', 'smw'], check=True)
    sources = sorted((TOOLS / 'replays').glob('cpu_*.txt'))
    wanted = {m for m in args.modes.split(',') if m}
    sources = [s for s in sources if not wanted or s.stem[len('cpu_'):] in wanted]
    jobs = [(s.stem[len('cpu_'):], s, seed, m, args.frames) for s in sources for seed in parse_range(args.seeds) for m in args.maps.split(',')]
    print(f'{len(jobs)} matches, {args.jobs} at once', file=sys.stderr)
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(play, jobs))
    rows = report(results)
    before = json.loads(Path(args.compare).read_text())['summary'] if args.compare else None
    print_table(rows, before)
    if args.json:
        Path(args.json).write_text(json.dumps({'summary': rows, 'matches': results}, indent=1))


if __name__ == '__main__':
    main()
