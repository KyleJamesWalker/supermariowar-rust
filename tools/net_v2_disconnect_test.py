#!/usr/bin/env python3
"""Netplay v2 mid-match disconnect: the host and a joiner start tools/ref/net_game's match, the joiner is killed a few
seconds in, and the host must show the joiner as lagging, wait at the prediction window, report the joiner as dropped
out, end the match, and leave a recording whose replay logs the same states.

Usage: tools/net_v2_disconnect_test.py [work_dir]
  SMW_RUST_BIN     the game (default: target/release/smw)
  SMW_RELAY_BIN    the relay (default: target/release/smw_relay)
  SMW_RUST_DATA    the data directory (default: data)
"""
import base64
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCENARIO = REPO / "tools" / "ref" / "net_game"
work = Path(sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="smw-net-v2-drop-")).resolve()
work.mkdir(parents=True, exist_ok=True)
EXE = ".exe" if os.name == "nt" else ""
smw = Path(os.environ.get("SMW_RUST_BIN", REPO / "target" / "release" / f"smw{EXE}")).resolve()
relay = Path(os.environ.get("SMW_RELAY_BIN", REPO / "target" / "release" / f"smw_relay{EXE}")).resolve()
data = Path(os.environ.get("SMW_RUST_DATA", REPO / "data")).resolve()
# The match stalls once the joiner is gone, and the host waits out GGRS's 20 s disconnect timeout.
FRAMES = 3000


def settings_dir(home):
    """Where the game keeps its settings for HOME (USERPROFILE on Windows) = `home`."""
    return home / ".smw" if os.name == "nt" else home / "Library" / "Preferences" / ".smw"


def client(role, name, log_name=None, **env):
    home = work / f"home-{role}"
    prefs = settings_dir(home)
    prefs.mkdir(parents=True, exist_ok=True)
    (prefs / "servers.toml").write_text(f'player_name = "{name}"\nservers = ["ws://127.0.0.1:12521"]\n')
    full = dict(os.environ, HOME=str(home), USERPROFILE=str(home), SMW_LAN_DISCOVERY="0", SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy",
                SMW_SEED="1", SMW_FRAMES=str(FRAMES), SMW_REPLAY=str(SCENARIO / f"{role}.txt"))
    full.update(env)
    log = open(work / f"{log_name or role}.log", "w")
    return subprocess.Popen([str(smw), "--datadir", str(data)], cwd=data.parent, env=full, stdout=log, stderr=subprocess.STDOUT)


def wait_for(path, text, timeout):
    end = time.time() + timeout
    while time.time() < end:
        if path.exists() and text in path.read_text(errors="replace"):
            return True
        time.sleep(0.1)
    return False


(work / "serverconfig").write_text("port 12521\nname Server\nmaxplayers 10\n")
server = subprocess.Popen([str(relay), "--port", "12521", "--config", "serverconfig"], cwd=work,
                          stdout=open(work / "server.log", "w"), stderr=subprocess.STDOUT)
procs = [server]
failures = []
try:
    if not wait_for(work / "server.log", "Ready!", 10):
        sys.exit("smw_relay did not start")
    host = client("host", "Host", SMW_RECORD_TO=str(work / "host.rec.txt"), SMW_V2_STATE_LOG=str(work / "host.v2state.log"))
    join = client("join", "Join")
    procs += [host, join]
    if not wait_for(work / "join.log", "GGRS session started", 60):
        sys.exit("the match did not start")
    time.sleep(3)
    join.kill()
    join.wait()
    host.wait(timeout=120)
    log = (work / "host.log").read_text(errors="replace")
    for want in ("[net] v2: Join dropped out", "[net] v2: match over"):
        if want not in log:
            failures.append(f"the host log lacks {want!r}")
    if not any(int(n) > 0 for n in re.findall(r"(\d+) waits for input", log)):
        failures.append("the host never waited for the dead joiner's input")
    lagging = base64.b64encode(b"Join is lagging").decode()
    if f".{lagging}" not in (work / "host.rec.txt").read_text(errors="replace"):
        failures.append("the host's recording has no 'Join is lagging' notice")

    replay = client("host", "Host", "host.replay", SMW_NOLIMIT="1", SMW_REPLAY=str(work / "host.rec.txt"),
                    SMW_V2_STATE_LOG=str(work / "host.replay.v2state.log"))
    procs.append(replay)
    if replay.wait(timeout=300) != 0:
        failures.append(f"the host's replay exited with {replay.returncode}")
    elif (work / "host.v2state.log").read_bytes() != (work / "host.replay.v2state.log").read_bytes():
        failures.append("the host's replay logs other states than the live match")
finally:
    for p in procs:
        if p.poll() is None:
            p.kill()

for f in failures:
    print(f"FAIL: {f}")
print(f"{'FAIL' if failures else 'ok'}: mid-match disconnect; logs in {work}")
sys.exit(1 if failures else 0)
