#!/usr/bin/env bash
# One run inside the flake_hunt container; tools/flake_hunt.sh sets the environment.
#   STRESS   stress-ng CPU workers beside the test (0: none)
#   JITTER   netem delay on loopback, as "<delay>ms <jitter>ms" (empty: none)
#   GAMES    scenarios for web_netplay_test.mjs
set -uo pipefail

cargo build -q --release -p smw_relay --manifest-path /repo/Cargo.toml --target-dir /cache/target || exit 2

if [[ "${STRESS:-0}" -gt 0 ]]; then
    stress-ng --cpu "$STRESS" --quiet &
fi
if [[ -n "${JITTER:-}" ]]; then
    # shellcheck disable=SC2086
    tc qdisc add dev lo root netem delay $JITTER || exit 2
fi

WEB_DIR=/web CHROME=/usr/local/bin/chrome RELAY_BIN=/cache/target/release/smw_relay NATIVE=0 NET_GAMES="$GAMES" \
    timeout 1200 node /repo/tools/web_netplay_test.mjs /out
status=$?
kill %1 2> /dev/null
exit $status
