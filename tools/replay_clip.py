#!/usr/bin/env python3
"""Cut one match out of a session recording as a standalone clip, or list the session's matches.

Usage:
  tools/replay_clip.py <recording.txt|.smwrp> --list
  tools/replay_clip.py <recording.txt|.smwrp> --match <k> [-o clip.txt|clip.smwrp]

The recording needs `#@ mark` and `#@ checkpoint` lines (docs/REPLAY.md, "Markers, checkpoints and
clips"). A clip holds the recording's header, match k's markers and checkpoint, its input lines and
`#@ segment=k`, so `smw --replay clip.txt`, run_rust.sh, web_replay.mjs and replay_video.py play only
that match, with the same frame numbers and dump as in the full recording.
"""
import argparse
import gzip
import re
import string
import sys
from pathlib import Path

FIELD = re.compile(r'(\w+)=("[^"]*"|\S*)')
NET_LINE = re.compile(r'^#@ netv? frame=(\d+) ')
UNFINISHED_TAIL_FRAMES = 188
GZIP_MAGIC = b'\x1f\x8b'


def read_recording(path):
    """A recording's text, plain or gzipped (.smwrp, .txt.gz)."""
    data = Path(path).read_bytes()
    return (gzip.decompress(data) if data[:2] == GZIP_MAGIC else data).decode()


def write_recording(path, text):
    """Writes text, gzipped when the name ends in .smwrp or .gz."""
    data = text.encode()
    Path(path).write_bytes(gzip.compress(data, mtime=0) if Path(path).suffix in ('.smwrp', '.gz') else data)


def recording_stem(path):
    """The file name without .smwrp, .txt.gz or .txt."""
    name = Path(path).name
    for ext in ('.smwrp', '.gz', '.txt'):
        name = name.removesuffix(ext)
    return name


def fields(text):
    """`key=value` pairs of a directive (values may be double-quoted), in order."""
    return {k: v[1:-1] if v.startswith('"') else v for k, v in FIELD.findall(text)}


class Match:
    def __init__(self, number, start, info):
        self.number = number
        self.start = start
        self.info = info
        self.end = None
        self.result = {}
        self.checkpoint = None
        self.marks = []
        self.reason = None

    @property
    def frames(self):
        return None if self.end is None else self.end - self.start

    def describe(self):
        i = self.info
        mode = string.capwords(i.get('mode', '?').replace('_', ' '))
        if i.get('style'):
            mode = 'Multi Star' if i['style'] == 'multi' else i['style'].capitalize()
        players = i.get('players', '?')
        seconds = '' if self.frames is None else f', {self.frames * 16 / 1000:.0f} s'
        where = f" ({i['type']})" if i.get('type') not in (None, 'single') else ''
        return f"Match {self.number} - {mode}, {i.get('map', '?')}, {players} players{seconds}{where}"


class Recording:
    def __init__(self, path):
        self.path = Path(path)
        self.lines = read_recording(self.path).splitlines()
        self.header = []
        self.events = []
        self.net = []
        self.matches = {}
        self.frames = None
        current = None
        for line in self.lines:
            if line.startswith('#@ mark ') or line.startswith('#@ checkpoint '):
                f = fields(line)
                if line.startswith('#@ checkpoint '):
                    m = self.matches.setdefault(int(f['match']), Match(int(f['match']), int(f['frame']), {}))
                    if 'unsupported' in f:
                        m.reason = f['unsupported']
                    else:
                        m.checkpoint = line
                    continue
                frame, state = int(f['frame']), f.get('state')
                if state == 'gameplay' and 'type' in f:
                    m = self.matches.setdefault(int(f['match']), Match(int(f['match']), frame, f))
                    m.start, m.info = frame, f
                    current = m
                if current is not None:
                    current.marks.append(line)
                    if state not in ('gameplay', 'scoreboard'):
                        current.end, current.result = frame, f
                        current = None
                    elif state == 'scoreboard':
                        current.result = f
            elif NET_LINE.match(line):
                self.net.append(line)
            elif line.startswith('#@ frames='):
                self.frames = int(line.split('=', 1)[1])
            elif line.startswith('#'):
                if not self.events:
                    self.header.append(line)
            elif line.strip():
                self.events.append(line)

    def event_frame(self, line):
        return int(line.split()[0])

    def end_of(self, match):
        """The frame the match left gameplay, or where the recording stops."""
        if match.end is not None:
            return match.end
        if self.frames is not None:
            return self.frames
        last = max((self.event_frame(e) for e in self.events), default=0)
        return last + UNFINISHED_TAIL_FRAMES

    def clippable(self):
        return [m for m in sorted(self.matches.values(), key=lambda m: m.number) if m.checkpoint]

    def clip(self, number):
        match = self.matches.get(number)
        if match is None or match.checkpoint is None:
            reason = f' ({match.reason} matches have no checkpoint)' if match and match.reason else ''
            raise SystemExit(f'{self.path}: no checkpoint for match {number}{reason}')
        end = self.end_of(match)
        devices = 0
        for e in self.events:
            parts = e.split()
            if parts[1] in ('jaxis', 'jbutton', 'jhat'):
                devices = max(devices, int(parts[2]) + 1)
        out = [f'# Super Mario War clip: match {number} of {self.path.name}.']
        out += [l for l in self.header if l.startswith('#@ ') and not l.startswith('#@ frames=')]
        out.append(f'#@ segment={number}')
        out += match.marks
        out.append(match.checkpoint)
        # Frame 0 hat lines only make a replay attach the recording's joysticks; frame 0 is never played.
        out += [f'0 jhat {d} 0 0' for d in range(devices)]
        out += [e for e in self.events if match.start < self.event_frame(e) < end]
        out += [n for n in self.net if match.start < int(NET_LINE.match(n)[1]) < end]
        out.append(f'#@ frames={end}')
        return '\n'.join(out) + '\n'


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('recording')
    ap.add_argument('--list', action='store_true', help="print the session's matches")
    ap.add_argument('--match', type=int, help='the match to cut (1, 2, ...)')
    ap.add_argument('-o', '--output', help='clip file, gzipped if it ends in .smwrp (default: <recording>_match<k>.txt)')
    args = ap.parse_args()
    rec = Recording(args.recording)
    if args.list or args.match is None:
        if not rec.matches:
            print(f'{args.recording}: no match markers (recorded before markers existed?)')
        for m in sorted(rec.matches.values(), key=lambda m: m.number):
            if m.checkpoint:
                print(f'{m.describe()}  [frames {m.start}-{rec.end_of(m)}]')
            else:
                print(f'Match {m.number} - no clip ({m.reason or "no checkpoint"})')
        return 0
    out = Path(args.output or rec.path.with_name(f'{recording_stem(rec.path)}_match{args.match}.txt'))
    write_recording(out, rec.clip(args.match))
    print(out)
    return 0


if __name__ == '__main__':
    sys.exit(main())
