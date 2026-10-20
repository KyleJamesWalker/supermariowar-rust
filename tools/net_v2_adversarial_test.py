#!/usr/bin/env python3
"""Netplay v2 matches between mismatched or misbehaving clients, each run live through smw_relay's lobby:

  rogue     net_game_3p, whose second joiner (SMW_V2_ROGUE) also claims the host's slot and sends its own start
            parts and GO; the others must ignore it, start the match, and stay in sync
  settings  net_game, whose joiner has sound and music off and lacks the music track the host's match plays; the
            joiner must play silence and stay in sync
  bad_map   net_game, whose host plays a truncated map; neither client may crash, and each must end the match
            or start it cleanly. Not run by default: the host's map menu still panics on a truncated map.

Usage: tools/net_v2_adversarial_test.py [work_dir]
  CASES            the cases to run (default: rogue settings; settings reuses rogue's options.bin)
  SMW_RUST_BIN     the game (default: target/release/smw)
  SMW_RELAY_BIN    the relay (default: target/release/smw_relay)
  SMW_RUST_DATA    the data directory (default: data)
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
REF = REPO / "tools" / "ref"
work = Path(sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="smw-net-v2-adv-")).resolve()
work.mkdir(parents=True, exist_ok=True)
EXE = ".exe" if os.name == "nt" else ""
smw = Path(os.environ.get("SMW_RUST_BIN", REPO / "target" / "release" / f"smw{EXE}")).resolve()
relay = Path(os.environ.get("SMW_RELAY_BIN", REPO / "target" / "release" / f"smw_relay{EXE}")).resolve()
data = Path(os.environ.get("SMW_RUST_DATA", REPO / "data")).resolve()
cases = (os.environ.get("CASES") or "rogue settings").split()
# Bytes of options.bin: 4 version i32s, then spawnstyle .. teamcolors, sound, music, musicvolume, soundvolume.
SOUND_OPTIONS = slice(23, 27)
MUSIC = "music/game/Standard/SMB_Forest.ogg"
TIMEOUT = 120


def settings_dir(home):
    """Where the game keeps its settings for HOME (USERPROFILE on Windows) = `home`."""
    return home / ".smw" if os.name == "nt" else home / "Library" / "Preferences" / ".smw"


def copy_data(dest):
    """A copy of the data tree named data/, so checkpoints name its files relative to it."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    if subprocess.run(["cp", "-c", "-R", str(data), str(dest)], stderr=subprocess.DEVNULL).returncode != 0:
        shutil.rmtree(dest, ignore_errors=True)
        shutil.copytree(data, dest)
    return dest


def frames(path):
    """Each frame's last saved state: a rollback saves a frame again, and the last save is the confirmed one."""
    out, cur = {}, None
    for line in path.read_text(errors="replace").splitlines():
        if line.startswith("#F "):
            fields = line.split()
            cur = [fields[2]]
            out[int(fields[1])] = cur
        elif cur is not None:
            cur.append(line)
    return out


def run(case, scenario, roles):
    """Plays `scenario` with one client per role, each {role: (name, data dir, extra env, options.bin or None)}, and
    returns each role's log text and exit status. Every client must exit within TIMEOUT seconds."""
    dir = work / case
    dir.mkdir(parents=True, exist_ok=True)
    (dir / "serverconfig").write_text("port 12521\nname Server\nmaxplayers 10\n")
    server = subprocess.Popen([str(relay), "--port", "12521", "--config", "serverconfig"], cwd=dir,
                              stdout=open(dir / "server.log", "w"), stderr=subprocess.STDOUT)
    procs = {}
    try:
        end = time.time() + 10
        while "Ready!" not in (dir / "server.log").read_text(errors="replace") and time.time() < end:
            time.sleep(0.1)
        frames_param = re.search(r"^#@ frames=(\d+)", (REF / scenario / "host.txt").read_text(), re.M).group(1)
        for role, (name, root, env, options) in roles.items():
            home = dir / f"home-{role}"
            prefs = settings_dir(home)
            prefs.mkdir(parents=True, exist_ok=True)
            (prefs / "servers.toml").write_text(f'player_name = "{name}"\nservers = ["ws://127.0.0.1:12521"]\n')
            if options is not None:
                (prefs / "options.bin").write_bytes(options)
            full = dict(os.environ, HOME=str(home), USERPROFILE=str(home), SMW_LAN_DISCOVERY="0",
                        SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy", SMW_SEED="1", SMW_FRAMES=frames_param,
                        SMW_REPLAY=str(REF / scenario / f"{role}.txt"), SMW_V2_STATE_LOG=str(dir / f"{role}.v2state.log"))
            full.update(env)
            procs[role] = subprocess.Popen([str(smw), "--datadir", str(root)], cwd=root.parent, env=full,
                                           stdout=open(dir / f"{role}.log", "w"), stderr=subprocess.STDOUT)
            time.sleep(0.2)
        status = {}
        for role, p in procs.items():
            try:
                status[role] = p.wait(timeout=TIMEOUT)
            except subprocess.TimeoutExpired:
                p.kill()
                status[role] = "timeout"
    finally:
        for p in list(procs.values()) + [server]:
            if p.poll() is None:
                p.kill()
    logs = {role: (dir / f"{role}.log").read_text(errors="replace") for role in roles}
    return dir, logs, status


def in_sync(dir, roles, problems):
    """Every client started the match, none reported a desync, and every joiner's confirmed states equal the host's."""
    host = frames(dir / "host.v2state.log") if (dir / "host.v2state.log").exists() else {}
    for role in roles:
        log = (dir / f"{role}.log").read_text(errors="replace")
        if "[net] v2: GGRS session started" not in log:
            problems.append(f"{role} did not start the match")
        if "[net] v2: desync" in log:
            problems.append(f"{role} reported a desync")
        if role == "host":
            continue
        path = dir / f"{role}.v2state.log"
        theirs = frames(path) if path.exists() else {}
        if not host or not theirs:
            problems.append(f"no states to compare with {role}")
            continue
        last = min(max(host), max(theirs)) - 8
        common = sorted(f for f in set(host) & set(theirs) if f <= last)
        differ = [f for f in common if host[f] != theirs[f]]
        if differ:
            problems.append(f"host and {role} states differ from frame {differ[0]}")
        elif len(common) < 300:
            problems.append(f"only {len(common)} frames to compare with {role}")


def crashed(status, problems):
    for role, code in status.items():
        if code != 0:
            problems.append(f"{role} exited with {code}")


failed = False
for case in cases:
    problems = []
    if case == "rogue":
        roles = {"host": ("Host", data, {}, None), "join": ("Join", data, {}, None),
                 "join2": ("Join2", data, {"SMW_V2_ROGUE": "1"}, None)}
        dir, logs, status = run(case, "net_game_3p", roles)
        crashed(status, problems)
        in_sync(dir, roles, problems)
        for role in ("host", "join"):
            if "[net] v2: ignoring peer" not in logs[role]:
                problems.append(f"{role} did not refuse the rogue's claim on the host's slot")
    elif case == "settings":
        options = work / "rogue" / "home-join" / settings_dir(Path()) / "options.bin"
        if not options.exists():
            print(f"settings: needs {options}, which the rogue case writes")
            failed = True
            continue
        quiet = bytearray(options.read_bytes())
        quiet[SOUND_OPTIONS] = bytes(4)
        joiner = copy_data(work / case / "join" / "data")
        (joiner / MUSIC).unlink()
        roles = {"host": ("Host", data, {}, None), "join": ("Join", joiner, {}, bytes(quiet))}
        dir, logs, status = run(case, "net_game", roles)
        crashed(status, problems)
        in_sync(dir, roles, problems)
        if "playing silence instead" not in logs["join"]:
            problems.append("the joiner did not fall back to silence for the missing track")
        shutil.rmtree(joiner, ignore_errors=True)
    elif case == "bad_map":
        host = copy_data(work / case / "host" / "data")
        whole = (REF / "net_maps" / "netplay_arena.map").read_bytes()
        (host / "maps" / "netplay_truncated.map").write_bytes(whole[: len(whole) // 2])
        roles = {"host": ("Host", host, {"SMW_MAP": "netplay_truncated"}, None), "join": ("Join", data, {}, None)}
        dir, logs, status = run(case, "net_game", roles)
        crashed(status, problems)
        shutil.rmtree(host, ignore_errors=True)
    else:
        problems.append("unknown case")
    print(f"{case}: " + ("; ".join(problems) if problems else "ok"))
    failed |= bool(problems)

print(f"logs in {work}")
sys.exit(1 if failed else 0)
