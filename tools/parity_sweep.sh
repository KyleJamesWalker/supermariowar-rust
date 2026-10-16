#!/usr/bin/env bash
# parity.sh over the map sweep (replays_sweep/), 4 jobs at a time by default.
# SHARD=k/n runs only every n-th replay, starting with the k-th.
tools="$(cd "$(dirname "$0")" && pwd)"
replays=("$tools"/replays_sweep/*.txt)
if [[ -n "${SHARD:-}" ]]; then
    k="${SHARD%/*}" n="${SHARD#*/}" picked=()
    for i in "${!replays[@]}"; do
        if (( i % n == k - 1 )); then picked+=("${replays[$i]}"); fi
    done
    replays=("${picked[@]}")
fi
JOBS="${JOBS:-4}" exec "$tools/parity.sh" "${replays[@]}"
