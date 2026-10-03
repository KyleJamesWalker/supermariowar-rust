#!/bin/bash
# servers.toml compatibility: the C++ NetConfigManager (net_config_ref.sh) and the Rust one (examples/net_config.rs)
# each load the same fuzzed servers.toml and save it back; the saved files and the warnings must be identical.
# Usage: net_config_interop.sh [cases]   (default 200)
HERE=$(cd "$(dirname "$0")" && pwd)
PORT=$(dirname "$(dirname "$HERE")")
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
"$HERE/net_config_ref.sh" "$WORK/cpp" || exit 2
cargo build --release --quiet --example net_config --manifest-path "$PORT/Cargo.toml" 2> /dev/null || exit 2
RUST=$PORT/target/release/examples/net_config
fail=0
for seed in $(seq 1 "${1:-200}"); do
  for side in cpp rust; do
    mkdir -p "$WORK/$side-home/Library/Preferences/.smw"
    python3 -c "import sys, random; sys.path.insert(0, '$HERE'); import persist_fuzz as p; sys.stdout.write(p.servers_toml(random.Random($seed)))" \
      > "$WORK/$side-home/Library/Preferences/.smw/servers.toml"
  done
  HOME="$WORK/cpp-home" "$WORK/cpp" > "$WORK/cpp.out" 2>&1
  HOME="$WORK/rust-home" "$RUST" > "$WORK/rust.out" 2>&1
  if ! cmp -s "$WORK/cpp-home/Library/Preferences/.smw/servers.toml" "$WORK/rust-home/Library/Preferences/.smw/servers.toml"; then
    echo "seed $seed: servers.toml differs"
    diff "$WORK/cpp-home/Library/Preferences/.smw/servers.toml" "$WORK/rust-home/Library/Preferences/.smw/servers.toml" | head -6
    fail=1
  fi
  if ! diff -q <(grep -v 'expected\|parse' "$WORK/cpp.out") <(grep -v 'expected\|parse' "$WORK/rust.out") > /dev/null; then
    echo "seed $seed: warnings differ"
    diff "$WORK/cpp.out" "$WORK/rust.out" | head -6
    fail=1
  fi
done
[ $fail -eq 0 ] && echo "all ${1:-200} cases identical"
exit $fail
