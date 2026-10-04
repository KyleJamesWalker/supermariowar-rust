#!/usr/bin/env bash
# Build the web version (wasm32-unknown-emscripten, NO_NETWORK) and assemble dist/web/:
# index.html, smw.js, smw.wasm and smw.data (the preloaded data/ tree).
# Usage: tools/package_web.sh            (needs emsdk 5.0.2 in $EMSDK or ~/work/emsdk)
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
emsdk="${EMSDK:-$HOME/work/emsdk}"
# shellcheck disable=SC1091
source "$emsdk/emsdk_env.sh" > /dev/null 2>&1
target_dir="${CARGO_TARGET_DIR:-$repo/target}"
out="$repo/dist/web"

# The page is published, so keep build-machine paths out of it: dependency panic locations are
# remapped, and the data package name the file packager derives from the output path is reset.
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
RUSTFLAGS="--remap-path-prefix=$cargo_home=cargo --remap-path-prefix=$repo=." \
    cargo build --release --target wasm32-unknown-emscripten --features no_network --bin smw \
    --manifest-path "$repo/Cargo.toml" --target-dir "$target_dir"

build="$target_dir/wasm32-unknown-emscripten/release"
rm -rf "$out"
mkdir -p "$out"
cp "$build/smw.wasm" "$build/deps/smw.data" "$out/"
sed "s|\"[^\"]*/deps/smw\.data\"|\"smw.data\"|g" "$build/smw.js" > "$out/smw.js"
if grep -qE "$HOME|$repo|$target_dir" "$out/smw.js" "$out/smw.wasm"; then
    echo "error: a build path leaked into dist/web" >&2
    exit 1
fi

# Upstream links smw.html (CMAKE_EXECUTABLE_SUFFIX .html), i.e. emcc's default shell page.
# Let emcc render that shell for a stub with the same settings and keep only the page.
stub="$(mktemp -d)"
trap 'rm -rf "$stub"' EXIT
echo 'int main(void) { return 0; }' > "$stub/smw.c"
emcc "$stub/smw.c" -o "$stub/smw.html" -sUSE_SDL=2 -sALLOW_MEMORY_GROWTH=1
cp "$stub/smw.html" "$out/index.html"

du -sh "$out"/*
