#!/usr/bin/env python3
"""Generate the coverage replays in tools/replays/ (see docs/REPLAY.md).

  gen_replays.py [name ...]     (default: every replay defined here)
  gen_replays.py sweep          (tools/replays_sweep/: one game on every map)
  gen_replays.py segments       (tools/segment_replays/: multi-match sessions for segment_check.py)

Each replay drives the real menus from boot. Frame numbers were checked
against the C++ reference dumps (M lines), so edit with care and regenerate
the golden output with make_golden.sh afterwards.
"""

import random
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent / "replays"
SWEEP_OUT = Path(__file__).resolve().parent / "replays_sweep"
SEGMENT_OUT = Path(__file__).resolve().parent / "segment_replays"
DATA = Path(__file__).resolve().parents[2] / "data"

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

    def write(self, name, header, out=None):
        ev = sorted(self.events, key=lambda e: (e[0], 0 if e[1] == "up" else 1))
        with open((out or OUT) / f"{name}.txt", "w") as fh:
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

def flow(name, desc, rights, sub_rights, returns, seed, frames, mapname="0smw", gap=40, options=None):
    s = Script()
    boot_to_main(s)
    players_all_cpu(s)
    s.tap("Return", gap=30)   # Start -> Match Selection
    match_type(s, rights, sub_rights)
    for _ in range(returns):
        s.tap("Return", gap=gap)
    shots = [s.f + 60, frames // 2, frames - 1]
    opts = f"#@ options={options}\n" if options else ""
    s.write(name, f"# 4 CPU players: {desc}.\n#@ seed={seed}\n#@ map={mapname}\n#@ frames={frames}\n#@ shots={','.join(map(str, shots))}\n{opts}")


REPLAYS["flow_tournament"] = lambda: flow("flow_tournament", "Tournament (2 wins) through the menus", 1, 0, 3, 201, 6000, "2skyfight")
REPLAYS["flow_tour"] = lambda: flow("flow_tour", "first Tour, first stop", 2, 0, 3, 202, 5000)
REPLAYS["flow_world"] = lambda: flow("flow_world", "first World, CPU-driven first stage", 3, 0, 3, 203, 6000)
REPLAYS["cov_tour_options"] = lambda: flow(
    "cov_tour_options", "Mario tour first stop (Frenzy, no settings on the line) with non-default Frenzy, CTF and Survival options from options.bin",
    2, 6, 3, 207, 3000, options="fixtures/options_modes.bin")
REPLAYS["mini_pipe"] = lambda: flow("mini_pipe", "Pipe Coin minigame", 4, 0, 2, 204, 3000)
REPLAYS["mini_hammerboss"] = lambda: flow("mini_hammerboss", "Hammer Boss minigame", 4, 1, 2, 205, 3000)
REPLAYS["mini_boxes"] = lambda: flow("mini_boxes", "Boxes minigame", 4, 4, 2, 206, 3000)

def edit_options(s, edits):
    """From the main menu (focus Start): open Options, then for each (submenu, fields) open that submenu
    and edit its fields top to bottom, then back out to Start. A field entry is an int (modify, Right n
    times, accept), "t" (auto-advance field: one Return), 0 (skip), or a list of raw keys."""
    s.tap("Down")
    s.tap("Down")
    s.tap("Down")            # Start -> Players -> Multiplayer -> Options
    s.tap("Return", gap=20)
    cur = 0
    for submenu, fields in edits:
        key = "Down" if submenu > cur else "Up"
        for _ in range(abs(submenu - cur)):
            s.tap(key)
        cur = submenu
        s.tap("Return", gap=20)
        for f in fields:
            if isinstance(f, list):
                for k in f:
                    s.tap(k)
                continue
            if f == "t":
                s.tap("Return")
            elif f:
                s.tap("Return")
                for _ in range(f):
                    s.tap("Right", gap=8)
                s.tap("Return")
            s.tap("Down")
        s.tap("Escape", gap=20)  # submenu -> Options, focus stays on its button
    s.tap("Escape", gap=20)      # Options -> main menu (focus Options)
    s.tap("Up")
    s.tap("Up")
    s.tap("Up")                  # back to Start


def set_players(s, deltas, order=(0, 1, 2, 3)):
    """Players field: per slot, a list of 'Up'/'Down' presses (Down: none->human->CPU->none).
    Slots are visited in `order`; the game refuses to drop below two players, so add players first."""
    s.tap("Down")
    s.tap("Return")
    cur = 0
    for slot in order:
        for _ in range((slot - cur) % 4):
            s.tap("Right")
        cur = slot
        for k in deltas[slot]:
            s.tap(k)
    s.tap("Return")
    s.tap("Up")


def options_game(name, desc, seed, mapname, frames, edits, players, mode=0, team_keys=("Return",), order=(0, 1, 2, 3)):
    s = Script()
    boot_to_main(s)
    if edits:
        edit_options(s, edits)
    set_players(s, players, order)
    s.tap("Return", gap=30)   # Start -> Match Selection
    s.tap("Return", gap=30)   # Start -> Team Select
    for key in team_keys:
        s.tap(key, gap=30)
    s.tap("Return", gap=30)   # -> Game Settings
    s.wait(10)
    game_settings_mode(s, mode)
    s.tap("Return", gap=40)   # Start
    shots = [s.f + 60, frames - 1]
    s.write(name, f"# {desc}\n#@ seed={seed}\n#@ map={mapname}\n#@ frames={frames}\n#@ shots={','.join(map(str, shots))}\n")


CPU4 = [["Down"], ["Down"], ["Up"], ["Up"]]

REPLAYS["opt_gameplay"] = lambda: options_game(
    "opt_gameplay", "Gameplay options changed (respawn, shields, bounds, suicide, warp lock, CPU difficulty, point speed, secrets), 4 CPUs, Classic.",
    301, "Manic Mountain", 3600, [(0, [3, 1, 2, 1, 2, 1, 1, 2, 1, 1])], CPU4, team_keys=())
REPLAYS["opt_items"] = lambda: options_game(
    "opt_items", "Item odds preset, item settings, projectile options and limits changed, 4 CPUs, Frenzy.",
    302, "Block Piles", 3600,
    [(2, [["Down", "Return", "Right", "Right", "Right", "Return"]]), (3, [2, 1, 1, 1, 1, "t"]),
     (4, [1, 1, 1, 2, 1, 1, 1, 1, "t", 1, 1, 1]), (5, [2, 1, 1, 1, 1, 1, 1, 1, 1])], CPU4, mode=15, team_keys=())
REPLAYS["opt_eyecandy"] = lambda: options_game(
    "opt_eyecandy", "Eye candy, team and top-layer options changed, 4 CPUs, Star mode.",
    303, "Cloud Props", 3600, [(7, [1, 2, 1, "t", "t", "t", "t", "t"]), (1, [1, "t", 1]), (6, ["t"])], CPU4, mode=10, team_keys=())
REPLAYS["players_h_n_c_c"] = lambda: options_game(
    "players_h_n_c_c", "Players: human, none, CPU, CPU; Classic; player 1 idle.",
    304, "2skyfight", 3000, [], [[], ["Up"], ["Up"], ["Up"]], order=(2, 3, 1))
REPLAYS["players_c_h_c_n"] = lambda: options_game(
    "players_c_h_c_n", "Players: CPU, human, CPU, none; Classic; player 2 idle.",
    305, "3blockforts", 3000, [], [["Down"], [], ["Up"], []], team_keys=("E", "Return"))
REPLAYS["teams_2v2"] = lambda: options_game(
    "teams_2v2", "Team play: players 3 and 4 as CPUs; player 2 moves onto player 1's team in Team Select; Classic.",
    306, "4highabove", 3000, [], [[], [], ["Up"], ["Up"]], team_keys=("A", "E", "Return"))

REPLAYS["fuzz_a"] = lambda: fuzz("fuzz_a", 7001, "0smw")
REPLAYS["fuzz_b"] = lambda: fuzz("fuzz_b", 7002, "Block Piles")
REPLAYS["fuzz_c"] = lambda: fuzz("fuzz_c", 7003, "Manic Mountain")


SESSION_FRAMES = 30000


def keep_going(s, frames):
    """Tap Return every 150 frames: it ends each game's scoreboard and confirms the menus between games."""
    for f in range(s.f + 200, frames - 10, 150):
        s.events += [(f, "down", "Return"), (f + 2, "up", "Return")]


def session_flow(name, desc, rights, sub_rights, returns, seed, mapname="0smw", options=None):
    s = Script()
    boot_to_main(s)
    players_all_cpu(s)
    s.tap("Return", gap=30)   # Start -> Match Selection
    match_type(s, rights, sub_rights)
    for _ in range(returns):
        s.tap("Return", gap=40)
    keep_going(s, SESSION_FRAMES)
    opts = f"#@ options={options}\n" if options else ""
    s.write(name, f"# 4 CPU players: {desc}, played on by tapping Return.\n#@ seed={seed}\n#@ map={mapname}\n"
                  f"#@ frames={SESSION_FRAMES}\n{opts}", SEGMENT_OUT)


def play(s, rng, n, players):
    """Random holds of each human player's game keys for n frames."""
    end = s.f + n
    for keys in players:
        busy = {k: 0 for k in keys}
        f = s.f
        while f < end - 10:
            k = rng.choice(keys)
            if busy[k] <= f:
                d = min(rng.choice([1, 2, 3, 5, 8, 15, 30, 60]), end - 5 - f)
                if d > 0:
                    s.hold(f, k, d)
                    busy[k] = f + d + 1
            f += rng.choice([1, 2, 3, 4, 6, 10])
    s.f = end


def exit_game(s):
    s.tap("Escape", gap=20)   # exit dialog
    s.tap("Left", gap=10)     # Yes
    s.tap("Return", gap=60)


def session_mixed():
    """Two human players: Classic, Star and Frenzy games, each left through the exit dialog; player 1
    holds Right across the first game's start."""
    p1 = ["Left", "Right", "Up", "Down", "Right Ctrl", "Right Shift"]
    p2 = ["A", "D", "W", "S", "E", "Q"]
    s = Script()
    rng = random.Random(7)
    boot_to_main(s)
    s.tap("Return", gap=20)   # Start -> Match Selection
    s.tap("Return", gap=30)   # -> Team Select
    s.tap("E", gap=30)        # player 2 ready
    s.tap("Return", gap=30)   # player 1 ready
    s.tap("Return", gap=30)   # -> Game Settings
    s.tap("Return", gap=10)   # Start
    s.hold(s.f, "Right", 120)
    s.wait(40)
    play(s, rng, 700, [p1, p2])
    exit_game(s)
    for mode in (10, 5):      # Star, then Frenzy
        s.wait(30)
        game_settings_mode(s, mode)
        s.tap("Return", gap=40)
        play(s, rng, 900, [p1, p2])
        exit_game(s)
    s.write("mixed_humans", f"# Two human players: Classic, Star and Frenzy, each left through the exit dialog.\n"
                            f"#@ seed=11\n#@ frames={s.f + 100}\n", SEGMENT_OUT)


SEGMENTS = {
    "tournament_long": lambda: session_flow("tournament_long", "Tournament", 1, 0, 3, 201, "2skyfight"),
    "tour_long": lambda: session_flow("tour_long", "first Tour", 2, 0, 3, 202),
    "tour_options_long": lambda: session_flow("tour_options_long", "Mario tour with options.bin mode settings", 2, 6, 3, 207,
                                              options="../replays/fixtures/options_modes.bin"),
    "world_long": lambda: session_flow("world_long", "first World", 3, 0, 3, 203),
    "mini_pipe_long": lambda: session_flow("mini_pipe_long", "Pipe Coin minigame", 4, 0, 2, 204),
    "mini_hammerboss_long": lambda: session_flow("mini_hammerboss_long", "Hammer Boss minigame", 4, 1, 2, 205),
    "mini_boxes_long": lambda: session_flow("mini_boxes_long", "Boxes minigame", 4, 4, 2, 206),
    "mixed_humans": session_mixed,
}


CPP_CRASH_FRAME = {"Tanuki_Moby Dick.map": 812}


def sweep():
    """4 CPU Classic games on every map in data/maps, about 900 gameplay frames each."""
    import re
    SWEEP_OUT.mkdir(exist_ok=True)
    for old in SWEEP_OUT.glob("*.txt"):
        old.unlink()
    maps = sorted(p.name for p in (DATA / "maps").glob("*.map"))
    for i, mapfile in enumerate(maps):
        s = Script()
        boot_to_main(s)
        players_all_cpu(s)
        s.tap("Return", gap=20)   # Start -> Match Selection
        s.tap("Return", gap=30)   # Start -> Team Select
        s.tap("Return", gap=30)   # confirm teams -> Game Settings
        s.wait(10)
        s.tap("Return", gap=40)   # Start
        frames = s.f + 900
        # The C++ reference segfaults here (CO_Spring::update -> collision_detection_map) at frame 813.
        if mapfile in CPP_CRASH_FRAME:
            frames = CPP_CRASH_FRAME[mapfile]
        name = re.sub(r"[^a-z0-9]+", "_", mapfile[:-4].lower()).strip("_")
        s.write(name, f"# Map sweep: 4 CPU players, Classic, on '{mapfile}'.\n"
                      f"#@ seed={1000 + i}\n#@ map={mapfile}\n#@ frames={frames}\n#@ shots={frames - 1}\n", SWEEP_OUT)
    print(f"sweep: {len(maps)} maps")


def main():
    if sys.argv[1:] == ["sweep"]:
        sweep()
        return
    if sys.argv[1:] == ["segments"]:
        SEGMENT_OUT.mkdir(exist_ok=True)
        for n, make in SEGMENTS.items():
            make()
            print(n)
        return
    names = sys.argv[1:] or list(REPLAYS)
    for n in names:
        REPLAYS[n]()
        print(n)


if __name__ == "__main__":
    main()
