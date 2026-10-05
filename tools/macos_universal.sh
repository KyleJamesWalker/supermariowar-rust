#!/usr/bin/env bash
# Merges an arm64 and an x86_64 "Super Mario War.app" (tools/package_macos.sh on each) into a universal app:
# every Mach-O file both have is joined with lipo, the rest is copied as is, and the result is ad-hoc signed again.
# Usage: tools/macos_universal.sh <arm64.app> <x86_64.app> <out.app>
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 <arm64.app> <x86_64.app> <out.app>" >&2
    exit 2
fi
arm="$1"
intel="$2"
out="$3"

rm -rf "$out"
cp -R "$arm" "$out"
while IFS= read -r -d '' f; do
    rel="${f#"$intel"/}"
    if [[ ! -e "$out/$rel" ]]; then
        mkdir -p "$(dirname "$out/$rel")"
        cp "$f" "$out/$rel"
    elif file -b "$f" | grep -q '^Mach-O'; then
        lipo -create "$arm/$rel" "$f" -output "$out/$rel"
    fi
done < <(find "$intel" -type f -print0)

codesign --force --deep --sign - "$out"
for f in "$out/Contents/MacOS/smw" "$out/Contents/Frameworks"/*.dylib; do
    printf '%-28s %s\n' "$(basename "$f")" "$(lipo -archs "$f")"
done
