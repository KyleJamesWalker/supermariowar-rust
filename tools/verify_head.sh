#!/usr/bin/env bash
# Verify committed HEAD in isolation: fresh worktree, clean release build, cargo test, full parity suite.
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

port="$tree"
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
step parity env PARITY_OUT="$work/parity" "$tools/parity.sh"

echo "=== verify_head $sha ($(git -C "$repo" log -1 --format=%s "$sha"))"
printf '%s\n' "${results[@]}"
grep -h '^test result' "$logs/test.log" 2>/dev/null | sed 's/^/  /'
sed -n '/^=== summary/,$p' "$logs/parity.log" 2>/dev/null | tail -n +2 | sed 's/^/  /'
exit "$failed"
