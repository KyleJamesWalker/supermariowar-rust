#!/usr/bin/env bash
# Builds the debug-signed APK, as upstream's supermariowar-android build.sh did: SDL's android-project
# template with this directory's app files on top, the game as libmain.so, and data/ as APK assets.
# Needs ANDROID_HOME, ANDROID_NDK_HOME, JDK 17, cmake and cargo-ndk.
# Usage: android/build.sh [abi...]  (default arm64-v8a; also armeabi-v7a, x86_64)
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
out="$repo/target/android"
project="$out/project"
api=21
abis=("${@:-arm64-v8a}")

: "${ANDROID_HOME:?ANDROID_HOME must point at the Android SDK}"
: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at the NDK}"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml" | head -1)"
IFS=. read -r major minor patch <<< "$version"
version_code=$((major * 10000 + minor * 100 + patch))

rm -rf "$project"
mkdir -p "$project/app/libs"
for abi in "${abis[@]}"; do
    sdl="$out/sdl/$abi"
    if [[ ! -f "$sdl/lib/libSDL2_mixer.so" ]]; then
        "$repo/android/build-sdl-libs.sh" "$abi" "$sdl" "$api"
    fi
    case "$abi" in
        arm64-v8a) triple=aarch64-linux-android ;;
        armeabi-v7a) triple=armv7-linux-androideabi ;;
        x86_64) triple=x86_64-linux-android ;;
        *) echo "unsupported ABI $abi" >&2; exit 1 ;;
    esac
    triple_env="$(tr a-z- A-Z_ <<< "$triple")"
    export "CARGO_TARGET_${triple_env}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384"
    SMW_SDL_LIB_DIR="$sdl/lib" cargo ndk -t "$abi" -P "$api" \
        --manifest-path "$repo/Cargo.toml" rustc --lib --crate-type cdylib --release --locked
    mkdir -p "$project/app/libs/$abi"
    cp "$repo/target/$triple/release/libsmw.so" "$project/app/libs/$abi/libmain.so"
    cp "$sdl"/lib/libSDL2{,_image,_mixer}.so "$project/app/libs/$abi/"
done

template="$(echo "$out"/sdl/src/SDL2-*/android-project)"
cp -R "$template"/{build.gradle,gradle,gradle.properties,gradlew,settings.gradle} "$project/"
mkdir -p "$project/app/src/main"
cp -R "$template/app/src/main/java" "$template/app/src/main/res" "$project/app/src/main/"
rm -rf "$project"/app/src/main/res/mipmap-*
cp "$repo/android/build.gradle" "$project/"
cp "$repo/android/gradle/wrapper/gradle-wrapper.properties" "$project/gradle/wrapper/"
cp -R "$repo/android/app/." "$project/app/"
mkdir -p "$project/app/src/main/res/mipmap-mdpi"
cp "$repo/resources/smw.png" "$project/app/src/main/res/mipmap-mdpi/ic_launcher.png"

mkdir -p "$project/app/src/main/assets"
rsync -a --exclude .git "$repo/data/" "$project/app/src/main/assets/data/"

(cd "$project" && ./gradlew --no-daemon assembleDebug -PsmwVersionCode="$version_code" -PsmwVersionName="$version")
mkdir -p "$repo/dist"
cp "$project/app/build/outputs/apk/debug/app-debug.apk" "$repo/dist/SuperMarioWar-$version.apk"
"$repo/android/check_page_size.sh" "$repo/dist/SuperMarioWar-$version.apk"
echo "dist/SuperMarioWar-$version.apk"
