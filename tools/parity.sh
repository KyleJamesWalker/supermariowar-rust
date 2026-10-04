#!/usr/bin/env bash
# Run the Rust port on every replay and diff it against the C++ golden output.
# Usage: parity.sh [replay.txt ...]   (default: every replays/*.txt); JOBS=n runs n at once.
# Goldens: replays/<x>.txt -> golden/<x>/, replays_sweep/<x>.txt -> golden_sweep/<x>/.
# GOLDEN_ROOT=<dir> reads <dir>/golden/<x>/ instead.
# A replay with a Rust golden (golden_rust/<x>/, see make_golden.sh) is checked against it and marked so.
# Output per replay: first divergence (diffreplay.py), screenshot check, then a summary.
# SMW_NO_BUILD=1 skips cargo; SMW_BIN and SMW_DATA_DIR pass through to run_rust.sh.
# Exit status 1 if any replay diverges.
set -uo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
port="$(dirname "$tools")"
out_root="${PARITY_OUT:-${TMPDIR:-/tmp}/smw-parity}"
replays=("$@")
if [[ ${#replays[@]} -eq 0 ]]; then
    replays=("$tools"/replays/*.txt)
fi

if [[ -z "${SMW_NO_BUILD:-}" ]]; then
    cargo build --release --quiet --manifest-path "$port/Cargo.toml" || exit 2
fi

run_one() {
    local replay="$1" name
    name="$(basename "$replay" .txt)"
    SMW_NO_BUILD=1 "$tools/run_rust.sh" "$replay" "$out_root/$name" > /dev/null
    echo $? > "$out_root/$name.status"
}
export -f run_one
export tools out_root
mkdir -p "$out_root"
printf '%s\0' "${replays[@]}" | xargs -0 -n 1 -P "${JOBS:-1}" bash -c 'run_one "$0"'

summary=()
failed=0
for replay in "${replays[@]}"; do
    name="$(basename "$replay" .txt)"
    dir="$(cd "$(dirname "$replay")" && pwd)"
    golden="${GOLDEN_ROOT:-$(dirname "$dir")}/$(basename "$dir" | sed 's/^replays/golden/')/$name"
    kind=""
    if [[ -d "$tools/golden_rust/$name" ]]; then
        golden="$tools/golden_rust/$name"
        kind=" (Rust golden)"
    fi
    out="$out_root/$name"
    echo "=== $name"

    status="$(cat "$out_root/$name.status" 2>/dev/null || echo 2)"
    if [[ $status -ne 0 ]]; then
        echo "rust exited with status $status; last output:"
        tail -n 5 "$out/stdout.log" | sed 's/^/    /'
    fi

    golden_dump="$golden/dump.txt"
    [[ -f "$golden_dump" ]] || golden_dump="$golden/dump.txt.gz"
    dump_result="$("$tools/diffreplay.py" --context 1 "$golden_dump" "$out/dump.txt" 2>&1)"
    dump_status=$?
    echo "$dump_result" | head -n 40

    shot_result="$("$tools/diffreplay.py" --shots "$golden" "$out" 2>&1)"
    shot_ok="$(grep -c ': OK ' <<< "$shot_result")"
    shot_total="$(grep -c '^frame ' <<< "$shot_result")"
    grep -v ': OK ' <<< "$shot_result" | sed 's/^/shot /'

    frames="$(gzip -dcf "$golden_dump" | grep -c '^F ')"
    if [[ $dump_status -eq 0 ]]; then
        verdict="MATCH all $frames frames$kind"
    else
        first="$(sed -n 's/^DIVERGED at frame \([0-9-]*\).*/\1/p; s/^DIVERGED: .* after \([0-9]*\) frames.*/\1/p' <<< "$dump_result" | head -n 1)"
        verdict="DIVERGE at frame ${first:-?} of $frames$kind"
        failed=1
    fi
    [[ "$shot_ok" -eq "$shot_total" ]] || failed=1
    summary+=("$(printf '%-18s %-32s shots %s/%s' "$name" "$verdict" "$shot_ok" "$shot_total")")
done

echo
echo "=== summary"
printf '%s\n' "${summary[@]}"
exit "$failed"
