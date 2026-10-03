#!/usr/bin/env python3
"""Derive non-default persistent files from game-written defaults (see persist_interop.sh).

Usage: persist_fuzz.py <seed> <defaults_dir> <out_dir>
defaults_dir holds options.bin, controls.sdl2.bin, mapsummary.txt and filters/*.txt as the game wrote them;
out_dir gets fuzzed copies plus servers.toml. Values stay in the ranges the menus can produce.
"""
import os
import random
import struct
import sys

U8_FIELDS = [  # (name, values) in options.bin order
    ('spawnstyle', range(3)), ('awardstyle', range(6)), ('teamcollision', range(3)), ('screencrunch', (0, 1)),
    ('toplayer', (0, 1)), ('scoreboardstyle', range(3)), ('teamcolors', (0, 1)), ('sound', (0, 1)),
    ('music', (0, 1)), ('musicvolume', range(0, 129, 16)), ('soundvolume', range(0, 129, 16)),
    ('respawn', range(0, 11)), ('outofboundstime', range(0, 11)), ('cpudifficulty', range(5)),
    ('framelimiter', range(0, 10)), ('bonuswheel', range(3)), ('keeppowerup', (0, 1)),
    ('showwinningcrown', (0, 1)), ('playnextmusic', (0, 1)), ('pointspeed', range(1, 41)), ('swapstyle', range(3)),
    ('overridepowerupsettings', range(3)), ('secretsenabled', (0, 1)), ('startgamecountdown', (0, 1)),
    ('deadteamnotice', (0, 1)), ('tournamentcontrolstyle', range(8)), ('startmodedisplay', (0, 1)),
]
I16_FIELDS = [
    ('shieldtime', range(0, 121, 10)), ('shieldstyle', range(4)), ('itemrespawntime', range(0, 1801, 60)),
    ('hiddenblockrespawn', range(0, 1801, 60)), ('fireballttl', range(30, 301, 30)), ('fireballlimit', range(0, 31)),
    ('hammerdelay', range(0, 31)), ('hammerttl', range(30, 301, 30)), ('hammerpower', (0, 1)),
    ('hammerlimit', range(0, 31)), ('boomerangstyle', range(4)), ('boomeranglife', range(30, 301, 30)),
    ('boomeranglimit', range(0, 31)), ('featherjumps', range(1, 6)), ('featherlimit', range(0, 31)),
    ('leaflimit', range(0, 31)), ('pwingslimit', range(0, 31)), ('tanookilimit', range(0, 31)),
    ('bombslimit', range(0, 31)), ('wandfreezetime', range(30, 361, 30)), ('wandlimit', range(0, 31)),
    ('shellttl', range(0, 1201, 60)), ('blueblockttl', range(0, 1201, 60)), ('redblockttl', range(0, 1201, 60)),
    ('grayblockttl', range(0, 1201, 60)), ('storedpowerupdelay', range(1, 9)), ('warplockstyle', range(5)),
    ('warplocktime', range(0, 601, 60)), ('suicidetime', range(0, 1201, 60)),
]
NUM_POWERUP_PRESETS, NUM_POWERUPS = 17, 26
GMS_SIZE = 194
CONTROL_SIZE = 68


def fuzz_options(rng, data):
    data = bytearray(data)
    off = 16
    for _, values in U8_FIELDS:
        data[off] = rng.choice(list(values))
        off += 1
    for _, values in I16_FIELDS:
        struct.pack_into('<h', data, off, rng.choice(list(values)))
        off += 2
    struct.pack_into('<h', data, off, rng.randrange(NUM_POWERUP_PRESETS))  # poweruppreset
    off += 2
    for _ in range(NUM_POWERUP_PRESETS * NUM_POWERUPS):
        struct.pack_into('<h', data, off, rng.randrange(11))
        off += 2
    off += 1  # fullscreen stays off under the dummy video driver
    goals = (len(data) - off - GMS_SIZE - 8 - 4 - 8 - 7) // 2
    off += 2 * goals + GMS_SIZE  # goals and mode settings must match menu select-field keys
    for p in range(4):
        struct.pack_into('<h', data, off + 2 * p, rng.randrange(4))  # skinids
    off += 8
    for p in range(4):
        data[off + p] = rng.randrange(2)  # randomskin
    off += 4
    off += 8  # playercontrol drives the team select menu the replay walks through
    for i in range(7):
        data[off + i] = rng.randrange(2)  # announcer, music, world music, sound, menu/world/game gfx packs
    return bytes(data)


def fuzz_controls(rng, data):
    data = bytearray(data)
    keys = [ord(c) for c in 'bcfghijklmnoprtuvxyz0123456789']
    # Player 1's keyboard keys drive the replay, so only players 2-4 are remapped.
    for player in range(1, 4):
        base = player * 2 * CONTROL_SIZE
        for k in range(8):
            struct.pack_into('<i', data, base + 4 + 4 * k, rng.choice(keys))
    for player in range(4):
        struct.pack_into('<h', data, 8 * CONTROL_SIZE + 2 * player, rng.choice((0, 0, 1, 5)))
    return bytes(data)


def fuzz_summary(rng, text):
    out = []
    for line in text.splitlines():
        if rng.random() < 0.1:
            continue
        name, *bits = line.split(',')
        out.append(','.join([name] + [str(int(b) ^ (rng.random() < 0.2)) for b in bits]))
    return '\n'.join(out) + '\n'


def fuzz_filter(rng, maps):
    picked = sorted(rng.sample(maps, rng.randrange(0, 30))) + ['No such map']
    return f'#Version\n2.0.0.1\n\n#Icon\n{rng.randrange(120)}\n\n#Maps\n' + ''.join(m + '\n' for m in picked)


def toml_string(s):
    out = ''
    for c in s:
        if c in '"\\':
            out += '\\' + c
        elif c == '\t':
            out += '\\t'
        elif ord(c) < 0x20 or ord(c) == 0x7F:
            out += '\\u%04X' % ord(c)
        else:
            out += c
    return '"' + out + '"'


def servers_toml(rng):
    """A servers.toml with escapes, UTF-8, invalid entries and arrays on both sides of toml11's inline limit."""
    alphabet = 'abcdefghijklmnopqrstuvwxyz' + 'AZ09 _-"\\\t' + '\u00e9\u00df'
    name = ''.join(rng.choice(alphabet) for _ in range(rng.randrange(2, 14)))
    hosts = ['127.0.0.1', 'smw.example.org', 'short', '192.168.0.10', 'localhost', 'a-much-longer-server-name.example.com',
             'game "quoted" host', 'tab\there.example', '\u00fcml\u00e4ut.example']
    rng.shuffle(hosts)
    entries = [toml_string(h) for h in hosts[:rng.randrange(0, len(hosts) + 1)]]
    if entries and rng.random() < 0.3:
        entries.insert(rng.randrange(len(entries)), str(rng.randrange(100000000, 999999999)))
    lines = []
    if rng.random() < 0.9:
        lines.append('player_name = ' + (toml_string(name) if rng.random() < 0.9 else str(rng.randrange(1000))))
    if rng.random() < 0.9:
        lines.append('servers = [' + ', '.join(entries) + ']' if rng.random() < 0.9 else 'servers = "not an array"')
    rng.shuffle(lines)
    return '\n'.join(lines) + '\n'


def main():
    seed, src, dst = int(sys.argv[1]), sys.argv[2], sys.argv[3]
    rng = random.Random(seed)
    with open(f'{src}/options.bin', 'rb') as f:
        options = fuzz_options(rng, f.read())
    with open(f'{src}/controls.sdl2.bin', 'rb') as f:
        controls = fuzz_controls(rng, f.read())
    with open(f'{src}/mapsummary.txt') as f:
        summary = fuzz_summary(rng, f.read())
    with open(f'{dst}/options.bin', 'wb') as f:
        f.write(options)
    with open(f'{dst}/controls.sdl2.bin', 'wb') as f:
        f.write(controls)
    with open(f'{dst}/mapsummary.txt', 'w') as f:
        f.write(summary)
    with open(f'{dst}/servers.toml', 'w') as f:
        f.write(servers_toml(rng))
    maps = [line.split(',')[0] for line in summary.splitlines()]
    os.makedirs(f'{dst}/filters', exist_ok=True)
    for name in sorted(os.listdir(f'{src}/filters')):
        with open(f'{dst}/filters/{name}', 'w') as f:
            f.write(fuzz_filter(rng, maps))


if __name__ == '__main__':
    main()
