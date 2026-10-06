#!/usr/bin/env python3
"""Writes the launcher icons for every mipmap density from resources/smw.png (32x32 RGBA), scaled by whole
numbers with nearest neighbour so each source pixel stays a sharp square, plus an adaptive icon (Android 8+)
whose background layer is the icon's sky blue.

Usage: android/make_icons.py <smw.png> <res dir>
"""
import struct
import sys
import zlib
from pathlib import Path

DENSITIES = {'mdpi': 1.0, 'hdpi': 1.5, 'xhdpi': 2.0, 'xxhdpi': 3.0, 'xxxhdpi': 4.0}
LEGACY_DP = 48
LAYER_DP = 108
# Inside the adaptive icon's 66 dp safe circle, corners included.
ART_DP = 48


def read_png(path):
    data = Path(path).read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        sys.exit(f'{path}: not a PNG')
    pos, idat, header = 8, b'', None
    while pos < len(data):
        length, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        pos += 12 + length
    width, height, depth, color, _, _, interlace = header
    if (depth, color, interlace) != (8, 6, 0):
        sys.exit(f'{path}: expected 8-bit RGBA, not interlaced')
    raw = zlib.decompress(idat)
    stride = width * 4
    rows, prev = [], bytearray(stride)
    for y in range(height):
        kind = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for x in range(stride):
            a = line[x - 4] if x >= 4 else 0
            b = prev[x]
            c = prev[x - 4] if x >= 4 else 0
            if kind == 1:
                line[x] = (line[x] + a) & 255
            elif kind == 2:
                line[x] = (line[x] + b) & 255
            elif kind == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return width, height, rows


def write_png(path, width, rows):
    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))
    raw = b''.join(b'\x00' + row for row in rows)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, len(rows), 8, 6, 0, 0, 0))
    png += chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_bytes(png)


def place(src, size, art):
    """`src` scaled by the largest whole number that fits in `art` pixels, centred on a transparent `size` square."""
    width, height, rows = src
    k = max(1, int(art // max(width, height)))
    x0, y0 = (size - width * k) // 2, (size - height * k) // 2
    out = [bytearray(size * 4) for _ in range(size)]
    for y in range(height * k):
        line = rows[y // k]
        for x in range(width * k):
            out[y0 + y][(x0 + x) * 4:(x0 + x) * 4 + 4] = line[(x // k) * 4:(x // k) * 4 + 4]
    return [bytes(r) for r in out]


def main():
    src = read_png(sys.argv[1])
    res = Path(sys.argv[2])
    width, height, rows = src
    r, g, b, _ = rows[height // 4][width // 8 * 4:width // 8 * 4 + 4]
    for name, density in DENSITIES.items():
        size = round(LEGACY_DP * density)
        write_png(res / f'mipmap-{name}' / 'ic_launcher.png', size, place(src, size, size))
        layer = round(LAYER_DP * density)
        write_png(res / f'mipmap-{name}' / 'ic_launcher_foreground.png', layer, place(src, layer, ART_DP * density))
    (res / 'values').mkdir(parents=True, exist_ok=True)
    (res / 'values' / 'ic_launcher_background.xml').write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n<resources>\n'
        f'    <color name="ic_launcher_background">#{r:02X}{g:02X}{b:02X}</color>\n</resources>\n')
    (res / 'mipmap-anydpi-v26').mkdir(parents=True, exist_ok=True)
    (res / 'mipmap-anydpi-v26' / 'ic_launcher.xml').write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
        '    <background android:drawable="@color/ic_launcher_background" />\n'
        '    <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n'
        '</adaptive-icon>\n')


if __name__ == '__main__':
    main()
