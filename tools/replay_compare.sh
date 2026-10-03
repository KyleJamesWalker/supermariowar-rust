#!/usr/bin/env bash
# Replay one recording (or any replay script) headless on the C++ reference and on the Rust build,
# report the first divergent frame with context, and save screenshots around it.
#
# Usage: replay_compare.sh <recording.txt> [out_dir]
# Env: SMW_CPP_BIN (default ~/work/supermariowar-cpp-reference/build-latest/smw),
#      SMW_CPP_DATA (default ~/work/supermariowar-cpp-reference/data), SMW_NO_BUILD=1 to skip cargo.
# Exit status: 0 identical, 1 diverged, 2 error. See REPLAY.md, "Recordings".
set -uo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <recording.txt> [out_dir]" >&2
    exit 2
fi

tools="$(cd "$(dirname "$0")" && pwd)"
replay="$1"
out="${2:-${TMPDIR:-/tmp}/smw-replay-compare/$(basename "$replay" .txt)}"
cpp_bin="${SMW_CPP_BIN:-$HOME/work/supermariowar-cpp-reference/build-latest/smw}"
cpp_data="${SMW_CPP_DATA:-$HOME/work/supermariowar-cpp-reference/data}"
rm -rf "$out"
mkdir -p "$out"

run_both() {  # run_both <shot frames>
    SMW_SHOT_FRAMES="$1" SMW_BIN="$cpp_bin" SMW_DATA_DIR="$cpp_data" "$tools/run_ref.sh" "$replay" "$out/cpp" > /dev/null
    echo "C++ exit status $?"
    SMW_SHOT_FRAMES="$1" "$tools/run_rust.sh" "$replay" "$out/rust" > /dev/null
    echo "Rust exit status $?"
}

run_both ""
result="$("$tools/diffreplay.py" --context 3 "$out/cpp/dump.txt" "$out/rust/dump.txt")"
status=$?
echo "$result"
if [[ $status -eq 0 ]]; then
    exit 0
fi

frame="$(sed -n 's/^DIVERGED at frame \([0-9]*\).*/\1/p; s/^DIVERGED: .* after \([0-9]*\) frames.*/\1/p' <<< "$result" | head -n 1)"
if [[ -n "$frame" ]]; then
    shots="$((frame > 0 ? frame - 1 : 0)),$frame,$((frame + 1))"
    echo
    echo "Screenshots at frames $shots:"
    SMW_NO_BUILD=1 run_both "$shots" > /dev/null
    for side in cpp rust; do
        for bmp in "$out/$side"/frame_*.bmp; do
            [[ -e "$bmp" ]] || continue
            sips -s format png "$bmp" --out "${bmp%.bmp}.png" > /dev/null && rm "$bmp"
        done
    done
    "$tools/diffreplay.py" --shots "$out/cpp" "$out/rust"
    echo "C++:  $out/cpp/frame_<n>.png"
    echo "Rust: $out/rust/frame_<n>.png"
fi
exit 1
