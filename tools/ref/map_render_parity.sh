#!/bin/bash
# Renders a sample of maps (spread across map file versions) with the C++ and the Rust port and compares BMPs.
# Usage: map_render_parity.sh [count]   (needs map_render built by map_render.sh at $SMW_MAP_RENDER)
set -e
HERE=$(cd $(dirname $0) && pwd)
PORT=$HERE/../..
DATA=${SMW_DATA_DIR:-$PORT/data}
CPP=${SMW_MAP_RENDER:-/tmp/map_render}
N=${1:-30}
OUT=$(mktemp -d)
mkdir -p $OUT/cpp $OUT/rust
MAPS=()
while IFS= read -r line; do MAPS+=("$line"); done < <(python3 - "$DATA" "$N" <<'PY'
import os, struct, sys, collections
data, n = sys.argv[1], int(sys.argv[2])
files = sorted(os.path.join(r, f) for r, _, fs in os.walk(data) for f in fs if f.endswith('.map'))
byver = collections.defaultdict(list)
for f in files:
    with open(f, 'rb') as fh:
        v = struct.unpack('<4i', fh.read(16))
    byver[v if v[0] == 1 and v[1] in (6, 7, 8) else 'v1.5'].append(f)
picked, i = [], 0
while len(picked) < n and any(i < len(l) for l in byver.values()):
    for k in sorted(byver, key=str):
        l = byver[k]
        if i * 7 < len(l) and len(picked) < n:
            picked.append(l[i * 7])
    i += 1
print('\n'.join(picked))
PY
)
SDL_VIDEODRIVER=dummy $CPP "$DATA" $OUT/cpp "${MAPS[@]}" > $OUT/cpp.log 2>&1
RUST=${SMW_RUST_RENDER:-}
if [ -z "$RUST" ]; then (cd $PORT && cargo build -q --release --example map_render); RUST=$PORT/target/release/examples/map_render; fi
SDL_VIDEODRIVER=dummy $RUST "$DATA" $OUT/rust "${MAPS[@]}" > $OUT/rust.log 2>&1
fail=0
for i in "${!MAPS[@]}"; do
  for k in f0 f40 thumb; do
    if ! cmp -s $OUT/cpp/${i}_$k.bmp $OUT/rust/${i}_$k.bmp; then echo "DIFF ${MAPS[$i]} $k"; fail=1; fi
  done
done
echo "compared ${#MAPS[@]} maps x 3 images in $OUT"
exit $fail
