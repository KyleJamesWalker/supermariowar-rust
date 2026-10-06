#!/usr/bin/env bash
# Run the Rust editors on every editor script and diff against the C++ golden output:
# per-frame dump, screenshots, and every saved file (byte for byte).
# Usage: editor_parity.sh [editor_replays/<editor>/<x>.txt ...]   (default: all)
# GOLDEN_ROOT=<dir> reads <dir>/editor_golden/ instead.
# Exit status 1 if anything differs.
set -uo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
port="$(dirname "$tools")"
out_root="${PARITY_OUT:-${TMPDIR:-/tmp}/smw-editor-parity}"
scripts=("$@")
if [[ ${#scripts[@]} -eq 0 ]]; then
    scripts=("$tools"/editor_replays/*/*.txt)
fi

cargo build --release --quiet --manifest-path "$port/Cargo.toml" || exit 2

# PNG encoders differ between SDL_image builds; the pixels must not.
same_file() {
    cmp -s "$1" "$2" || { [[ $1 == *.png && -f $2 ]] && "$tools/diffreplay.py" --image "$1" "$2" > /dev/null 2>&1; }
}

summary=()
failed=0
for script in "${scripts[@]}"; do
    editor="$(basename "$(dirname "$script")")"
    name="$(basename "$script" .txt)"
    golden="${GOLDEN_ROOT:-$tools}/editor_golden/$editor/$name"
    out="$out_root/$editor/$name"
    echo "=== $editor/$name"

    SMW_BIN="$port/target/release/${editor%it}itor" SMW_DATA_DIR="$port/data" \
        "$tools/run_editor.sh" "$editor" "$script" "$out" > /dev/null
    status=$?
    if [[ $status -ne 0 ]]; then
        echo "rust exited with status $status; last output:"
        tail -n 5 "$out/stdout.log" | sed 's/^/    /'
    fi

    "$tools/diffreplay.py" --context 1 "$golden/dump.txt.gz" "$out/dump.txt" 2>&1 | head -n 40
    dump_status=${PIPESTATUS[0]}

    shot_result="$("$tools/diffreplay.py" --shots "$golden" "$out" 2>&1)"
    shot_ok="$(grep -c ': OK ' <<< "$shot_result")"
    shot_total="$(grep -c '^frame ' <<< "$shot_result")"
    grep -v ': OK ' <<< "$shot_result" | sed 's/^/shot /'

    file_bad=0
    file_total=0
    while IFS= read -r -d '' f; do
        rel="${f#"$golden"/}"
        file_total=$((file_total + 1))
        if ! same_file "$f" "$out/$rel"; then
            echo "file $rel differs"
            file_bad=$((file_bad + 1))
        fi
    done < <(find "$golden/files" "$golden/home" -type f -print0 2>/dev/null)

    verdict="dump $([[ $dump_status -eq 0 ]] && echo MATCH || echo DIVERGE), shots $shot_ok/$shot_total, files $((file_total - file_bad))/$file_total"
    if [[ $dump_status -ne 0 || $shot_ok -ne $shot_total || $file_bad -ne 0 || $status -ne 0 ]]; then
        failed=1
    fi
    summary+=("$editor/$name: $verdict")
done

echo
echo "=== summary"
printf '%s\n' "${summary[@]}"
exit $failed
