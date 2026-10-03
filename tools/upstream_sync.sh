#!/usr/bin/env bash
# Measure how far the port is from a newer upstream C++: merge upstream into the harness branch,
# build a reference from it, generate goldens into a scratch directory and run both parity suites.
# Nothing is pushed and the committed goldens are not touched. See UPSTREAM_SYNC.md.
#
# Usage: upstream_sync.sh [upstream-ref]   (default: upstream/master)
#   REF_REPO     C++ reference clone with `harness` and remote `upstream` (~/work/supermariowar-cpp-reference)
#   SYNC_DIR     scratch root (~/work/smw-upstream-sync-<short sha>): worktree, build, goldens, logs
#   JOBS         parallel game replays (default 8); editor goldens always run one at a time
# Stops after the merge if it conflicts: resolve in $SYNC_DIR/cpp, commit, then rerun.
set -euo pipefail

tools="$(cd "$(dirname "$0")" && pwd)"
port="$(dirname "$tools")"
ref_repo="${REF_REPO:-$HOME/work/supermariowar-cpp-reference}"
upstream_ref="${1:-upstream/master}"

git -C "$ref_repo" fetch upstream
target="$(git -C "$ref_repo" rev-parse --short "$upstream_ref")"
sync_dir="${SYNC_DIR:-$HOME/work/smw-upstream-sync-$target}"
cpp="$sync_dir/cpp"
build="$sync_dir/build"
mkdir -p "$sync_dir"

if [[ ! -d "$cpp" ]]; then
    git -C "$ref_repo" worktree add -b "harness-$target" "$cpp" harness
fi
if ! git -C "$cpp" merge-base --is-ancestor "$upstream_ref" HEAD; then
    if ! git -C "$cpp" merge --no-edit "$upstream_ref"; then
        echo "merge conflicts in $cpp; resolve, commit, and rerun" >&2
        exit 1
    fi
fi
git -C "$cpp" submodule update --init

base="$(git -C "$cpp" merge-base harness "$upstream_ref")"
git -C "$cpp" log --reverse --format='%h %s' "$base..$upstream_ref" > "$sync_dir/commits.txt"
git -C "$cpp" diff --stat "$base" "$upstream_ref" -- data > "$sync_dir/data-submodule.txt"
echo "$(wc -l < "$sync_dir/commits.txt") upstream commits since $(git -C "$cpp" rev-parse --short "$base"): $sync_dir/commits.txt"

cmake -S "$cpp" -B "$build" -DNO_NETWORK=ON -DBUILD_TESTS=OFF -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_CXX_FLAGS="-O2 -ffp-contract=off" > "$sync_dir/cmake.log"
cmake --build "$build" --target smw smw-leveledit smw-worldedit -j"$(sysctl -n hw.ncpu)" > "$sync_dir/build.log"

# run_ref.sh and run_editor.sh clone the data tree with `cp -R`, which copies a symlink as a link,
# so the editors' reference directory must hold a real copy or the runs write into the worktree.
edref="$sync_dir/edref"
rm -rf "$edref"
mkdir -p "$edref"
ln -s "$build" "$edref/build"
cp -c -R "$cpp/data" "$edref/data" 2>/dev/null || cp -R "$cpp/data" "$edref/data"

export SMW_BIN="$build/smw" SMW_DATA_DIR="$cpp/data"
for run in 1 2; do
    "$tools/run_ref.sh" "$tools/replays/start_classic.txt" "$sync_dir/determinism/$run" > /dev/null
done
"$tools/diffreplay.py" "$sync_dir/determinism/1/dump.txt" "$sync_dir/determinism/2/dump.txt"

goldens="$sync_dir/goldens"
GOLDEN_ROOT="$goldens" JOBS="${JOBS:-8}" "$tools/make_golden.sh"
unset SMW_BIN SMW_DATA_DIR
# The editors are only deterministic one at a time; generate twice and compare.
for g in "$goldens" "$sync_dir/goldens-check"; do
    SMW_EDITOR_REF_DIR="$edref" GOLDEN_ROOT="$g" JOBS=1 "$tools/make_editor_golden.sh" > /dev/null
done
diff -r "$goldens/editor_golden" "$sync_dir/goldens-check/editor_golden" && echo "editor goldens stable"

GOLDEN_ROOT="$goldens" PARITY_OUT="$sync_dir/out" JOBS="${JOBS:-8}" "$tools/parity.sh" > "$sync_dir/parity.log" || true
GOLDEN_ROOT="$goldens" PARITY_OUT="$sync_dir/editor-out" "$tools/editor_parity.sh" > "$sync_dir/editor_parity.log" || true
sed -n '/=== summary/,$p' "$sync_dir/parity.log" "$sync_dir/editor_parity.log"
echo "port $(git -C "$port" rev-parse --short HEAD) vs upstream $target: logs in $sync_dir"
