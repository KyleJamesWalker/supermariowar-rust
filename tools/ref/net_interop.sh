#!/bin/bash
# Netplay interop: the C++ lobby server (smw-server) and the Rust one (bin smw_server), each with every
# host/joiner pairing of the C++ client (net_driver, see net_driver.sh) and the Rust client (examples/net_interop.rs).
# Env: SMW_SERVER (smw-server binary), SMW_RUST_SERVER (smw_server binary), SMW_NET_DRIVER (net_driver),
# SMW_RUST_NET (net_interop binary).
HERE=$(cd $(dirname $0) && pwd)
PORT=$HERE/../..
DATA=${SMW_DATA_DIR:-$PORT/data}
SERVER=${SMW_SERVER:-$HOME/work/supermariowar-cpp-reference/build-net/smw-server}
CPP=${SMW_NET_DRIVER:-/tmp/net_driver}
RUST=${SMW_RUST_NET:-}
if [ -z "$RUST" ]; then (cd $PORT && cargo build -q --example net_interop) || exit 2; RUST=$PORT/target/debug/examples/net_interop; fi
RUST_SERVER=${SMW_RUST_SERVER:-}
if [ -z "$RUST_SERVER" ]; then (cd $PORT && cargo build -q --bin smw_server) || exit 2; RUST_SERVER=$PORT/target/debug/smw_server; fi
WORK=$(mktemp -d)
fail=0
# wait_for <file> <pattern>: poll every 0.1 s, give up after 15 s.
wait_for() {
  for _ in $(seq 150); do grep -q "$2" "$1" 2>/dev/null && return 0; sleep 0.1; done
  return 1
}
run_pair() {
  local sname=$1 sbin=$2 hname=$3 hbin=$4 jname=$5 jbin=$6
  local dir=$WORK/$sname-$hname-$jname
  mkdir -p $dir/hh/Library/Preferences $dir/hj/Library/Preferences
  cp $HERE/../../../src/server/serverconfig $dir/ 2>/dev/null || printf 'port 12521\nname Server\nmaxplayers 10\n' > $dir/serverconfig
  (cd $dir && exec $sbin serverconfig > server.log 2>&1) & local spid=$!
  sleep 0.5
  HOME=$dir/hh SDL_VIDEODRIVER=dummy $hbin host "$DATA" > $dir/host.log 2>&1 & local hpid=$!
  # serverlog.txt is flushed per line on both servers; the C++ client's stdout is not.
  if wait_for $dir/serverlog.txt "New room by"; then sleep 0.3; else echo "  host never created a room"; fail=1; fi
  HOME=$dir/hj SDL_VIDEODRIVER=dummy $jbin join "$DATA" > $dir/join.log 2>&1; local jst=$?
  wait $hpid; local hst=$?
  kill $spid 2>/dev/null; wait $spid 2>/dev/null
  echo "== server=$sname host=$hname join=$jname: host exit $hst, join exit $jst"
  grep -h "^EV" $dir/host.log | sed 's/^/  host: /'
  grep -h "^EV" $dir/join.log | sed 's/^/  join: /'
  [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
  if cmp -s $dir/hj/Library/Preferences/.smw/net_last.map "$DATA/maps/0smw.map"; then echo "  map transfer: identical"; else echo "  map transfer: DIFFERS"; fail=1; fi
  grep -q "hello from host" $dir/join.log && echo "  chat relayed" || { echo "  chat missing"; fail=1; }
  # The server shares the joiner's skin with the room host only.
  [ -f $dir/hh/Library/Preferences/.smw/net_skin1.bmp ] && echo "  joiner skin reached host" || { echo "  joiner skin missing"; fail=1; }
}
for srv in "cpp $SERVER" "rust $RUST_SERVER"; do
  set -- $srv
  run_pair $1 "$2" cpp "$CPP" cpp "$CPP"
  run_pair $1 "$2" rust "$RUST" rust "$RUST"
  run_pair $1 "$2" cpp "$CPP" rust "$RUST"
  run_pair $1 "$2" rust "$RUST" cpp "$CPP"
done
echo "logs in $WORK"
exit $fail
