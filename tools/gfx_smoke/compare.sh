#!/usr/bin/env bash
# Renders the gfx smoke scene with the C++ originals and the Rust port and byte-compares the BMPs.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
port="$here/../.."
data="$(cd "$port/data" && pwd)"
tmp="${TMPDIR:-/tmp}/gfx_smoke"
mkdir -p "$tmp"
export SDL_VIDEODRIVER=dummy

"$here/build.sh" "$tmp/gfx_smoke_cpp"
"$tmp/gfx_smoke_cpp" "$data" "$tmp/cpp.bmp" >/dev/null
(cd "$port" && cargo run -q --example gfx_smoke -- "$data" "$tmp/rust.bmp" >/dev/null)

cmp "$tmp/cpp.bmp" "$tmp/rust.bmp"
echo "gfx smoke: identical ($tmp/cpp.bmp)"
