#!/usr/bin/env bash
# Installs the debug APK on the connected device or emulator, plays replays through the harness (intent extras,
# see MainActivity) from a clean install, and diffs each dump and screenshot against the C++ golden like parity.sh.
# Usage: android/smoke.sh <apk> [replay.txt ...]   (default tools/replays/cpu_classic.txt)
set -uo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
tools="$repo/tools"
apk=$1
shift
replays=("${@:-$tools/replays/cpu_classic.txt}")
pkg=com.kylejameswalker.supermariowar
dir=/data/data/$pkg/files/smoke
out="${SMOKE_OUT:-${TMPDIR:-/tmp}/smw-android-smoke}"
timeout_s="${SMOKE_TIMEOUT:-600}"

adb install -r "$apk" || exit 2
mkdir -p "$out"
failed=0
for replay in "${replays[@]}"; do
    name="$(basename "$replay" .txt)"
    directive() { sed -n "s/^#@ *$1=\(.*\)$/\1/p" "$replay" | tail -n 1; }
    echo "=== $name"
    rm -rf "${out:?}/$name"
    mkdir -p "$out/$name"

    adb shell pm clear "$pkg" > /dev/null
    adb shell run-as "$pkg" mkdir -p "$dir/shots"
    adb exec-in run-as "$pkg" sh -c "cat > $dir/replay.txt" < "$replay"
    adb logcat -c
    extras=(--es SMW_REPLAY "$dir/replay.txt" --es SMW_DUMP "$dir/dump.txt" --es SMW_SHOT_DIR "$dir/shots" --es SMW_NOLIMIT 1)
    for key in seed frames map shots; do
        value="$(directive "$key")"
        [[ -n "$value" ]] && extras+=(--es "SMW_$(tr a-z A-Z <<< "$key")" "$value")
    done
    adb shell am start -W -n "$pkg/.MainActivity" "${extras[@]/#SMW_SHOTS/SMW_SHOT_FRAMES}" > /dev/null

    sleep 5
    elapsed=5
    while [[ -n "$(adb shell pidof "$pkg" | tr -d '\r')" ]]; do
        if ((elapsed >= timeout_s)); then
            echo "still running after ${timeout_s}s"
            adb shell am force-stop "$pkg"
            break
        fi
        sleep 5
        elapsed=$((elapsed + 5))
    done

    adb logcat -d > "$out/$name/logcat.txt"
    adb exec-out run-as "$pkg" cat "$dir/dump.txt" > "$out/$name/dump.txt"
    adb exec-out run-as "$pkg" tar -C "$dir/shots" -cf - . | tar -xf - -C "$out/$name"
    if grep -E 'FATAL EXCEPTION|Fatal signal' "$out/$name/logcat.txt" || ! grep -q 'Finished main function' "$out/$name/logcat.txt"; then
        echo "the game crashed or did not return from SDL_main"
        grep -E ' (smw|SDL|AndroidRuntime|DEBUG) ' "$out/$name/logcat.txt" | grep -v 'I smw *: loading' | tail -n 40
        failed=1
    fi
    if grep -q 'Could not open client connection port' "$out/$name/logcat.txt"; then
        echo "the network client port did not open"
        failed=1
    fi

    golden="$tools/golden/$name"
    [[ -d "$tools/golden_rust/$name" ]] && golden="$tools/golden_rust/$name"
    golden_dump="$golden/dump.txt"
    [[ -f "$golden_dump" ]] || golden_dump="$golden/dump.txt.gz"
    "$tools/diffreplay.py" --context 1 "$golden_dump" "$out/$name/dump.txt" | head -n 40
    dump_status=${PIPESTATUS[0]}
    shots="$("$tools/diffreplay.py" --shots "$golden" "$out/$name" 2>&1)"
    echo "$shots"
    if [[ $dump_status -ne 0 ]] || grep '^frame ' <<< "$shots" | grep -qv ': OK '; then
        failed=1
    fi
done
exit "$failed"
