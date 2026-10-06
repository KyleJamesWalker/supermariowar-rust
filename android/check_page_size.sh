#!/usr/bin/env bash
# Fails unless the APK runs on 16 KB page devices: every 64-bit native library's LOAD segments aligned to at least
# 16 KB, and the uncompressed libraries 16 KB-aligned in the zip. Needs ANDROID_HOME (build-tools 35+) and ANDROID_NDK_HOME.
# Usage: android/check_page_size.sh <apk>
set -euo pipefail

apk=$1
readelf="$(echo "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-readelf)"
zipalign="$(ls -d "$ANDROID_HOME"/build-tools/*/zipalign | sort -V | tail -n 1)"
work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

unzip -q "$apk" 'lib/*' -d "$work"
failed=0
for so in "$work"/lib/{arm64-v8a,x86_64}/*.so; do
    [[ -f "$so" ]] || continue
    for align in $("$readelf" -lW "$so" | awk '$1 == "LOAD" { print $NF }' | sort -u); do
        if (( align < 0x4000 )); then
            echo "${so#"$work"/}: LOAD alignment $align is below 16 KB"
            failed=1
        fi
    done
done
if ! "$zipalign" -c -P 16 -v 4 "$apk" > "$work/zipalign.txt"; then
    grep -v '(OK' "$work/zipalign.txt" | tail -n 20
    failed=1
fi
if unzip -v "$apk" 'lib/*' | grep -q ' Defl'; then
    echo "native libraries are compressed"
    failed=1
fi
[[ $failed -eq 0 ]] && echo "16 KB page size: OK"
exit "$failed"
