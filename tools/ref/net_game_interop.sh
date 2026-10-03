#!/bin/bash
# Full-client netplay interop: two real game clients (C++ net-enabled harness build and/or the Rust port)
# meet on the C++ lobby server through the real menus, play a scripted net game, and compare their dumps.
# Usage: net_game_interop.sh [cpp-cpp|rust-rust|cpp-rust|rust-cpp ...]   (host-joiner; default: all four)
# Env: SMW_SERVER, SMW_CPP_NET (net-enabled harness smw), SMW_CPP_DATA, SMW_RUST_BIN, SMW_RUST_DATA.
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

client() {
  local kind=$1 role=$2 dir=$3 name=$4
  local bin=$CPP data=$CPP_DATA
  [ "$kind" = rust ] && bin=$RUST data=$RUST_DATA
  local home=$dir/home-$role
  mkdir -p "$home/Library/Preferences/.smw"
  printf 'player_name = "%s"\nservers = ["127.0.0.1"]\n' "$name" > "$home/Library/Preferences/.smw/servers.toml"
  (cd "$data/.." && exec env HOME="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SMW_SEED=1 SMW_FRAMES=1100 \
    SMW_REPLAY="$HERE/net_game/$role.txt" SMW_DUMP="$dir/$role.dump" \
    "$bin" --datadir "$data" > "$dir/$role.log" 2>&1)
}

fail=0
for pair in "${pairs[@]}"; do
  h=${pair%-*} j=${pair#*-}
  dir=$WORK/$pair
  mkdir -p "$dir"
  printf 'port 12521\nname Server\nmaxplayers 10\n' > "$dir/serverconfig"
  (cd "$dir" && exec "$SERVER" serverconfig > server.log 2>&1) & spid=$!
  sleep 0.5
  client "$h" host "$dir" Host & hpid=$!
  client "$j" join "$dir" Join & jpid=$!
  wait $hpid; hst=$?
  wait $jpid; jst=$?
  kill $spid 2>/dev/null; wait $spid 2>/dev/null
  echo "== host=$h join=$j: host exit $hst, join exit $jst"
  python3 "$HERE/net_game_compare.py" "$dir/host.dump" "$dir/join.dump" || fail=1
  [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
done
echo "logs in $WORK"
exit $fail
