#!/usr/bin/env bash
# Runs each replay script as a live, recorded session (SMW_LIVE_SCRIPT), replays the recording it made, and
# fails unless the two dumps match: live input capture must record exactly what the game saw.
# Usage: live_roundtrip.sh <script.txt>...   SMW_BIN and SMW_DATA_DIR as for run_rust.sh; SMW_NO_BUILD=1 skips cargo.
set -uo pipefail
tools="$(cd "$(dirname "$0")" && pwd)"
port="$(dirname "$tools")"
[[ -n "${SMW_NO_BUILD:-}" ]] || cargo build --release --quiet --locked --manifest-path "$port/Cargo.toml" --bin smw || exit 2
bin="${SMW_BIN:-$port/target/release/smw}"
data="${SMW_DATA_DIR:-$port/data}"

fail=0
for script in "$@"; do
    name="$(basename "$script" .txt)"
    script="$(cd "$(dirname "$script")" && pwd)/$(basename "$script")"
    sandbox="$(mktemp -d)"
    directive() { sed -n "s/^#@ *$1=\(.*\)$/\1/p" "$script" | tail -n 1; }
    home="$sandbox/home"
    mkdir -p "$home/Library/Preferences"
    cp -c -R "$data" "$sandbox/data" 2>/dev/null || cp -R "$data" "$sandbox/data"
    rm -f "$sandbox/data/maps/cache/mapsummary.txt"
    (cd "$sandbox" && env HOME="$home" USERPROFILE="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
        SDL_JOYSTICK_HIDAPI=0 SDL_JOYSTICK_MFI=0 SDL_JOYSTICK_IOKIT=0 \
        SMW_SEED="$(directive seed)" SMW_NOLIMIT=1 SMW_FRAMES="$(directive frames)" SMW_MAP="$(directive map)" \
        SMW_LIVE_SCRIPT="$script" SMW_DUMP="$sandbox/live.dump" \
        "$bin" --datadir "$sandbox/data") > "$sandbox/live.log" 2>&1
    recording="$(find "$home" -path '*replays/*.txt' | head -n 1)"
    if [[ -z "$recording" ]]; then
        echo "$name: FAIL, the live session wrote no recording (log: $sandbox/live.log)"
        fail=1
        continue
    fi
    SMW_NO_BUILD=1 SMW_BIN="$bin" SMW_DATA_DIR="$data" "$tools/run_rust.sh" "$recording" "$sandbox/replay" > /dev/null 2>&1
    if cmp -s "$sandbox/live.dump" "$sandbox/replay/dump.txt"; then
        echo "$name: MATCH ($(grep -c '^F ' "$sandbox/live.dump") frames)"
        rm -rf "$sandbox"
    else
        echo "$name: DIVERGE (kept in $sandbox)"
        "$tools/diffreplay.py" --context 1 "$sandbox/live.dump" "$sandbox/replay/dump.txt" 2>&1 | head -n 20
        fail=1
    fi
done
exit $fail
