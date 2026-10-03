#!/usr/bin/env bash
# parity.sh over the map sweep (replays_sweep/), 4 jobs at a time by default.
tools="$(cd "$(dirname "$0")" && pwd)"
JOBS="${JOBS:-4}" exec "$tools/parity.sh" "$tools"/replays_sweep/*.txt
