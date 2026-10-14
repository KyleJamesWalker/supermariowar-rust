#!/usr/bin/env bash
# Lists the runs under the given parity output dirs whose stdout.log does not report "[blit] SDL fallbacks: none".
# Usage: blit_fallbacks.sh <parity out dir>...   Exit status 1 if any run fell back to SDL or no run was found.
set -uo pipefail
logs=0
bad=()
while IFS= read -r -d '' log; do
    logs=$((logs + 1))
    grep -q '^\[blit\] SDL fallbacks: none' "$log" || bad+=("$log")
done < <(find "$@" -name stdout.log -print0 2>/dev/null)
if [[ $logs -eq 0 ]]; then
    echo "no stdout.log under $*"
    exit 1
fi
[[ ${#bad[@]} -eq 0 ]] && { echo "$logs runs, no SDL fallbacks"; exit 0; }
for log in "${bad[@]}"; do
    echo "$log: $(grep -h '^\[blit\] SDL fallback' "$log" | tr '\n' ' ')"
done
exit 1
