#!/usr/bin/env bash
# Build the web version (wasm32-unknown-emscripten) and assemble dist/web/:
# index.html, smw.js, smw.wasm, smw.data (the preloaded data/ tree), the page's touch controls, manifest and icons,
# and manual.html with its images (docs/guides/GAME_MANUAL.md rendered by tools/render_manual.py).
# Usage: tools/package_web.sh            (needs emsdk 5.0.2 in $EMSDK or ~/work/emsdk)
# SMW_RELAY_URL=wss://... sets the netplay relay the page uses when it has no ?relay= parameter.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
emsdk="${EMSDK:-$HOME/work/emsdk}"
# shellcheck disable=SC1091
source "$emsdk/emsdk_env.sh" > /dev/null 2>&1
. "$repo/tools/cargo_target.sh"
target_dir="$(cargo_target)"
out="$repo/dist/web"

# The page is published, so keep build-machine paths out of it: dependency panic locations are
# remapped, and the data package name the file packager derives from the output path is reset.
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
RUSTFLAGS="--remap-path-prefix=$cargo_home=cargo --remap-path-prefix=$repo=." \
    cargo build --release --target wasm32-unknown-emscripten --bin smw --features smw/gl \
    --manifest-path "$repo/Cargo.toml" --target-dir "$target_dir"

build="$target_dir/wasm32-unknown-emscripten/release"
rm -rf "$out"
mkdir -p "$out"
# The file packager names the data file by its absolute path; where the linker writes it varies across Rust versions.
data="$(grep -o 'PACKAGE_NAME="[^"]*/smw\.data"' "$build/smw.js" | head -1 | cut -d'"' -f2)"
cp "$build/smw.wasm" "$data" "$out/"
sed "s|\"[^\"]*/smw\.data\"|\"smw.data\"|g" "$build/smw.js" > "$out/smw.js"
if grep -qE "$HOME|$repo|$target_dir" "$out/smw.js" "$out/smw.wasm"; then
    echo "error: a build path leaked into dist/web" >&2
    exit 1
fi

# Upstream ships emcc's default shell page; web/shell.html is our own (no Emscripten branding, log hidden).
# Let emcc render it for a stub with the same settings and keep only the page.
stub="$(mktemp -d)"
trap 'rm -rf "$stub"' EXIT
echo 'int main(void) { return 0; }' > "$stub/smw.c"
emcc "$stub/smw.c" -o "$stub/smw.html" -sUSE_SDL=2 -sALLOW_MEMORY_GROWTH=1 --shell-file "$repo/web/shell.html"
cp "$stub/smw.html" "$out/index.html"
cp "$repo/resources/smw.png" "$out/favicon.png"
cp "$repo"/web/{touch.js,textinput.js,touch.css,manifest.webmanifest,icon-180.png,icon-192.png,icon-512.png} "$out/"
python3 "$repo/tools/render_manual.py" "$repo/docs/guides/GAME_MANUAL.md" "$repo/web/manual.html" "$out"

du -sh "$out"/*
