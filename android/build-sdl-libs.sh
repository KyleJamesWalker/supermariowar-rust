#!/usr/bin/env bash
# Builds SDL2, SDL2_image (PNG via stb_image) and SDL2_mixer (WAV, OGG via stb_vorbis) as shared
# libraries for one Android ABI, and installs them into PREFIX. Needs ANDROID_NDK_HOME and cmake.
# Usage: android/build-sdl-libs.sh <abi> <prefix> [api]
set -euo pipefail

abi=$1
prefix=$2
api=${3:-21}
sdl_version=2.32.10
sdl_sha256=5f5993c530f084535c65a6879e9b26ad441169b3e25d789d83287040a9ca5165
image_version=2.8.12
image_sha256=393f5efb50536ec13ca4f4affb69cc9966d3c3f969e6c5e701faddf9f9785381
mixer_version=2.8.2
mixer_sha256=938dff531d00ace2296557a6599abe6f34599e2f34f0a4a08a397e2ccac8b8f7

: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at the NDK}"
src="${SDL_SRC_DIR:-$(dirname "$prefix")/src}"
mkdir -p "$src"
work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

fetch() {
    local repo=$1 name=$2 version=$3 sha256=$4
    [[ -f "$src/$name-$version/CMakeLists.txt" ]] && return
    curl -fsSL -o "$work/$name.tar.gz" \
        "https://github.com/libsdl-org/$repo/releases/download/release-$version/$name-$version.tar.gz"
    echo "$sha256  $work/$name.tar.gz" | shasum -a 256 -c -
    tar -xzf "$work/$name.tar.gz" -C "$src"
}

build() {
    local dir=$1
    shift
    cmake -S "$dir" -B "$work/$(basename "$dir")" \
        -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
        -DANDROID_ABI="$abi" -DANDROID_PLATFORM="android-$api" -DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$prefix" \
        -DCMAKE_FIND_ROOT_PATH="$prefix" -DCMAKE_PREFIX_PATH="$prefix" "$@"
    cmake --build "$work/$(basename "$dir")" --parallel
    cmake --install "$work/$(basename "$dir")"
}

fetch SDL SDL2 "$sdl_version" "$sdl_sha256"
fetch SDL_image SDL2_image "$image_version" "$image_sha256"
fetch SDL_mixer SDL2_mixer "$mixer_version" "$mixer_sha256"

build "$src/SDL2-$sdl_version" -DSDL_SHARED=ON -DSDL_STATIC=OFF -DSDL_TEST=OFF

build "$src/SDL2_image-$image_version" -DSDL2_DIR="$prefix/lib/cmake/SDL2" \
    -DSDL2IMAGE_VENDORED=OFF -DSDL2IMAGE_BACKEND_STB=ON -DSDL2IMAGE_SAMPLES=OFF \
    -DSDL2IMAGE_AVIF=OFF -DSDL2IMAGE_JXL=OFF -DSDL2IMAGE_SVG=OFF -DSDL2IMAGE_TIF=OFF -DSDL2IMAGE_WEBP=OFF

build "$src/SDL2_mixer-$mixer_version" -DSDL2_DIR="$prefix/lib/cmake/SDL2" \
    -DSDL2MIXER_VENDORED=OFF -DSDL2MIXER_SAMPLES=OFF -DSDL2MIXER_VORBIS=STB \
    -DSDL2MIXER_FLAC=OFF -DSDL2MIXER_GME=OFF -DSDL2MIXER_MOD=OFF -DSDL2MIXER_MP3=OFF \
    -DSDL2MIXER_MIDI=OFF -DSDL2MIXER_OPUS=OFF -DSDL2MIXER_WAVPACK=OFF
