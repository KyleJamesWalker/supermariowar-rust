#!/usr/bin/env python3
"""Check that every match of a recording replays exactly from its checkpoint.

Usage: tools/segment_check.py [--web] [--jobs N] [--keep DIR] <recording-or-replay.txt|.smwrp> ...

For each file it runs the whole replay headless (tools/run_rust.sh) and, if the file has no
checkpoints yet (a hand-written replay or an older recording), records them on the way with
SMW_RECORD_TO. Then, for each match with a checkpoint, it compares the full replay's dump blocks
from the match's first frame up to the frame it left gameplay with

  segment  SMW_SEGMENT=k on the recording, and
  clip     the clip tools/replay_clip.py cuts, replayed on its own,
  web      (--web) that clip in the web build (tools/web_replay.mjs, dist/web).

All must be byte-identical and cover exactly those frames. Exit status 0 when all are, 1 otherwise.
"""
import argparse
import concurrent.futures
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from replay_clip import Recording, read_recording, recording_stem  # noqa: E402

TOOLS = Path(__file__).resolve().parent
REPO = TOOLS.parent


def blocks(dump):
    """Frame number -> that frame's dump block."""
    out, cur = {}, None
    with open(dump) as f:
        for line in f:
            if line.startswith('F '):
                cur = int(line.split()[1])
                out[cur] = []
            out[cur].append(line)
    return {k: ''.join(v) for k, v in out.items()}


def run(cmd, env=None, out_dir=None):
    e = dict(os.environ, SMW_NO_BUILD='1', **(env or {}))
    r = subprocess.run(cmd, env=e, capture_output=True, text=True)
    if r.returncode != 0:
        log = Path(out_dir, 'stdout.log').read_text()[-600:] if out_dir and Path(out_dir, 'stdout.log').exists() else ''
        raise RuntimeError(f"{' '.join(map(str, cmd))} exited {r.returncode}: {r.stderr.strip()[-400:]} {log}")
    return Path(out_dir) / 'dump.txt'


def compare(full, start, end, dump):
    """None when the dump is exactly the full replay's frames start..end-1, else where it differs."""
    seg = blocks(dump)
    expected = list(range(start, end))
    if sorted(seg) != expected:
        have = sorted(seg)
        return f'frames {have[0] if have else "-"}..{have[-1] if have else "-"}, expected {start}..{end - 1}'
    for f in expected:
        if seg[f] != full.get(f):
            a, b = (full.get(f) or '').splitlines(), seg[f].splitlines()
            line = next((i for i in range(max(len(a), len(b))) if (a[i:i + 1] or [''])[0] != (b[i:i + 1] or [''])[0]), 0)
            return f'frame {f}: full "{(a[line:line + 1] or [""])[0]}" vs "{(b[line:line + 1] or [""])[0]}"'
    return None


def check(path, work, jobs, web):
    name = recording_stem(path)
    src = read_recording(path)
    rec_path = work / f'{name}.rec.txt'
    env = {}
    if '#@ checkpoint ' in src:
        rec_path.write_text(src)
    else:
        env['SMW_RECORD_TO'] = str(rec_path)
    probe = Recording(path)
    if probe.frames is None:
        last = max((probe.event_frame(e) for e in probe.events), default=0)
        env['SMW_FRAMES'] = str(last + 188)
    full_dump = run([TOOLS / 'run_rust.sh', path, work / f'{name}.full'], env, work / f'{name}.full')
    full = blocks(full_dump)
    rec = Recording(rec_path)
    rows = []
    for m in sorted(rec.matches.values(), key=lambda m: m.number):
        if not m.checkpoint:
            rows.append((name, m.number, f'no checkpoint ({m.reason})', '-', 'excluded', 'excluded', 'excluded' if web else ''))
            continue
        end = rec.end_of(m)
        clip = work / f'{name}_match{m.number}.txt'
        clip.write_text(rec.clip(m.number))
        rows.append([name, m.number, m.describe(), f'{m.start}-{end - 1}', (m, end, clip)])

    def one(row):
        m, end, clip = row[4]
        results = []
        tasks = [('segment', [TOOLS / 'run_rust.sh', rec_path, work / f'{name}.seg{m.number}'], {'SMW_SEGMENT': str(m.number)}, work / f'{name}.seg{m.number}'),
                 ('clip', [TOOLS / 'run_rust.sh', clip, work / f'{name}.clip{m.number}'], {}, work / f'{name}.clip{m.number}')]
        if web:
            tasks.append(('web', ['node', TOOLS / 'web_replay.mjs', clip, work / f'{name}.web{m.number}'], {}, work / f'{name}.web{m.number}'))
        for label, cmd, e, out in tasks:
            try:
                diff = compare(full, m.start, end, run(cmd, e, out))
            except RuntimeError as err:
                diff = str(err)
            results.append('MATCH' if diff is None else f'DIFF {diff[:160]}')
        return row[:4] + results

    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        done = list(pool.map(lambda r: one(r) if isinstance(r, list) else list(r), rows))
    return done


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('files', nargs='+')
    ap.add_argument('--web', action='store_true', help='also replay each clip in the web build')
    ap.add_argument('--jobs', type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument('--keep', help='keep recordings, clips and dumps in this directory')
    args = ap.parse_args()

    build = subprocess.run(['cargo', 'build', '--release', '--quiet', '--manifest-path', REPO / 'Cargo.toml'], capture_output=True, text=True)
    if build.returncode != 0:
        sys.exit(build.stderr)
    work = Path(args.keep) if args.keep else Path(tempfile.mkdtemp(prefix='smw-segment-check-'))
    work.mkdir(parents=True, exist_ok=True)
    try:
        rows = []
        for f in args.files:
            if Path(f).is_dir():
                print(f'{f}: a directory, skipped', file=sys.stderr)
                continue
            try:
                rows += check(f, work, args.jobs, args.web)
            except RuntimeError as err:
                rows.append((recording_stem(f), '-', str(err), '-', 'ERROR', 'ERROR') + (('ERROR',) if args.web else ()))
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)

    head = ['recording', 'match', 'content', 'frames', 'segment', 'clip'] + (['web'] if args.web else [])
    print('| ' + ' | '.join(head) + ' |')
    print('|' + '---|' * len(head))
    for r in rows:
        print('| ' + ' | '.join(str(c) for c in r) + ' |')
    bad = [r for r in rows if any(str(c).startswith(('DIFF', 'ERROR')) for c in r[4:])]
    checked = [r for r in rows if r[4] != 'excluded']
    print(f'\n{len(checked) - len(bad)}/{len(checked)} matches identical' + (f', {len(rows) - len(checked)} excluded' if len(rows) > len(checked) else ''))
    return 1 if bad or not checked else 0


if __name__ == '__main__':
    sys.exit(main())
