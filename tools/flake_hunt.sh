#!/usr/bin/env bash
# Hunts browser netplay flakes: runs tools/web_netplay_test.mjs in a container under CPU and memory limits,
# optional CPU contention and loopback jitter, repeated per limit. On demand only; CI does not run it.
# Usage: tools/flake_hunt.sh (--web <dir> | --run <CI run id>) [options]
#   --web <dir>        web build to test (dist/web, or an unzipped smw-web artifact)
#   --run <id>         download that CI run's smw-web artifact instead
#   --games "<names>"  scenarios (default: net_game_stomp net_game_blocks)
#   --runs <n>         runs per CPU limit (default: 10)
#   --cpus "<list>"    CPU limits to sweep (default: "4 2 1")
#   --memory <size>    memory limit (default: 4g)
#   --stress <n>       stress-ng CPU workers in the container (default: 0)
#   --jitter "<d j>"   loopback netem delay and jitter, e.g. "20ms 10ms" (default: none)
#   --out <dir>        results (default: ~/.cache/smw-flake-hunt); passing runs' files are removed
# Needs Docker. The first run builds the image and smw_relay; later runs reuse both. Colima shares only $HOME with
# its VM, so the web build is copied into <out> and <out> must be under $HOME.
set -uo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
web="" run="" games="net_game_stomp net_game_blocks" runs=10 cpus="4 2 1" memory=4g stress=0 jitter=""
out="${XDG_CACHE_HOME:-$HOME/.cache}/smw-flake-hunt"
while [[ $# -gt 0 ]]; do
    case $1 in
        --web) web=$2 ;;
        --run) run=$2 ;;
        --games) games=$2 ;;
        --runs) runs=$2 ;;
        --cpus) cpus=$2 ;;
        --memory) memory=$2 ;;
        --stress) stress=$2 ;;
        --jitter) jitter=$2 ;;
        --out) out=$2 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
    shift 2
done
mkdir -p "$out"

if [[ -n "$run" ]]; then
    rm -rf "$out/artifact" && mkdir -p "$out/artifact"
    gh run download "$run" -n smw-web -D "$out/artifact" || exit 2
    (cd "$out/artifact" && unzip -q ./*.zip) || exit 2
    web="$(dirname "$(find "$out/artifact" -name index.html | head -n 1)")"
fi
[[ -f "$web/index.html" ]] || { echo "no web build at '$web'" >&2; exit 2; }
out="$(cd "$out" && pwd)"
if [[ "$(cd "$web" && pwd)" != "$out/web" ]]; then
    rm -rf "$out/web" && cp -R "$web" "$out/web" || exit 2
fi
web="$out/web"

docker build -q -t smw-flake-hunt "$repo/tools/flake_hunt" > /dev/null || exit 2

summary="$out/summary.txt"
: > "$summary"
for cpu in $cpus; do
    for i in $(seq "$runs"); do
        dir="$out/cpus$cpu-run$i"
        rm -rf "$dir" && mkdir -p "$dir"
        caps=()
        [[ -n "$jitter" ]] && caps=(--cap-add NET_ADMIN)
        docker run --rm --cpus "$cpu" --memory "$memory" ${caps[@]+"${caps[@]}"} \
            -v "$repo:/repo:ro" -v "$web:/web:ro" -v "$dir:/out" -v smw-flake-hunt-cargo:/cache \
            -e GAMES="$games" -e STRESS="$stress" -e JITTER="$jitter" \
            smw-flake-hunt flake-inside > "$dir/test.log" 2>&1
        for game in $games; do
            result="$(grep -oE "^$game: (PASS|FAIL)" "$dir/test.log" | awk '{print $2}')"
            over="$(sed -n "/^== $game\$/,/^$game: /s/.*game over at gameplay frame: host \([0-9]*\).*/\1/p" "$dir/test.log")"
            line="cpus=$cpu run=$i $game ${result:-ERROR} gameover=${over:--}"
            echo "$line" | tee -a "$summary"
        done
        grep -qE ': (FAIL|ERROR)|^FAIL' "$dir/test.log" || ! grep -q ': PASS' "$dir/test.log" || rm -rf "$dir"
    done
done

echo
echo "per CPU limit and scenario (pass/total):"
awk '{split($1, c, "="); key = c[2] " " $3; total[key]++; if ($4 == "PASS") pass[key]++}
     END {for (k in total) printf "  cpus=%s: %d/%d\n", k, pass[k], total[k]}' "$summary" | sort
echo "failing runs kept in $out"
