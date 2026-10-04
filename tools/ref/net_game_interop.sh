#!/bin/bash
# Full-client netplay interop: two real game clients (C++ net-enabled harness build and/or the Rust port)
# meet on the C++ lobby server through the real menus, play a scripted net game, and compare their dumps.
# Usage: net_game_interop.sh [cpp-cpp|rust-rust|cpp-rust|rust-cpp ...]   (host-joiner; default: all four)
# Env: SMW_SERVER, SMW_CPP_NET (net-enabled harness smw), SMW_CPP_DATA, SMW_RUST_BIN, SMW_RUST_DATA,
#      NET_GAMES (scenario directories beside this script; default: every net_game*).
# A scenario's host.txt may name a map from net_maps/ (`#@ map=<name>`; each client gets an APFS clone of its data
# tree with the map added) and extra net_game_compare.py arguments (`#@ compare=...`). Only rust-rust pairings
# compare spawned powerups: the C++ harness does not record them.
HERE=$(cd "$(dirname "$0")" && pwd)
PORT=$(cd "$HERE/../.." && pwd)
SERVER=${SMW_SERVER:-$HOME/work/supermariowar-cpp-reference/build-net/smw-server}
CPP=${SMW_CPP_NET:-$HOME/work/supermariowar-cpp-reference/build-net/smw}
CPP_DATA=${SMW_CPP_DATA:-$HOME/work/supermariowar-cpp-reference/data}
RUST=${SMW_RUST_BIN:-$PORT/target/release/smw}
RUST_DATA=${SMW_RUST_DATA:-$PORT/data}
[ -n "$SMW_RUST_BIN" ] || cargo build --release --quiet --manifest-path "$PORT/Cargo.toml" || exit 2
WORK=$(mktemp -d)
pairs=("$@")
[ ${#pairs[@]} -gt 0 ] || pairs=(cpp-cpp rust-rust cpp-rust rust-cpp)
games=(${NET_GAMES:-net_game net_game_blocks net_game_frenzy net_game_stomp net_game_coins})

param() { sed -n "s/^#@ $1=//p" "$HERE/$2/host.txt"; }

datadir() {
  local kind=$1 role=$2 dir=$3 game=$4
  local data=$CPP_DATA map
  [ "$kind" = rust ] && data=$RUST_DATA
  map=$(param map "$game")
  if [ -n "$map" ]; then
    cp -c -R "$data" "$dir/data-$role"
    data=$dir/data-$role
    cp "$HERE/net_maps/$map.map" "$data/maps/"
  fi
  echo "$data"
}

client() {
  local kind=$1 role=$2 dir=$3 name=$4 game=$5 data=$6
  local bin=$CPP mapenv=()
  [ "$kind" = rust ] && bin=$RUST
  [ "$role" = host ] && [ -n "$(param map "$game")" ] && mapenv=(SMW_MAP="$(param map "$game")")
  local home=$dir/home-$role
  mkdir -p "$home/Library/Preferences/.smw"
  printf 'player_name = "%s"\nservers = ["127.0.0.1"]\n' "$name" > "$home/Library/Preferences/.smw/servers.toml"
  (cd "$data/.." && exec env HOME="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SMW_SEED=1 SMW_FRAMES="$(param frames "$game")" "${mapenv[@]}" \
    SMW_REPLAY="$HERE/$game/$role.txt" SMW_DUMP="$dir/$role.dump" \
    "$bin" --datadir "$data" > "$dir/$role.log" 2>&1)
}

fail=0
for game in "${games[@]}"; do
  for pair in "${pairs[@]}"; do
    h=${pair%-*} j=${pair#*-}
    dir=$WORK/$game/$pair
    mkdir -p "$dir"
    printf 'port 12521\nname Server\nmaxplayers 10\n' > "$dir/serverconfig"
    (cd "$dir" && exec "$SERVER" serverconfig > server.log 2>&1) & spid=$!
    sleep 0.5
    hdata=$(datadir "$h" host "$dir" "$game")
    jdata=$(datadir "$j" join "$dir" "$game")
    client "$h" host "$dir" Host "$game" "$hdata" & hpid=$!
    client "$j" join "$dir" Join "$game" "$jdata" & jpid=$!
    wait $hpid; hst=$?
    wait $jpid; jst=$?
    kill $spid 2>/dev/null; wait $spid 2>/dev/null
    rm -rf "${dir:?}/data-host" "${dir:?}/data-join"
    echo "== $game host=$h join=$j: host exit $hst, join exit $jst"
    args=($(param compare "$game"))
    [ "$pair" = rust-rust ] && args+=(--spawns)
    python3 "$HERE/net_game_compare.py" "${args[@]}" "$dir/host.dump" "$dir/join.dump" || fail=1
    [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
  done
done
echo "logs in $WORK"
exit $fail
