#!/bin/bash
# Persistent-file compatibility between the C++ reference and the Rust port.
# Each case seeds HOME (options.bin, controls.sdl2.bin, servers.toml), maps/cache/mapsummary.txt and filters/*.txt,
# runs both builds through persist/filter_edit.txt (edits a user filter, then StartGame writes the config and filters;
# shutdown writes servers.toml), then requires byte-identical rewritten files and identical frame dumps.
# Case "empty" starts from no files and the repository filters; seeds start from fuzzed files.
# Usage: persist_interop.sh [empty] [seed ...]   (default: empty 1 2 3 4 5)
# Env: SMW_CPP_BIN (default: the NO_NETWORK reference ~/work/supermariowar-cpp-reference/build/smw), SMW_RUST_BIN
# (default: a `--features no_network` build to match; with a networking C++ build, pass a default Rust build),
# SMW_PERSIST_FRAMES (default 730). servers.toml contents are covered by net_config_interop.sh.
HERE=$(cd "$(dirname "$0")" && pwd)
TOOLS=$(dirname "$HERE")
PORT=$(dirname "$TOOLS")
CPP=${SMW_CPP_BIN:-$HOME/work/supermariowar-cpp-reference/build/smw}
RUST=${SMW_RUST_BIN:-$PORT/target/no_network/release/smw}
FRAMES=${SMW_PERSIST_FRAMES:-730}
REPLAY=$HERE/persist/filter_edit.txt
[ -n "$SMW_RUST_BIN" ] || CARGO_TARGET_DIR="$PORT/target/no_network" cargo build --release --quiet --bin smw --features no_network \
  --manifest-path "$PORT/Cargo.toml" || exit 2
WORK=$(mktemp -d)
rsync -a --exclude maps/cache/mapsummary.txt "$PORT/data/" "$WORK/data/"
cp -R "$WORK/data/filters" "$WORK/filters-orig"
cases=("$@")
[ ${#cases[@]} -gt 0 ] || cases=(empty 1 2 3 4 5)
FILES=(options.bin controls.sdl2.bin servers.toml mapsummary.txt)
for f in "$WORK/filters-orig"/*; do FILES+=("filters/$(basename "$f")"); done

# run <bin> <input_dir> <out_dir>
run() {
  local bin=$1 in=$2 out=$3
  local home=$out/home
  mkdir -p "$home/Library/Preferences/.smw"
  for f in options.bin controls.sdl2.bin servers.toml; do
    [ -f "$in/$f" ] && cp "$in/$f" "$home/Library/Preferences/.smw/"
  done
  rm -f "$WORK/data/maps/cache/mapsummary.txt"
  [ -f "$in/mapsummary.txt" ] && cp "$in/mapsummary.txt" "$WORK/data/maps/cache/"
  rm -rf "$WORK/data/filters"
  if [ -d "$in/filters" ]; then cp -R "$in/filters" "$WORK/data/filters"; else cp -R "$WORK/filters-orig" "$WORK/data/filters"; fi
  (cd "$WORK" && env HOME="$home" SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SMW_SEED=1 SMW_NOLIMIT=1 \
    SMW_FRAMES="$FRAMES" SMW_REPLAY="$REPLAY" SMW_DUMP="$out/dump.txt" \
    "$bin" --datadir "$WORK/data" > "$out/stdout.log" 2>&1)
  local status=$?
  for f in options.bin controls.sdl2.bin servers.toml; do
    [ -f "$home/Library/Preferences/.smw/$f" ] && cp "$home/Library/Preferences/.smw/$f" "$out/"
  done
  [ -f "$WORK/data/maps/cache/mapsummary.txt" ] && cp "$WORK/data/maps/cache/mapsummary.txt" "$out/"
  cp -R "$WORK/data/filters" "$out/filters"
  return $status
}

mkdir -p "$WORK/empty"
run "$CPP" "$WORK/empty" "$WORK/defaults" || { echo "C++ defaults run failed"; exit 2; }

fail=0
for c in "${cases[@]}"; do
  in=$WORK/in-$c
  mkdir -p "$in"
  [ "$c" = empty ] || python3 "$HERE/persist_fuzz.py" "$c" "$WORK/defaults" "$in" || exit 2
  run "$CPP" "$in" "$WORK/$c-cpp"; cst=$?
  run "$RUST" "$in" "$WORK/$c-rust"; rst=$?
  line="case $c: exit cpp $cst rust $rst"
  [ $cst -eq $rst ] || fail=1
  for f in "${FILES[@]}"; do
    if [ ! -f "$WORK/$c-cpp/$f" ] && [ ! -f "$WORK/$c-rust/$f" ]; then
      line+=", $f absent"
    elif cmp -s "$WORK/$c-cpp/$f" "$WORK/$c-rust/$f"; then
      line+=", $f same"
    else
      line+=", $f DIFFERS"; fail=1
    fi
  done
  if "$TOOLS/diffreplay.py" "$WORK/$c-cpp/dump.txt" "$WORK/$c-rust/dump.txt" > "$WORK/$c-diff.txt" 2>&1; then
    line+=", dump same"
  else
    line+=", dump DIFFERS ($(head -n 1 "$WORK/$c-diff.txt"))"; fail=1
  fi
  echo "$line"
done
echo "logs in $WORK"
exit $fail
