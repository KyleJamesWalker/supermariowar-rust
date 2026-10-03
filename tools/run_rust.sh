#!/usr/bin/env bash
# Build the Rust port and run it against a replay exactly like run_ref.sh runs the C++.
# Usage: run_rust.sh <replay.txt> [out_dir]
set -euo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
port="$(dirname "$tools")"

if [[ -z "${SMW_NO_BUILD:-}" ]]; then
    cargo build --release --quiet --manifest-path "$port/Cargo.toml"
fi

SMW_BIN="$port/target/release/smw" SMW_DATA_DIR="$port/data" \
    exec "$tools/run_ref.sh" "$@"
