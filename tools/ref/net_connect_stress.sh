#!/bin/bash
# Connects the Rust client to the Rust lobby server N times on a clock that advances on every read, and fails if
# the Servers menu would not move on after any of them (the connect reply must be recorded before the skin is sent).
# Usage: net_connect_stress.sh [N]   (default 20)
# Env: SMW_RUST_SERVER (smw_server binary), SMW_RUST_NET (net_interop example binary), SMW_DATA_DIR.
HERE=$(cd "$(dirname "$0")" && pwd)
PORT=$(cd "$HERE/../.." && pwd)
DATA=${SMW_DATA_DIR:-$PORT/data}
N=${1:-20}
SERVER=${SMW_RUST_SERVER:-}
RUST=${SMW_RUST_NET:-}
if [ -z "$SERVER" ] || [ -z "$RUST" ]; then
  cargo build -q --release --locked --manifest-path "$PORT/Cargo.toml" --bin smw_server --example net_interop || exit 2
  SERVER=${SERVER:-$PORT/target/release/smw_server}
  RUST=${RUST:-$PORT/target/release/examples/net_interop}
fi
WORK=$(mktemp -d)
trap 'kill $spid 2>/dev/null; wait $spid 2>/dev/null; rm -rf "${WORK:?}"' EXIT
printf 'port 12521\nname Server\nmaxplayers 10\n' > "$WORK/serverconfig"
(cd "$WORK" && exec "$SERVER" serverconfig > server.log 2>&1) & spid=$!
sleep 0.5

fail=0
for i in $(seq "$N"); do
  mkdir -p "$WORK/home$i"
  HOME=$WORK/home$i SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
    perl -e 'alarm 20; exec @ARGV' "$RUST" connect "$DATA" > "$WORK/client$i.log" 2>&1
  status=$?
  line=$(grep -h '^EV servers menu' "$WORK/client$i.log")
  echo "connect $i: ${line:-no result (exit $status)}"
  if [ $status -ne 0 ]; then
    fail=1
    tail -n 5 "$WORK/client$i.log" | sed 's/^/    /'
  fi
done
echo "$([ $fail -eq 0 ] && echo PASS || echo FAIL): $N connects"
exit $fail
