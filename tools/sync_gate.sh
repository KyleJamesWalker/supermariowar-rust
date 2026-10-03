#!/usr/bin/env bash
# Gate a commit during an upstream sync: check it out in a private detached worktree, build, run
# cargo test, then both parity suites against the sync goldens, and compare the summaries with the
# last accepted baseline. Only committed state is tested, so edits in other worktrees cannot leak in.
#
# Usage: sync_gate.sh [rev]          (default: upstream-sync)
#        sync_gate.sh --accept [rev] (also record this run as the new baseline)
#   GOLDEN_ROOT  sync goldens (~/work/smw-upstream-goldens)
#   GATE_DIR     gate worktree (~/work/smw-upstream-gate); its target/ persists between runs
# Exit status 1 if a test fails or any replay or session is worse than the baseline.
set -uo pipefail

accept=0
if [[ "${1:-}" == "--accept" ]]; then
    accept=1
    shift
fi
tools="$(cd "$(dirname "$0")" && pwd)"
repo="$(git -C "$tools" rev-parse --show-toplevel)"
rev="$(git -C "$repo" rev-parse --short "${1:-upstream-sync}")"
goldens="${GOLDEN_ROOT:-$HOME/work/smw-upstream-goldens}"
gate="${GATE_DIR:-$HOME/work/smw-upstream-gate}"
logs="$goldens/gate"
mkdir -p "$logs"

if [[ ! -d "$gate" ]]; then
    git -C "$repo" worktree add --detach "$gate" "$rev" > /dev/null
fi
git -C "$gate" checkout -q --detach "$rev"
git -C "$gate" submodule update --init -q

log="$logs/$rev.log"
summary="$logs/$rev.summary"
{
    echo "=== cargo test"
    (cd "$gate" && cargo test --release 2>&1 | grep -E '^test result|FAILED|panicked|^error')
    echo "=== parity"
    GOLDEN_ROOT="$goldens" PARITY_OUT="$logs/out-game" JOBS="${JOBS:-8}" "$gate/tools/parity.sh"
    echo "=== editor parity"
    GOLDEN_ROOT="$goldens" PARITY_OUT="$logs/out-editor" "$gate/tools/editor_parity.sh"
} > "$log" 2>&1

failed=0
grep -q -E 'FAILED|panicked|^error|test result: FAILED' "$log" && failed=1
awk '/^=== summary/{on=1; next} /^=== /{on=0} on && NF' "$log" | sort > "$summary"
rm -rf "$logs/out-game" "$logs/out-editor"

baseline="$logs/baseline.summary"
if [[ -f "$baseline" ]]; then
    if ! diff "$baseline" "$summary" > "$logs/$rev.diff"; then
        echo "changed against the baseline ($(cat "$logs/baseline.rev")):"
        cat "$logs/$rev.diff"
        failed=1
    else
        echo "same as the baseline ($(cat "$logs/baseline.rev"))"
    fi
fi
grep -E '^test result' "$log" | awk '{p+=$4; f+=$6} END{print "cargo test: " p " passed, " f " failed"}'
echo "game: $(grep -c 'MATCH all' "$summary")/$(grep -c 'frames' "$summary") dumps match; editor: $(grep -c 'dump MATCH' "$summary")/$(grep -c 'dump ' "$summary") dumps match"
echo "log: $log"

if [[ $accept -eq 1 ]]; then
    cp "$summary" "$baseline"
    echo "$rev" > "$logs/baseline.rev"
    echo "accepted $rev as the baseline"
fi
exit $failed
