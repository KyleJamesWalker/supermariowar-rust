#!/usr/bin/env bash
# Lists the runs under the given parity output dirs whose stdout.log reports a blit the software blitter is not
# verified against SDL for ("[blit] unverified blit").
# Usage: blit_unverified.sh <parity out dir>...   Exit status 1 if any run reported one or no run was found.
set -uo pipefail
logs=0
bad=()
while IFS= read -r -d '' log; do
    logs=$((logs + 1))
    grep -q '^\[blit\] unverified blit' "$log" && bad+=("$log")
done < <(find "$@" -name stdout.log -print0 2>/dev/null)
if [[ $logs -eq 0 ]]; then
    echo "no stdout.log under $*"
    exit 1
fi
[[ ${#bad[@]} -eq 0 ]] && { echo "$logs runs, no unverified blits"; exit 0; }
for log in "${bad[@]}"; do
    echo "$log: $(grep -h '^\[blit\] unverified blit' "$log" | head -1)"
done
exit 1
