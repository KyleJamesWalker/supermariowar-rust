#!/usr/bin/env bash
# Builds dist/SuperMarioWar-linux.tar.gz: one SuperMarioWar/ folder with smw, leveleditor, worldeditor, smw_server,
# data/, README.md and CREDITS. The binaries link the system SDL2, SDL2_image and SDL2_mixer, like upstream's Linux build.
# Usage: tools/package_linux.sh   (PACKAGE_OUT overrides the tarball path)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${PACKAGE_OUT:-$root/dist/SuperMarioWar-linux.tar.gz}"
mkdir -p "$(dirname "$out")"
out="$(cd "$(dirname "$out")" && pwd)/$(basename "$out")"

cargo build --release --locked --bins --manifest-path "$root/Cargo.toml"
git -C "$root" submodule update --init data

stage="$(mktemp -d)"
trap 'rm -rf "${stage:?}"' EXIT
app="$stage/SuperMarioWar"
mkdir -p "$app"
for bin in smw leveleditor worldeditor smw_server; do
    cp "$root/target/release/$bin" "$app/$bin"
    strip "$app/$bin"
done
rsync -a --exclude .git --exclude maps/cache/mapsummary.txt "$root/data/" "$app/data/"
cp "$root/README.md" "$root/CREDITS" "$app/"

echo "shared libraries:"
readelf -d "$app/smw" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/    \1/p'
glibc="$(objdump -T "$app"/smw "$app"/leveleditor "$app"/worldeditor "$app"/smw_server | grep -o 'GLIBC_[0-9.]*' | sort -u -V | tail -n 1)"
echo "requires $glibc or newer"

# The game lists some data directories unsorted; extracting onto FAT keeps the archive's order.
tar --sort=name --owner=0 --group=0 --numeric-owner -C "$stage" -czf "$out" SuperMarioWar
echo "wrote $out ($(du -h "$out" | cut -f 1))"
