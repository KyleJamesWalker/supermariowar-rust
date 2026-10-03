#!/usr/bin/env bash
# Regenerate port/tools/golden/<script>/ from the C++ reference build.
# Usage: make_golden.sh [replay.txt ...]   (default: every replays/*.txt)
set -euo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
replays=("$@")
if [[ ${#replays[@]} -eq 0 ]]; then
    replays=("$tools"/replays/*.txt)
fi

for replay in "${replays[@]}"; do
    name="$(basename "$replay" .txt)"
    out="$tools/golden/$name"
    rm -rf "$out"
    "$tools/run_ref.sh" "$replay" "$out" > /dev/null
    for bmp in "$out"/frame_*.bmp; do
        [[ -e "$bmp" ]] || continue
        sips -s format png "$bmp" --out "${bmp%.bmp}.png" > /dev/null
        rm "$bmp"
    done
    rm -f "$out/stdout.log"
    echo "$name: $(grep -c '^F ' "$out/dump.txt") frames"
done
