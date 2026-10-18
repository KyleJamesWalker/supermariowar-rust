#!/usr/bin/env python3
"""Replays each script with SMW_CHECK_DISPLAY_LIST=1 and fails if any gameplay frame does not re-render exactly from
its display list (common/gfx/display_list.rs), which would mean a screen write the interpolated frames miss.

Usage: display_list_check.py [--jobs N] <replay.txt>...   SMW_BIN and SMW_DATA_DIR as for run_rust.sh.
"""
import argparse
import concurrent.futures
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent


def check(replay):
    out = tempfile.mkdtemp(prefix='smw-dlcheck-')
    try:
        env = dict(os.environ, SMW_NO_BUILD='1', SMW_CHECK_DISPLAY_LIST='1')
        run = subprocess.run([str(TOOLS / 'run_rust.sh'), replay, out], env=env, capture_output=True, text=True)
        log = Path(out, 'stdout.log').read_text(errors='replace') if Path(out, 'stdout.log').exists() else ''
        misses = [line for line in log.splitlines() if line.startswith('[display list]')]
        return replay, run.returncode, misses
    finally:
        shutil.rmtree(out, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('replays', nargs='+')
    args = ap.parse_args()
    failed = 0
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        for replay, code, misses in pool.map(check, args.replays):
            name = Path(replay).stem
            if code != 0:
                print(f'{name}: exit {code}')
                failed += 1
            elif misses:
                print(f'{name}: {len(misses)} frames differ, first: {misses[0]}')
                failed += 1
            else:
                print(f'{name}: exact')
    print(f'{len(args.replays) - failed}/{len(args.replays)} replays exact')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
