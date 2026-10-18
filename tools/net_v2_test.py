#!/usr/bin/env python3
"""Netplay v2 matches end to end: the scripted net games of tools/ref/net_game_interop.sh with SMW_NETPLAY_V2=1, so
each match runs under GGRS over WebRTC, with lag and packet loss on the GGRS channel. Both clients log the state they
checksum at every saved frame (SMW_V2_STATE_LOG); every frame both logged must be identical, every match must start
its GGRS session, and neither client may report a desync.

Usage: tools/net_v2_test.py [work_dir]
  NET_GAMES        scenarios (default: every tools/ref/net_game*); a scenario's `matches` file sets how many GGRS
                   sessions each client must start (default 1)
  SMW_PEER_LAG_MS  delay per GGRS packet (default 60)
  SMW_PEER_LOSS    percent of GGRS packets dropped (default 10)
  LAN=1            no smw_server and no signaling server: the host runs SMW_LAN_HOST=1 and both clients list
                   ws://127.0.0.1:12523, as players on a network without internet do
  SMW_RUST_BIN, SIGNAL_BIN  binaries (default: target/release/smw, target/release/examples/signal)
"""
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
work = Path(sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="smw-net-v2-")).resolve()
work.mkdir(parents=True, exist_ok=True)
games = os.environ.get("NET_GAMES") or " ".join(sorted(p.name for p in (REPO / "tools" / "ref").glob("net_game*") if p.is_dir()))
smw = Path(os.environ.get("SMW_RUST_BIN", REPO / "target" / "release" / "smw")).resolve()
signal = Path(os.environ.get("SIGNAL_BIN", REPO / "target" / "release" / "examples" / "signal")).resolve()
states = work / "states"
states.mkdir(exist_ok=True)

# net_game_interop.sh runs each client with HOME=<work>/<game>/home-<role>; the wrapper names the state log after both.
wrapper = work / "smw_v2.sh"
wrapper.write_text(
    "#!/bin/bash\n"
    f'export SMW_V2_STATE_LOG="{states}/$(basename "$(dirname "$HOME")")_$(basename "$HOME").log"\n'
    f'exec "{smw}" "$@"\n'
)
wrapper.chmod(0o755)

lan = os.environ.get("LAN") == "1"
server = None
if not lan:
    server = subprocess.Popen([str(signal)], stdout=subprocess.PIPE, text=True)
    port = None
    for line in server.stdout:
        m = re.match(r"listening (\d+)", line)
        if m:
            port = int(m.group(1))
            break
    if port is None:
        sys.exit("the signal example did not start")

env = dict(
    os.environ,
    SMW_NETPLAY_V2="1",
    NET_GAMES=games,
    NET_GAME_WORK=str(work),
    SMW_RUST_BIN=str(wrapper),
    SMW_NO_BUILD="1",
    SMW_PEER_LAG_MS=os.environ.get("SMW_PEER_LAG_MS", "60"),
    SMW_PEER_LOSS=os.environ.get("SMW_PEER_LOSS", "10"),
)
if lan:
    env.update(NET_SERVER_ENTRY="ws://127.0.0.1:12523", NET_NO_SERVER="1", HOST_ENV="SMW_LAN_HOST=1")
else:
    env["SMW_SIGNAL_URL"] = f"ws://127.0.0.1:{port}"

try:
    # Its v1 dump comparison does not apply: v2 frames write no harness dump.
    subprocess.run(["bash", str(REPO / "tools" / "ref" / "net_game_interop.sh")], cwd=REPO, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
finally:
    if server:
        server.kill()


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


failed = False
for game in games.split():
    problems = []
    logs = {role: (work / game / f"{role}.log").read_text(errors="replace") for role in ("host", "join")}
    matches_file = REPO / "tools" / "ref" / game / "matches"
    matches = int(matches_file.read_text()) if matches_file.exists() else 1
    for role, log in logs.items():
        started = log.count("[net] v2: GGRS session started")
        if started < matches:
            problems.append(f"{role} started {started} of {matches} GGRS sessions")
        desyncs = [l for l in log.splitlines() if "[net] v2: desync" in l]
        if desyncs:
            problems.append(f"{role}: {desyncs[0]}")
    reruns = sum(int(m) for log in logs.values() for m in re.findall(r"(\d+) re-run ticks", log)[-1:])
    host, join = states / f"{game}_home-host.log", states / f"{game}_home-join.log"
    compared = 0
    if host.exists() and join.exists():
        h, j = frames(host), frames(join)
        # The last frames may be predictions the session never confirmed before the match ended.
        last = min(max(h), max(j)) - 8
        common = sorted(f for f in set(h) & set(j) if f <= last)
        compared = len(common)
        differ = [f for f in common if h[f] != j[f]]
        if differ:
            problems.append(f"states differ from frame {differ[0]}")
        if compared < 300:
            problems.append(f"only {compared} frames to compare")
    else:
        problems.append("no state log")
    print(f"{game}: {compared} frames compared, {reruns} re-run ticks, " + ("; ".join(problems) if problems else "identical"))
    failed |= bool(problems)

print(f"logs in {work}")
sys.exit(1 if failed else 0)
