#!/usr/bin/env bash
# Run the C++ reference build headlessly against a replay script.
#
# Usage: run_ref.sh <replay.txt|.smwrp> [out_dir]
#
# Run parameters come from `#@ key=value` lines in the replay (seed, frames,
# map, shots, options, options_b64, controls_b64); SMW_SEED/SMW_FRAMES/SMW_MAP/SMW_SHOT_FRAMES in the environment
# override them. Writes <out_dir>/dump.txt and <out_dir>/frame_<n>.bmp.
# SMW_BIN and SMW_DATA_DIR select another binary and data tree (run_rust.sh uses them).
# See docs/REPLAY.md.
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <replay.txt|.smwrp> [out_dir]" >&2
    exit 2
fi

replay="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
replay_dir="$(dirname "$replay")"
name="$(basename "$replay")"
name="${name%.smwrp}"
name="${name%.gz}"
name="${name%.txt}"
out="${2:-${TMPDIR:-/tmp}/smw-$(basename "$0" .sh)-out/$name}"
ref="${SMW_REF_DIR:-$HOME/work/supermariowar-cpp-reference}"
bin="${SMW_BIN:-$ref/build/smw}"
data="${SMW_DATA_DIR:-$ref/data}"

if [[ ! -x "$bin" ]]; then
    echo "missing $bin; build it first (see docs/REPLAY.md)" >&2
    exit 2
fi

# Per-run sandbox: a fresh HOME (no options.bin / controls file) and a private clone of the data tree,
# since the game writes data/maps/cache/mapsummary.txt and replays run concurrently.
sandbox="$(mktemp -d)"
game=""
cleanup() {
    [[ -n "$game" ]] && kill "$game" 2>/dev/null
    rm -rf "${sandbox:?}"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# The C++ harness reads plain text.
if [[ "$(head -c 2 "$replay" | od -An -tx1 | tr -d ' \n')" == 1f8b ]]; then
    gzip -dc "$replay" > "$sandbox/replay.txt"
    replay="$sandbox/replay.txt"
fi

directive() {
    sed -n "s/^#@ *$1=\(.*\)$/\1/p" "$replay" | tail -n 1
}

seed="${SMW_SEED:-$(directive seed)}"
frames="${SMW_FRAMES:-$(directive frames)}"
map="${SMW_MAP:-$(directive map)}"
shots="${SMW_SHOT_FRAMES:-$(directive shots)}"
options="$(directive options)"
options_b64="$(directive options_b64)"
controls_b64="$(directive controls_b64)"
servers_b64="$(directive servers_b64)"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
rm -f "$out/dump.txt" "$out"/frame_*.bmp

home="$sandbox/home"
settings="$home/Library/Preferences/.smw"
# Windows builds keep their settings in %USERPROFILE%/.smw/.
[[ "$OSTYPE" == msys || "$OSTYPE" == cygwin ]] && settings="$home/.smw"
mkdir -p "$(dirname "$settings")"
if [[ -n "$options" ]]; then
    mkdir -p "$settings"
    cp "$replay_dir/$options" "$settings/options.bin"
fi
# Session recordings embed the settings files they started with.
if [[ -n "$options_b64" || -n "$controls_b64" || -n "$servers_b64" ]]; then
    mkdir -p "$settings"
    [[ -z "$options_b64" ]] || printf '%s' "$options_b64" | base64 -d > "$settings/options.bin"
    [[ -z "$controls_b64" ]] || printf '%s' "$controls_b64" | base64 -d > "$settings/controls.sdl2.bin"
    [[ -z "$servers_b64" ]] || printf '%s' "$servers_b64" | base64 -d > "$settings/servers.toml"
fi
# Git Bash's cp -R does not keep listing order; on a FAT TMPDIR this gives the goldens' order.
if [[ "$OSTYPE" == msys || "$OSTYPE" == cygwin ]]; then
    python3 "$(dirname "$0")/apfs_order.py" "$data" "$sandbox/data"
else
    cp -c -R "$data" "$sandbox/data" 2>/dev/null || cp -R "$data" "$sandbox/data"
fi
rm -f "$sandbox/data/maps/cache/mapsummary.txt"
data="$sandbox/data"

cd "$sandbox"
status=0
# Background + wait so a killed script also stops the game; ulimit caps each file the game writes at 1 GB
# where the shell can set it (not Git Bash).
# The SDL_JOYSTICK_* hints hide physical pads, which would otherwise join the replay's virtual joysticks.
(ulimit -f 1048576 2> /dev/null || true; exec env HOME="$home" USERPROFILE="$home" \
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
    SDL_JOYSTICK_HIDAPI=0 SDL_JOYSTICK_MFI=0 SDL_JOYSTICK_IOKIT=0 \
    SMW_SEED="${seed:-1}" \
    SMW_NOLIMIT="${SMW_NOLIMIT-1}" \
    SMW_FRAMES="${frames:-600}" \
    SMW_MAP="$map" \
    SMW_REPLAY="$replay" \
    SMW_DUMP="$out/dump.txt" \
    SMW_SHOT_FRAMES="$shots" \
    SMW_SHOT_DIR="$out" \
    "$bin" --datadir "$data") > "$out/stdout.log" 2>&1 &
game=$!
wait "$game" || status=$?
game=""

echo "$out"
exit "$status"
