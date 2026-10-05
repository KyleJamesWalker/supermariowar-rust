#!/usr/bin/env bash
# Packages the game as a muOS .muxapp: a zip whose single SuperMarioWar/ folder holds mux_launch.sh, smw,
# lib/, data/ and glyph/smw.png. muOS's Archive Manager extracts it to <storage>/MUOS/application/.
# Runs on aarch64 Linux with patchelf, binutils, rsync and zip. Builds smw first unless given a binary.
# Usage: device/muos/build-muxapp.sh [smw]   (MUXAPP_OUT overrides dist/SuperMarioWar.muxapp)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${MUXAPP_OUT:-$root/dist/SuperMarioWar.muxapp}"
mkdir -p "$(dirname "$out")"
out="$(cd "$(dirname "$out")" && pwd)/$(basename "$out")"

bin="${1:-}"
if [[ -z "$bin" ]]; then
    LIBZ_SYS_STATIC=1 cargo build --release --locked --bin smw --manifest-path "$root/Cargo.toml"
    bin="$root/target/release/smw"
fi
git -C "$root" submodule update --init data

stage="$(mktemp -d)"
trap 'rm -rf "${stage:?}"' EXIT
app="$stage/SuperMarioWar"
mkdir -p "$app/lib" "$app/glyph"
cp "$here/mux_launch.sh" "$app/mux_launch.sh"
cp "$here/glyph/smw.png" "$app/glyph/smw.png"
cp "$bin" "$app/smw"
chmod +x "$app/mux_launch.sh" "$app/smw"
rsync -a --exclude .git --exclude maps/cache/mapsummary.txt "$root/data/" "$app/data/"
patchelf --set-rpath '$ORIGIN/lib' "$app/smw"

# Device-provided: glibc, libSDL2 (it carries the GPU driver) and the GL, DRM and audio stacks beneath it.
system='^(ld-linux.*|lib(c|m|dl|rt|pthread|gcc_s|stdc\+\+|SDL2-2\.0|GL.*|EGL|GLES.*|drm|gbm|mali|asound|pulse.*)\.so(\..*)?)$'
queue=("$app/smw")
while ((${#queue[@]})); do
    file="${queue[0]}"
    queue=("${queue[@]:1}")
    while read -r name; do
        [[ "$name" =~ $system || -f "$app/lib/$name" ]] && continue
        path="$(ldd "$file" | awk -v name="$name" '$1 == name { print $3 }')"
        if [[ ! -f "$path" ]]; then
            echo "cannot resolve $name (needed by $(basename "$file"))" >&2
            exit 1
        fi
        cp -L "$path" "$app/lib/$name"
        chmod u+w "$app/lib/$name"
        patchelf --set-rpath '$ORIGIN' "$app/lib/$name"
        queue+=("$app/lib/$name")
    done < <(readelf -d "$file" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p')
done

echo "bundled libraries:"
ls -l "$app/lib"
glibc="$(find "$app/smw" "$app/lib" -type f -exec objdump -T {} + | grep -o 'GLIBC_[0-9.]*' | sort -u -V | tail -n 1)"
echo "requires $glibc or newer"

rm -f "$out"
# The game lists some data directories unsorted, and FAT keeps entries in the order they were extracted.
(cd "$stage" && find SuperMarioWar | LC_ALL=C sort | zip -q -X "$out" -@)
echo "wrote $out ($(du -h "$out" | cut -f 1))"
