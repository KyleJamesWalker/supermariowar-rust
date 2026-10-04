#!/usr/bin/env bash
# Run an editor (C++ reference by default) headlessly against an editor replay script.
#
# Usage: run_editor.sh <worldedit|leveledit> <script.txt> [out_dir]
#
# Run parameters come from `#@ key=value` lines in the script (seed, frames, shots,
# files); SMW_SEED/SMW_FRAMES/SMW_SHOT_FRAMES in the environment override them.
# The editor runs on a private clone of the data tree, so saves never touch the
# reference data. Writes <out_dir>/dump.txt, frame_<n>.bmp, stdout.log, and copies
# every path (relative to the run directory, which holds data/) listed in
# `#@ files=a,b` to <out_dir>/files/<path>
# (plus home/<name> for every name in `#@ homefiles=`).
# SMW_BIN and SMW_DATA_DIR select another binary and data tree (Rust runs use them).
# See docs/EDITOR_REPLAY.md.
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <worldedit|leveledit> <script.txt> [out_dir]" >&2
    exit 2
fi

editor="$1"
script="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
name="$(basename "$script" .txt)"
out="${3:-${TMPDIR:-/tmp}/smw-editor-out/$editor/$name}"
ref="${SMW_EDITOR_REF_DIR:-$HOME/work/supermariowar-cpp-reference}"
bin="${SMW_BIN:-$ref/build/smw-$editor}"
data="${SMW_DATA_DIR:-$ref/data}"

if [[ ! -x "$bin" ]]; then
    echo "missing $bin; build it first (see docs/EDITOR_REPLAY.md)" >&2
    exit 2
fi

directive() {
    sed -n "s/^#@ *$1=\(.*\)$/\1/p" "$script" | tail -n 1
}

seed="${SMW_SEED:-$(directive seed)}"
frames="${SMW_FRAMES:-$(directive frames)}"
shots="${SMW_SHOT_FRAMES:-$(directive shots)}"
files="$(directive files)"
homefiles="$(directive homefiles)"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
rm -rf "$out/dump.txt" "$out"/frame_*.bmp "$out/files" "$out/home"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
home="$work/home"
mkdir -p "$home/Library/Preferences"
# APFS clone: instant, and writes stay private to this run.
cp -Rc "$data" "$work/data" 2>/dev/null || cp -R "$data" "$work/data"
rm -f "$work/data/maps/cache/mapsummary.txt"

cd "$work"
status=0
env HOME="$home" \
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
    SMW_SEED="${seed:-1}" \
    SMW_NOLIMIT="${SMW_NOLIMIT-1}" \
    SMW_FRAMES="${frames:-300}" \
    SMW_REPLAY="$script" \
    SMW_DUMP="$out/dump.txt" \
    SMW_SHOT_FRAMES="$shots" \
    SMW_SHOT_DIR="$out" \
    "$bin" --datadir data > "$out/stdout.log" 2>&1 || status=$?

IFS=',' read -r -a file_list <<< "$files"
for f in ${file_list[@]+"${file_list[@]}"}; do
    [[ -z "$f" ]] && continue
    if [[ -f "$work/$f" ]]; then
        mkdir -p "$out/files/$(dirname "$f")"
        cp "$work/$f" "$out/files/$f"
    else
        echo "missing output file $f" >> "$out/stdout.log"
    fi
done

IFS=',' read -r -a home_list <<< "$homefiles"
for f in ${home_list[@]+"${home_list[@]}"}; do
    [[ -z "$f" ]] && continue
    if [[ -f "$home/Library/Preferences/.smw/$f" ]]; then
        mkdir -p "$out/home"
        cp "$home/Library/Preferences/.smw/$f" "$out/home/$f"
    else
        echo "missing home file $f" >> "$out/stdout.log"
    fi
done

echo "$out"
exit "$status"
