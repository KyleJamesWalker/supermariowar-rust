#!/usr/bin/env bash
# Builds "dist/Super Mario War.app": release binary, data/ in Contents/Resources (SDL's base path inside a
# bundle), and every non-system dylib copied into Contents/Frameworks with rewritten install names.
# Usage: tools/package_macos.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
app="$root/dist/Super Mario War.app"
version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)"

cargo build --release --bin smw --manifest-path "$root/Cargo.toml"
git -C "$root" submodule update --init data

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$app/Contents/Frameworks"
cp "$root/target/release/smw" "$app/Contents/MacOS/smw"
rsync -a --exclude .git --exclude maps/cache/mapsummary.txt "$root/data/" "$app/Contents/Resources/data/"

# sdl2-compat dlopen()s SDL3 as @loader_path/libSDL3.dylib, so otool never lists it.
frameworks="$app/Contents/Frameworks"
queue=("$app/Contents/MacOS/smw")
sdl2="$(otool -L "$app/Contents/MacOS/smw" | awk '/libSDL2-2\.0\.0\.dylib/ {print $1}')"
sdl3="$(dirname "$(dirname "$(realpath "$sdl2")")")/../sdl3/lib/libSDL3.0.dylib"
[[ -f "$sdl3" ]] || sdl3="$(brew --prefix sdl3)/lib/libSDL3.0.dylib"
cp -L "$sdl3" "$frameworks/libSDL3.dylib"
chmod u+w "$frameworks/libSDL3.dylib"
queue+=("$frameworks/libSDL3.dylib")

# Homebrew libs refer to siblings as @rpath/... or @loader_path/..., so resolve those from where the lib came from.
origins="$(mktemp -d)"
trap 'rm -rf "$origins"' EXIT
dirname "$(realpath "$sdl3")" > "$origins/libSDL3.dylib"
resolve() {
    local dep=$1 from=$2 name
    name="$(basename "$dep")"
    for dir in "$(cat "$origins/$from" 2>/dev/null)" "$(brew --prefix)/lib"; do
        [[ -n "$dir" && -f "$dir/$name" ]] && { realpath "$dir/$name"; return; }
    done
    echo "cannot resolve $dep (needed by $from)" >&2
    return 1
}

while ((${#queue[@]})); do
    file="${queue[0]}"
    queue=("${queue[@]:1}")
    while read -r dep; do
        case "$dep" in /usr/lib/* | /System/* | @executable_path/*) continue ;; esac
        name="$(basename "$dep")"
        [[ "$dep" == "@rpath/$(basename "$file")" ]] && continue
        src="$dep"
        [[ "$dep" == @* ]] && src="$(resolve "$dep" "$(basename "$file")")"
        if [[ ! -f "$frameworks/$name" ]]; then
            dirname "$(realpath "$src")" > "$origins/$name"
            cp -L "$src" "$frameworks/$name"
            chmod u+w "$frameworks/$name"
            install_name_tool -id "@rpath/$name" "$frameworks/$name"
            queue+=("$frameworks/$name")
        fi
        install_name_tool -change "$dep" "@rpath/$name" "$file"
    done < <(otool -L "$file" | tail -n +2 | awk '{print $1}')
done
install_name_tool -id "@rpath/libSDL3.dylib" "$frameworks/libSDL3.dylib"
install_name_tool -add_rpath "@executable_path/../Frameworks" "$app/Contents/MacOS/smw"
for lib in "$frameworks"/*.dylib; do
    install_name_tool -add_rpath "@loader_path" "$lib" 2>/dev/null || true
done

# The newest minos of the binary and the bundled libs; Homebrew builds its bottles for the runner's macOS.
minos="$(for f in "$app/Contents/MacOS/smw" "$frameworks"/*.dylib; do
    otool -l "$f" | sed -n -e '/LC_BUILD_VERSION/,/minos/s/^ *minos //p' -e '/LC_VERSION_MIN_MACOSX/,/version/s/^ *version //p'
done | sort -u -V | tail -n 1)"
[[ -n "$minos" ]] || { echo "no minimum macOS version found" >&2; exit 1; }

iconset="$(mktemp -d)/smw.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$root/resources/smw.png" --out "$iconset/icon_${size}x${size}.png" > /dev/null
    sips -z $((size * 2)) $((size * 2)) "$root/resources/smw.png" --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/smw.icns"
rm -rf "$(dirname "$iconset")"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Super Mario War</string>
    <key>CFBundleDisplayName</key><string>Super Mario War</string>
    <key>CFBundleIdentifier</key><string>com.kylejameswalker.supermariowar-rust</string>
    <key>CFBundleExecutable</key><string>smw</string>
    <key>CFBundleIconFile</key><string>smw</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>$minos</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

codesign --force --deep --sign - "$app"
leftover="$(for f in "$app/Contents/MacOS/smw" "$frameworks"/*.dylib; do otool -L "$f" | tail -n +2; done | grep -E '/opt/homebrew|/usr/local' || true)"
if [[ -n "$leftover" ]]; then
    echo "non-bundled dylib references remain:" >&2
    echo "$leftover" >&2
    exit 1
fi
echo "built $app (macOS $minos or later)"
