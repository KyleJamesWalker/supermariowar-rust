#!/usr/bin/env bash
# Regenerate golden output from the C++ reference build: replays/<x>.txt -> golden/<x>/,
# replays_sweep/<x>.txt -> golden_sweep/<x>/.
# Usage: make_golden.sh [replay.txt ...]   (default: every replays/*.txt); JOBS=n runs n at once.
# GOLDEN_ROOT=<dir> writes <dir>/golden/<x>/ instead (e.g. goldens from another C++ build via SMW_BIN).
# Replays that test deliberate deviations from the C++ have Rust goldens instead: RUST_GOLDEN=1 writes
# golden_rust/<x>/ from the Rust build, and a C++ run skips any replay that has one.
set -euo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
replays=("$@")
if [[ ${#replays[@]} -eq 0 ]]; then
    replays=("$tools"/replays/*.txt)
fi

one() {
    local replay="$1"
    local dir name out
    dir="$(cd "$(dirname "$replay")" && pwd)"
    name="$(basename "$replay" .txt)"
    out="${GOLDEN_ROOT:-$(dirname "$dir")}/$(basename "$dir" | sed 's/^replays/golden/')/$name"
    if [[ -n "${RUST_GOLDEN:-}" ]]; then
        out="$tools/golden_rust/$name"
    elif [[ -d "$tools/golden_rust/$name" ]]; then
        echo "$name: skipped, it has a Rust golden"
        return
    fi
    rm -rf "$out"
    if [[ -n "${RUST_GOLDEN:-}" ]]; then
        SMW_NO_BUILD=1 "$tools/run_rust.sh" "$replay" "$out" > /dev/null
    else
        "$tools/run_ref.sh" "$replay" "$out" > /dev/null
    fi
    for bmp in "$out"/frame_*.bmp; do
        [[ -e "$bmp" ]] || continue
        sips -s format png "$bmp" --out "${bmp%.bmp}.png" > /dev/null
        rm "$bmp"
    done
    rm -f "$out/stdout.log"
    gzip -9 -n "$out/dump.txt"
    echo "$name: $(gzip -dc "$out/dump.txt.gz" | grep -c '^F ') frames"
}
export -f one
export tools
if [[ -n "${RUST_GOLDEN:-}" ]]; then
    cargo build --release --quiet --manifest-path "$(dirname "$tools")/Cargo.toml"
fi

printf '%s\0' "${replays[@]}" | xargs -0 -n 1 -P "${JOBS:-1}" bash -c 'one "$0"'
