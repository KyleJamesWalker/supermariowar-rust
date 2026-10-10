#!/bin/bash
# Full-client netplay: two Rust game clients meet on the Rust lobby server through the real menus, play each
# scripted net game, and compare their dumps (net_game_compare.py, including spawned outcomes). Each client records
# the match (SMW_RECORD_TO), which must then replay offline, with no server, to the client's own dump, and every
# match of it must replay from its checkpoint (segment_check.py).
# Usage: net_game_interop.sh
# Env: SMW_RUST_BIN (smw), SMW_RUST_SERVER (smw_server), SMW_RUST_DATA,
#      NET_GAMES (scenario directories beside this script; default: every net_game*).
# A scenario's host.txt may name a map from net_maps/ (`#@ map=<name>`; each client gets an APFS clone of its data
# tree with the map added) and extra net_game_compare.py arguments (`#@ compare=...`).
HERE=$(cd "$(dirname "$0")" && pwd)
PORT=$(cd "$HERE/../.." && pwd)
RUST=${SMW_RUST_BIN:-$PORT/target/release/smw}
SERVER=${SMW_RUST_SERVER:-$PORT/target/release/smw_server}
RUST_DATA=${SMW_RUST_DATA:-$PORT/data}
if [ -z "$SMW_RUST_BIN" ] || [ -z "$SMW_RUST_SERVER" ]; then
  cargo build --release --quiet --locked --manifest-path "$PORT/Cargo.toml" --bin smw --bin smw_server || exit 2
fi
WORK=$(mktemp -d)
games=(${NET_GAMES:-net_game net_game_blocks net_game_frenzy net_game_stomp net_game_coins net_game_classic})

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
  local home=$dir/home-$role
  mkdir -p "$home/Library/Preferences/.smw"
  printf 'player_name = "%s"\nservers = ["127.0.0.1"]\n' "$name" > "$home/Library/Preferences/.smw/servers.toml"
  (cd "$data/.." && exec env HOME="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SMW_SEED=1 SMW_FRAMES="$(param frames "$game")" "${mapenv[@]}" \
    SMW_REPLAY="$HERE/$game/$role.txt" SMW_DUMP="$dir/$role.dump" "$@" \
    "$RUST" --datadir "$data")
}

fail=0
for game in "${games[@]}"; do
  dir=$WORK/$game
  mkdir -p "$dir"
  printf 'port 12521\nname Server\nmaxplayers 10\n' > "$dir/serverconfig"
  (cd "$dir" && exec "$SERVER" serverconfig > server.log 2>&1) & spid=$!
  sleep 0.5
  hdata=$(datadir host "$dir" "$game")
  jdata=$(datadir join "$dir" "$game")
  client host "$dir" Host "$game" "$hdata" SMW_RECORD_TO="$dir/${game}_host.rec.txt" > "$dir/host.log" 2>&1 & hpid=$!
  client join "$dir" Join "$game" "$jdata" SMW_RECORD_TO="$dir/${game}_join.rec.txt" > "$dir/join.log" 2>&1 & jpid=$!
  wait $hpid; hst=$?
  wait $jpid; jst=$?
  kill $spid 2>/dev/null; wait $spid 2>/dev/null
  echo "== $game: host exit $hst, join exit $jst"
  python3 "$HERE/net_game_compare.py" --spawns $(param compare "$game") "$dir/host.dump" "$dir/join.dump" || fail=1
  [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
  for role in host join; do
    data=$hdata name=Host; [ $role = join ] && data=$jdata name=Join
    rm -rf "${dir:?}/home-$role"
    client $role "$dir" $name "$game" "$data" SMW_NOLIMIT=1 SMW_REPLAY="$dir/${game}_$role.rec.txt" SMW_DUMP="$dir/$role.replay.dump" > "$dir/$role.replay.log" 2>&1
    if cmp -s "$dir/$role.dump" "$dir/$role.replay.dump"; then
      echo "  $role offline replay: 0 dump differences"
    else
      echo "  $role offline replay: $(diff "$dir/$role.dump" "$dir/$role.replay.dump" | grep -c '^<') dump differences"
      fail=1
    fi
  done
  rm -rf "${dir:?}/host/data" "${dir:?}/join/data"
done

# Every match of the recordings from its checkpoint, on a data tree with the scenarios' maps.
segdata=$WORK/data
cp -c -R "$RUST_DATA" "$segdata" 2>/dev/null || cp -R "$RUST_DATA" "$segdata"
cp "$HERE"/net_maps/*.map "$segdata/maps/"
SMW_NO_BUILD=1 SMW_BIN="$RUST" SMW_DATA_DIR="$segdata" python3 "$PORT/tools/segment_check.py" "$WORK"/*/*.rec.txt || fail=1
echo "logs in $WORK"
exit $fail
