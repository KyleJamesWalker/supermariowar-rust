#!/usr/bin/env bash
# Run the C++ reference build headlessly against a replay script.
#
# Usage: run_ref.sh <replay.txt> [out_dir]
#
# Run parameters come from `#@ key=value` lines in the replay (seed, frames,
# map, shots); SMW_SEED/SMW_FRAMES/SMW_MAP/SMW_SHOT_FRAMES in the environment
# override them. Writes <out_dir>/dump.txt and <out_dir>/frame_<n>.bmp.
# See port/REPLAY.md.
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <replay.txt> [out_dir]" >&2
    exit 2
fi

replay="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
name="$(basename "$replay" .txt)"
out="${2:-${TMPDIR:-/tmp}/smw-ref-out/$name}"
ref="${SMW_REF_DIR:-$HOME/work/smw-ref}"
bin="$ref/build/smw"

if [[ ! -x "$bin" ]]; then
    echo "missing $bin; build it first (see port/REPLAY.md)" >&2
    exit 2
fi

directive() {
    sed -n "s/^#@ *$1=\(.*\)$/\1/p" "$replay" | tail -n 1
}

seed="${SMW_SEED:-$(directive seed)}"
frames="${SMW_FRAMES:-$(directive frames)}"
map="${SMW_MAP:-$(directive map)}"
shots="${SMW_SHOT_FRAMES:-$(directive shots)}"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
rm -f "$out/dump.txt" "$out"/frame_*.bmp

# Fresh HOME so no options.bin / controls file from a real install leaks in.
home="$(mktemp -d)"
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/Library/Preferences"

# The game writes this cache on exit; start every run from the pristine data tree.
rm -f "$ref/data/maps/cache/mapsummary.txt"

cd "$ref"
env HOME="$home" \
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
    SMW_SEED="${seed:-1}" \
    SMW_NOLIMIT="${SMW_NOLIMIT-1}" \
    SMW_FRAMES="${frames:-600}" \
    SMW_MAP="$map" \
    SMW_REPLAY="$replay" \
    SMW_DUMP="$out/dump.txt" \
    SMW_SHOT_FRAMES="$shots" \
    SMW_SHOT_DIR="$out" \
    "$bin" --datadir "$ref/data" > "$out/stdout.log" 2>&1

echo "$out"
