#!/usr/bin/env python3
"""Copies the bundled shader presets (shaders/presets/*.slangp) and every file they use into a release folder.

It follows each preset's #reference chain, its shaderN passes and textures, and each shader's #include lines, and copies
the files with their paths relative to shaders/, so the references still resolve. It then writes SHADER_LICENSES.md:
each upstream file with the license line from its header.

Usage: tools/shader_bundle.py <out_dir>
"""
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "shaders"
UPSTREAM = ROOT / "slang-shaders"
LICENSE_LINE = re.compile(r"(GNU (Lesser )?General Public License|GPL|public domain|MIT License|Permission is hereby granted|BSD|Apache License|CC0|Creative Commons|zlib)", re.I)


def value(raw):
    return raw.strip().strip('"').strip()


def preset_files(path, seen):
    """The preset at `path` and every file it uses."""
    path = path.resolve()
    if path in seen:
        return
    seen.add(path)
    entries = {}
    for line in path.read_text(errors="replace").splitlines():
        line = line.split("//")[0].strip() if not line.lstrip().startswith("#reference") else line.strip()
        if line.startswith("#reference"):
            preset_files(path.parent / value(line[len("#reference"):]), seen)
        elif "=" in line:
            key, raw = line.split("=", 1)
            entries[key.strip()] = value(raw)
    for key, val in entries.items():
        if re.fullmatch(r"shader\d+", key):
            shader_files(path.parent / val, seen)
    for name in filter(None, (t.strip() for t in entries.get("textures", "").split(";"))):
        if name in entries:
            texture = (path.parent / entries[name]).resolve()
            if texture.is_file():
                seen.add(texture)


def shader_files(path, seen):
    """The shader at `path` and every file it includes."""
    path = path.resolve()
    if path in seen or not path.is_file():
        return
    seen.add(path)
    for m in re.finditer(r'^\s*#include\s+"([^"]+)"', path.read_text(errors="replace"), re.M):
        shader_files(path.parent / m.group(1), seen)


def license_line(path):
    for line in path.read_text(errors="replace").splitlines()[:150]:
        if LICENSE_LINE.search(line):
            return line.strip(" /*#\t")
    return "no license line; part of the shader whose main file is listed above"


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = Path(sys.argv[1]).resolve()
    presets = sorted((ROOT / "presets").glob("*.slangp"))
    if not presets:
        sys.exit(f"no presets in {ROOT / 'presets'}")
    seen = set()
    for preset in presets:
        preset_files(preset, seen)
    missing = [p for p in seen if not p.is_file()]
    if missing:
        sys.exit("missing: " + ", ".join(str(p) for p in missing))
    shutil.rmtree(out, ignore_errors=True)
    for path in sorted(seen):
        dest = out / path.relative_to(ROOT.resolve())
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
    commit = subprocess.run(["git", "-C", str(UPSTREAM), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    upstream = sorted(p for p in seen if p.is_relative_to(UPSTREAM.resolve()) and p.suffix in (".slang", ".h", ".inc", ".glsl"))
    lines = [
        "# Shader licenses",
        "",
        "The shader presets in `presets/` belong to Super Mario War. The files in `slang-shaders/` come unchanged from",
        f"[libretro/slang-shaders](https://github.com/libretro/slang-shaders) at commit `{commit}`, and each keeps the license",
        "in its own header. The game runs them through [librashader](https://github.com/SnowflakePowered/librashader), used",
        "under the Mozilla Public License 2.0; its source is at that link.",
        "",
        "| File | License line from its header |",
        "|---|---|",
    ]
    lines += [f"| `{p.relative_to(ROOT.resolve())}` | {license_line(p).replace('|', '/')} |" for p in upstream]
    (out / "SHADER_LICENSES.md").write_text("\n".join(lines) + "\n")
    size = sum(p.stat().st_size for p in seen)
    print(f"wrote {len(presets)} presets and {len(seen) - len(presets)} files ({size // 1024} KB) to {out}")


if __name__ == "__main__":
    main()
