#!/usr/bin/env bash
# Regenerate editor golden output from the C++ reference build:
# editor_replays/<editor>/<x>.txt -> editor_golden/<editor>/<x>/ (dump.txt.gz, frame_<n>.png, files/, home/).
# Usage: make_editor_golden.sh [editor_replays/<editor>/<x>.txt ...]   (default: all); JOBS=n runs n at once.
set -euo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
scripts=("$@")
if [[ ${#scripts[@]} -eq 0 ]]; then
    scripts=("$tools"/editor_replays/*/*.txt)
fi

one() {
    local script="$1"
    local editor name out
    editor="$(basename "$(dirname "$script")")"
    name="$(basename "$script" .txt)"
    out="$tools/editor_golden/$editor/$name"
    rm -rf "$out"
    "$tools/run_editor.sh" "$editor" "$script" "$out" > /dev/null
    for bmp in "$out"/frame_*.bmp; do
        [[ -e "$bmp" ]] || continue
        sips -s format png "$bmp" --out "${bmp%.bmp}.png" > /dev/null
        rm "$bmp"
    done
    rm -f "$out/stdout.log"
    gzip -9 -n "$out/dump.txt"
    echo "$editor/$name: $(gzip -dc "$out/dump.txt.gz" | grep -c '^F ') frames"
}
export -f one
export tools

printf '%s\0' "${scripts[@]}" | xargs -0 -n 1 -P "${JOBS:-1}" bash -c 'one "$0"'
