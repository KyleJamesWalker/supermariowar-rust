#!/usr/bin/env python3
"""Generate the coverage replays in tools/replays/ (see REPLAY.md).

  gen_replays.py [name ...]     (default: every replay defined here)

Each replay drives the real menus from boot. Frame numbers were checked
against the C++ reference dumps (M lines), so edit with care and regenerate
the golden output with make_golden.sh afterwards.
"""

import random
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent / "replays"

GAME_MODES = [
    "classic", "frag", "timelimit", "jail", "coins", "stomp", "eggs", "ctf", "chicken", "tag", "star",
    "domination", "kingofthehill", "race", "owned", "frenzy", "survival", "greed", "health", "collection",
    "chase", "shyguytag",
]


class Script:
    def __init__(self):
        self.events = []
        self.f = 10

    def tap(self, key, gap=14, hold=2):
        self.events.append((self.f, "down", key))
        self.events.append((self.f + hold, "up", key))
        self.f += gap

    def wait(self, n):
        self.f += n

    def hold(self, f, key, n):
        self.events.append((f, "down", key))
        self.events.append((f + n, "up", key))

    def write(self, name, header):
        ev = sorted(self.events, key=lambda e: (e[0], 0 if e[1] == "up" else 1))
        with open(OUT / f"{name}.txt", "w") as fh:
            fh.write(header)
            for f, d, k in ev:
                fh.write(f"{f} {d} {k}\n")


def boot_to_main(s):
    s.tap("Return", gap=20)  # splash -> main menu


def players_all_cpu(s):
    # Main menu Players field: P1/P2 human -> CPU (Down), P3/P4 none -> CPU (Up).
    s.tap("Down")
    s.tap("Return")
    s.tap("Down")
    s.tap("Right")
    s.tap("Down")
    s.tap("Right")
    s.tap("Up")
    s.tap("Right")
    s.tap("Up")
    s.tap("Return")
    s.tap("Up")  # back to Start


def match_type(s, rights, sub_rights=0):
    """In Match Selection (focus Start): pick the match type and its sub-field value, then refocus Start."""
    s.tap("Up")       # Start -> Match field (hidden fields are skipped)
    s.tap("Return")
    for _ in range(rights):
        s.tap("Right", gap=8)
    s.tap("Return", gap=20)
    if rights:
        s.tap("Down")  # -> the visible sub-field
        if sub_rights:
            s.tap("Return")
            for _ in range(sub_rights):
                s.tap("Right", gap=8)
            s.tap("Return")
    s.tap("Down")      # -> Start


def probe(name, rights, sub_rights, returns, gap=40, frames=2500, mapname="0smw"):
    s = Script()
    boot_to_main(s)
    players_all_cpu(s)
    s.tap("Return", gap=30)   # Start -> Match Selection
    match_type(s, rights, sub_rights)
    for _ in range(returns):
        s.tap("Return", gap=gap)
    s.write(name, f"# probe\n#@ seed=1\n#@ map={mapname}\n#@ frames={frames}\n")


def game_settings_mode(s, mode_index):
    """In Game Settings (focus Start): select the game mode, then return focus to Start."""
    if mode_index:
        s.tap("Down")
        s.tap("Return")
        for _ in range(mode_index):
            s.tap("Right", gap=8)
        s.tap("Return")
        s.tap("Up")


def cpu_mode(name, idx, mapname, seed, frames=3600):
    s = Script()
    boot_to_main(s)
    players_all_cpu(s)
    s.tap("Return", gap=20)   # Start -> Match Selection
    s.tap("Return", gap=30)   # Start -> Team Select
    s.tap("Return", gap=30)   # confirm teams -> Game Settings
    s.wait(10)
    game_settings_mode(s, idx)
    s.tap("Return", gap=40)   # Start
    shots = [s.f + 40, s.f + 1500, frames - 1]
    s.write(name, f"# 4 CPU players, mode {GAME_MODES[idx]} (index {idx}) on '{mapname}'.\n"
                  f"#@ seed={seed}\n#@ map={mapname}\n#@ frames={frames}\n#@ shots={','.join(map(str, shots))}\n")


def fuzz(name, seed, mapname, frames=5000):
    s = Script()
    boot_to_main(s)
    s.tap("Return", gap=20)
    s.tap("Return", gap=20)
    s.tap("E", gap=30)
    s.tap("Return", gap=30)
    s.tap("Return", gap=30)
    s.tap("Return", gap=40)
    rng = random.Random(seed)
    keys = [["Left", "Right", "Up", "Down", "Right Ctrl", "Right Shift"], ["A", "D", "W", "S", "E", "Q"]]
    for player_keys in keys:
        busy = {k: 0 for k in player_keys}
        f = s.f
        while f < frames - 10:
            k = rng.choice(player_keys)
            if busy[k] <= f:
                n = rng.choice([1, 2, 3, 5, 8, 15, 30, 60])
                s.hold(f, k, n)
                busy[k] = f + n + 1
            f += rng.choice([1, 2, 3, 4, 6, 10])
    s.write(name, f"# Fuzzed input for 2 human players (python random seed {seed}) on '{mapname}'.\n"
                  f"#@ seed={seed}\n#@ map={mapname}\n#@ frames={frames}\n#@ shots={frames // 2},{frames - 1}\n")


REPLAYS = {}

MAPS = ["0smw", "Block Piles", "2skyfight", "3blockforts", "4highabove", "Cloud Props", "Death From Above",
        "Bouncy Spikes", "Manic Mountain", "Hanging Cactus Gardens", "Wacky Woods", "5fall"]

for i, mode in enumerate(GAME_MODES):
    REPLAYS[f"cpu_{mode}"] = (lambda i=i, mode=mode: cpu_mode(f"cpu_{mode}", i, MAPS[i % len(MAPS)], 100 + i))

def flow(name, desc, rights, sub_rights, returns, seed, frames, mapname="0smw", gap=40):
    s = Script()
    boot_to_main(s)
    players_all_cpu(s)
    s.tap("Return", gap=30)   # Start -> Match Selection
    match_type(s, rights, sub_rights)
    for _ in range(returns):
        s.tap("Return", gap=gap)
    shots = [s.f + 60, frames // 2, frames - 1]
    s.write(name, f"# 4 CPU players: {desc}.\n#@ seed={seed}\n#@ map={mapname}\n#@ frames={frames}\n#@ shots={','.join(map(str, shots))}\n")


REPLAYS["flow_tournament"] = lambda: flow("flow_tournament", "Tournament (2 wins) through the menus", 1, 0, 3, 201, 6000, "2skyfight")
REPLAYS["flow_tour"] = lambda: flow("flow_tour", "first Tour, first stop", 2, 0, 3, 202, 5000)
REPLAYS["flow_world"] = lambda: flow("flow_world", "first World, CPU-driven first stage", 3, 0, 3, 203, 6000)
REPLAYS["mini_pipe"] = lambda: flow("mini_pipe", "Pipe Coin minigame", 4, 0, 2, 204, 3000)
REPLAYS["mini_hammerboss"] = lambda: flow("mini_hammerboss", "Hammer Boss minigame", 4, 1, 2, 205, 3000)
REPLAYS["mini_boxes"] = lambda: flow("mini_boxes", "Boxes minigame", 4, 4, 2, 206, 3000)

REPLAYS["fuzz_a"] = lambda: fuzz("fuzz_a", 7001, "0smw")
REPLAYS["fuzz_b"] = lambda: fuzz("fuzz_b", 7002, "Block Piles")
REPLAYS["fuzz_c"] = lambda: fuzz("fuzz_c", 7003, "Manic Mountain")


def main():
    names = sys.argv[1:] or list(REPLAYS)
    for n in names:
        REPLAYS[n]()
        print(n)


if __name__ == "__main__":
    main()
