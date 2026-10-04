#!/usr/bin/env bash
# Builds SDL2_image (PNG/JPG via stb_image) and SDL2_mixer (WAV, OGG via stb_vorbis) with no external
# codec libraries, and installs them into PREFIX (default /usr/local, using sudo when not writable).
# Needs libsdl2-dev, cmake and a C compiler. Usage: device/muos/build-sdl-libs.sh [prefix]
set -euo pipefail

prefix="${1:-/usr/local}"
image_version=2.8.12
image_sha256=393f5efb50536ec13ca4f4affb69cc9966d3c3f969e6c5e701faddf9f9785381
mixer_version=2.8.2
mixer_sha256=938dff531d00ace2296557a6599abe6f34599e2f34f0a4a08a397e2ccac8b8f7

work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

fetch() {
    local repo=$1 name=$2 version=$3 sha256=$4
    curl -fsSL -o "$work/$name.tar.gz" \
        "https://github.com/libsdl-org/$repo/releases/download/release-$version/$name-$version.tar.gz"
    echo "$sha256  $work/$name.tar.gz" | sha256sum -c -
    tar -xzf "$work/$name.tar.gz" -C "$work"
}

build() {
    local src=$1
    shift
    cmake -S "$src" -B "$src/build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$prefix" "$@"
    cmake --build "$src/build" --parallel
    if [[ -w "$prefix" ]]; then
        cmake --install "$src/build"
    else
        sudo cmake --install "$src/build"
    fi
}

fetch SDL_image SDL2_image "$image_version" "$image_sha256"
build "$work/SDL2_image-$image_version" \
    -DSDL2IMAGE_VENDORED=OFF -DSDL2IMAGE_BACKEND_STB=ON -DSDL2IMAGE_SAMPLES=OFF \
    -DSDL2IMAGE_AVIF=OFF -DSDL2IMAGE_JXL=OFF -DSDL2IMAGE_SVG=OFF -DSDL2IMAGE_TIF=OFF -DSDL2IMAGE_WEBP=OFF

fetch SDL_mixer SDL2_mixer "$mixer_version" "$mixer_sha256"
build "$work/SDL2_mixer-$mixer_version" \
    -DSDL2MIXER_VENDORED=OFF -DSDL2MIXER_SAMPLES=OFF -DSDL2MIXER_VORBIS=STB \
    -DSDL2MIXER_FLAC=OFF -DSDL2MIXER_GME=OFF -DSDL2MIXER_MOD=OFF -DSDL2MIXER_MP3=OFF \
    -DSDL2MIXER_MIDI=OFF -DSDL2MIXER_OPUS=OFF -DSDL2MIXER_WAVPACK=OFF

if [[ -w /etc/ld.so.cache ]]; then ldconfig; elif command -v sudo > /dev/null; then sudo ldconfig; fi
