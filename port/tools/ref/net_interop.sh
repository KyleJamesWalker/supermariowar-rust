#!/bin/bash
# Netplay interop: the C++ lobby server (smw-server) with every host/joiner pairing of the C++ client
# (net_driver, see net_driver.sh) and the Rust client (examples/net_interop.rs).
# Env: SMW_SERVER (smw-server binary), SMW_NET_DRIVER (net_driver), SMW_RUST_NET (net_interop binary).
HERE=$(cd $(dirname $0) && pwd)
PORT=$HERE/../..
DATA=${SMW_DATA_DIR:-$PORT/../data}
SERVER=${SMW_SERVER:-$HOME/work/smw-ref-net/build/smw-server}
CPP=${SMW_NET_DRIVER:-/tmp/net_driver}
RUST=${SMW_RUST_NET:-}
if [ -z "$RUST" ]; then (cd $PORT && cargo build -q --example net_interop) || exit 2; RUST=$PORT/target/debug/examples/net_interop; fi
WORK=$(mktemp -d)
fail=0
run_pair() {
  local hname=$1 hbin=$2 jname=$3 jbin=$4
  local dir=$WORK/$hname-$jname
  mkdir -p $dir/hh/Library/Preferences $dir/hj/Library/Preferences
  cp $HERE/../../../src/server/serverconfig $dir/ 2>/dev/null || printf 'port 12521\nname Server\nmaxplayers 10\n' > $dir/serverconfig
  (cd $dir && exec $SERVER serverconfig > server.log 2>&1) & local spid=$!
  sleep 0.5
  HOME=$dir/hh SDL_VIDEODRIVER=dummy $hbin host "$DATA" > $dir/host.log 2>&1 & local hpid=$!
  sleep 0.7
  HOME=$dir/hj SDL_VIDEODRIVER=dummy $jbin join "$DATA" > $dir/join.log 2>&1; local jst=$?
  wait $hpid; local hst=$?
  kill $spid 2>/dev/null; wait $spid 2>/dev/null
  echo "== host=$hname join=$jname: host exit $hst, join exit $jst"
  grep -h "^EV" $dir/host.log | sed 's/^/  host: /'
  grep -h "^EV" $dir/join.log | sed 's/^/  join: /'
  [ $hst -eq 0 ] && [ $jst -eq 0 ] || fail=1
  if cmp -s $dir/hj/Library/Preferences/.smw/net_last.map "$DATA/maps/0smw.map"; then echo "  map transfer: identical"; else echo "  map transfer: DIFFERS"; fail=1; fi
  grep -q "hello from host" $dir/join.log && echo "  chat relayed" || { echo "  chat missing"; fail=1; }
  # The server shares the joiner's skin with the room host only.
  [ -f $dir/hh/Library/Preferences/.smw/net_skin1.bmp ] && echo "  joiner skin reached host" || { echo "  joiner skin missing"; fail=1; }
}
run_pair cpp "$CPP" cpp "$CPP"
run_pair rust "$RUST" rust "$RUST"
run_pair cpp "$CPP" rust "$RUST"
run_pair rust "$RUST" cpp "$CPP"
echo "logs in $WORK"
exit $fail
