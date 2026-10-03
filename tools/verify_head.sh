#!/usr/bin/env bash
# Verify committed HEAD in isolation: fresh worktree, clean release build, cargo test, then the game, sweep
# and editor parity suites against the committed goldens. SMW_MAP_DUMP (a tools/ref/map_dump.sh binary) adds
# the all-maps comparison.
# Usage: verify_head.sh [work_dir]   (default: $TMPDIR/smw-verify-<sha>; logs stay there, the worktree is removed)
# Exit status 1 if any step fails.
set -uo pipefail

repo="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
sha="$(git -C "$repo" rev-parse --short HEAD)"
work="${1:-${TMPDIR:-/tmp}/smw-verify-$sha}"
tree="$work/tree"
logs="$work/logs"

rm -rf "$work"
mkdir -p "$logs"
git -C "$repo" worktree add --detach --quiet "$tree" HEAD || exit 2
trap 'git -C "$repo" worktree remove --force "$tree"' EXIT
git -C "$tree" submodule update --init --quiet || exit 2

port="$tree"
tools="$tree/tools"
results=()
failed=0
step() {
    local name=$1; shift
    local start=$SECONDS
    "$@" > "$logs/$name.log" 2>&1
    local status=$?
    [[ $status -eq 0 ]] || failed=1
    results+=("$(printf '%-8s %-5s %4ss  %s' "$name" "$([[ $status -eq 0 ]] && echo ok || echo FAIL)" $((SECONDS - start)) "$logs/$name.log")")
    return $status
}

step build cargo build --release --manifest-path "$port/Cargo.toml"
step test cargo test --release --manifest-path "$port/Cargo.toml"
step parity env PARITY_OUT="$work/parity" JOBS="${JOBS:-8}" "$tools/parity.sh"
step sweep env PARITY_OUT="$work/sweep" JOBS="${JOBS:-8}" "$tools/parity_sweep.sh"
step editor env PARITY_OUT="$work/editor" "$tools/editor_parity.sh"
if [[ -n "${SMW_MAP_DUMP:-}" ]]; then
    step mapdump cargo test --release --manifest-path "$port/Cargo.toml" --lib matches_cpp_map_dump
fi

echo "=== verify_head $sha ($(git -C "$repo" log -1 --format=%s "$sha"))"
printf '%s\n' "${results[@]}"
grep -h '^test result' "$logs/test.log" 2>/dev/null | sed 's/^/  /'
for suite in parity sweep editor; do
    summary="$(sed -n '/^=== summary/,$p' "$logs/$suite.log" 2>/dev/null | tail -n +2)"
    echo "  $suite: $(grep -c -E 'MATCH all|dump MATCH' <<< "$summary")/$(grep -c . <<< "$summary") match"
    awk '{ ok = /MATCH/; for (i = 1; i <= NF; i++) if ($i ~ /^[0-9]+\/[0-9]+,?$/) { split($i, n, /[\/,]/); if (n[1] != n[2]) ok = 0 } } !ok { print "    " $0 }' <<< "$summary"
done
exit "$failed"
