#!/bin/bash
# Full-client netplay: two Rust game clients meet on smw_relay's lobby through the real menus and play each
# scripted net game. Each client records its session (SMW_RECORD_TO), which then replays offline, with no relay, writing
# its v2 state log to <game>/<role>.replay.v2state.log, and its match 1 replays from its checkpoint (SMW_SEGMENT=1) to
# <game>/<role>.segment.v2state.log. tools/net_v2_test.py runs this and checks the matches.
# Usage: net_game_run.sh
# Env: SMW_RUST_BIN (smw), SMW_RELAY_BIN (smw_relay), SMW_RUST_DATA,
#      NET_GAMES (scenario directories beside this script; default: every net_game*),
#      NET_GAME_WORK (where the logs go; default: a temporary directory),
#      HOST_PRESENT_HZ / JOIN_PRESENT_HZ (that client presents at this display rate while ticking at 60, SMW_PRESENT_HZ),
#      NET_SERVER_ENTRY (the clients' server list entry; default ws://127.0.0.1:12521, the smw_relay this starts),
#      NET_NO_SERVER=1 (start no smw_relay), HOST_ENV (extra VAR=value settings for the host only).
# A scenario's host.txt may name a map from net_maps/ (`#@ map=<name>`; each client gets an APFS clone of its data
# tree with the map added) and a host seed (`#@ net_seed=<n>`, so the game spawns alike every run).
HERE=$(cd "$(dirname "$0")" && pwd)
PORT=$(cd "$HERE/../.." && pwd)
RUST=${SMW_RUST_BIN:-$PORT/target/release/smw}
RELAY=${SMW_RELAY_BIN:-$PORT/target/release/smw_relay}
RUST_DATA=${SMW_RUST_DATA:-$PORT/data}
if [ -z "$SMW_NO_BUILD" ]; then
  cargo build --release --quiet --locked --manifest-path "$PORT/Cargo.toml" --bin smw --bin smw_relay || exit 2
fi
WORK=${NET_GAME_WORK:-$(mktemp -d)}
mkdir -p "$WORK"
games=(${NET_GAMES:-net_game net_game_blocks net_game_frenzy net_game_stomp net_game_coins net_game_classic net_game_stompkill net_game_stompkill_powerup})

param() { sed -n "s/^#@ $1=//p" "$HERE/$2/host.txt"; }

datadir() {
  local role=$1 dir=$2 game=$3
  local data=$RUST_DATA map
  map=$(param map "$game")
  if [ -n "$map" ]; then
    # Named data/, so checkpoints name its files relative to it.
    mkdir -p "$dir/$role"
    cp -c -R "$data" "$dir/$role/data" 2>/dev/null || cp -R "$data" "$dir/$role/data"
    data=$dir/$role/data
    cp "$HERE/net_maps/$map.map" "$data/maps/"
  fi
  echo "$data"
}

client() {
  local role=$1 dir=$2 name=$3 game=$4 data=$5
  shift 5
  local mapenv=()
  [ "$role" = host ] && [ -n "$(param map "$game")" ] && mapenv=(SMW_MAP="$(param map "$game")")
  [ "$role" = host ] && [ -n "$(param net_seed "$game")" ] && mapenv+=(SMW_NET_SEED="$(param net_seed "$game")")
  [ "$role" = host ] && [ -n "${HOST_PRESENT_HZ:-}" ] && mapenv+=(SMW_PRESENT_HZ="$HOST_PRESENT_HZ")
  [ "$role" = join ] && [ -n "${JOIN_PRESENT_HZ:-}" ] && mapenv+=(SMW_PRESENT_HZ="$JOIN_PRESENT_HZ")
  local home=$dir/home-$role
  mkdir -p "$home/Library/Preferences/.smw"
  printf 'player_name = "%s"\nservers = ["%s"]\n' "$name" "${NET_SERVER_ENTRY:-ws://127.0.0.1:12521}" > "$home/Library/Preferences/.smw/servers.toml"
  (cd "$data/.." && exec env HOME="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SMW_SEED=1 SMW_FRAMES="$(param frames "$game")" "${mapenv[@]}" \
    SMW_REPLAY="$HERE/$game/$role.txt" SMW_DUMP="$dir/$role.dump" "$@" \
    "$RUST" --datadir "$data")
}

fail=0
for game in "${games[@]}"; do
  dir=$WORK/$game
  mkdir -p "$dir"
  printf 'port 12521\nname Server\nmaxplayers 10\n' > "$dir/serverconfig"
  spid=
  if [ -z "${NET_NO_SERVER:-}" ]; then
    (cd "$dir" && exec "$RELAY" --port 12521 --config serverconfig > server.log 2>&1) & spid=$!
  fi
  sleep 0.5
  hdata=$(datadir host "$dir" "$game")
  jdata=$(datadir join "$dir" "$game")
  # shellcheck disable=SC2086
  client host "$dir" Host "$game" "$hdata" SMW_RECORD_TO="$dir/host.rec.txt" ${HOST_ENV:-} > "$dir/host.log" 2>&1 & hpid=$!
  client join "$dir" Join "$game" "$jdata" SMW_RECORD_TO="$dir/join.rec.txt" > "$dir/join.log" 2>&1 & jpid=$!
  wait $hpid; hst=$?
  wait $jpid; jst=$?
  [ -n "$spid" ] && { kill $spid 2>/dev/null; wait $spid 2>/dev/null; }
  echo "== $game: host exit $hst, join exit $jst"
  [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
  for role in host join; do
    data=$hdata name=Host; [ $role = join ] && data=$jdata name=Join
    mv "$dir/home-$role" "$dir/home-$role.live"
    client $role "$dir" $name "$game" "$data" SMW_NOLIMIT=1 SMW_REPLAY="$dir/$role.rec.txt" SMW_DUMP="$dir/$role.replay.dump" \
      SMW_FRAMES="$(sed -n 's/^#@ frames=//p' "$dir/$role.rec.txt")" SMW_V2_STATE_LOG="$dir/$role.replay.v2state.log" > "$dir/$role.replay.log" 2>&1
    client $role "$dir" $name "$game" "$data" SMW_NOLIMIT=1 SMW_REPLAY="$dir/$role.rec.txt" SMW_SEGMENT=1 SMW_DUMP="$dir/$role.segment.dump" \
      SMW_FRAMES="$(sed -n 's/^#@ frames=//p' "$dir/$role.rec.txt")" SMW_V2_STATE_LOG="$dir/$role.segment.v2state.log" > "$dir/$role.segment.log" 2>&1
  done
  rm -rf "${dir:?}/host/data" "${dir:?}/join/data"
done

echo "logs in $WORK"
exit $fail
