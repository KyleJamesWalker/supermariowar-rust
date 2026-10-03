#!/usr/bin/env python3
"""Regenerate src/alias_audit.rs: a test that every struct reachable through Ptr<T>, Global<T> or a `static mut`
carries the Aliased marker (directly or through a field), so LLVM gets no noalias/readonly for &self/&mut self.

Every concrete type behind a Ptr<dyn Trait> counts too, via its `impl Trait for Type`, as does every non-Copy
struct with a &self/&mut self method containing `unsafe` (it can reach itself again through a global).

Usage: alias_audit.py [--check]
Writes src/alias_audit.rs (with --check: exits 1 if it is out of date instead). Then `cargo test alias_audit`
fails listing every target type that is still Unpin. Types in ALLOW are plain values that are never reached
through another path while borrowed; each needs a reason.
"""
import os
import re
import sys

PORT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(PORT, 'src')
OUT = os.path.join(SRC, 'alias_audit.rs')
LIB_MODULES = re.findall(r'(?m)^pub mod (\w+);', open(os.path.join(SRC, 'lib.rs')).read())

ALLOW = {
    'MixChunkPtr': 'owning handle; the raw pointer is set once at construction and never written through another path',
    'MixMusicPtr': 'owning handle; the raw pointer is set once at construction and never written through another path',
    'CInputPlayerControl': 'Copy value image of controls.sdl2.bin',
    'COutputControl': 'Copy key-state value; Ptr targets are only read and written field by field',
    'EditorMapTile': 'Copy value',
    'TileType': 'Copy newtype',
    'Warp': 'Copy value',
    'WarpExit': 'Copy value',
    'WorldWarp': 'Copy value',
    'SystemRandomNumberGenerator': 'stateless unit struct',
    'NetConfigManager': 'stateless unit struct',
}

STRUCT_RE = re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*(<)?', re.M)
WRAPPER_RE = re.compile(r'\b(?:Ptr|Global)<')
STATIC_RE = re.compile(r'\bstatic\s+mut\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*([^=;]+)')
IMPL_BLOCK_RE = re.compile(r'\bimpl(?:<[^>]*>)?\s+(?:[\w:]+(?:<[^>]*>)?\s+for\s+)?(\w+)(?:<[^>]*>)?\s*\{')
SELF_METHOD_RE = re.compile(r'fn \w+\s*(?:<[^>]*>)?\(\s*&(?:mut\s+)?self')
DERIVE_COPY_RE = re.compile(r'#\[derive\(([^)]*)\)\]\s*(?:#\[[^\]]*\]\s*)*pub struct (\w+)')
IMPL_RE = re.compile(r'\bimpl\s*(?:<[^>]*>)?\s*([A-Za-z_][A-Za-z0-9_]*)\s+for\s+([A-Za-z_][A-Za-z0-9_]*)\b')
TYPE_RE = re.compile(r'([A-Za-z_][A-Za-z0-9_:]*)(<[A-Za-z0-9_, ]*>)?')
PRIMITIVE_ARGS = re.compile(r'^<\s*(?:i8|i16|i32|i64|u8|u16|u32|u64|bool|f32|f64)(?:\s*,\s*(?:i8|i16|i32|i64|u8|u16|u32|u64|bool|f32|f64))*\s*>$')


def wrapped(text, start):
    """The type text inside the angle brackets that open at text[start - 1]."""
    depth, i = 1, start
    while i < len(text) and depth:
        depth += {'<': 1, '>': -1}.get(text[i], 0)
        i += 1
    return text[start:i - 1]


def block(text, start):
    """The body of the brace block that opens at text[start - 1]."""
    depth, i = 1, start
    while i < len(text) and depth:
        depth += {'{': 1, '}': -1}.get(text[i], 0)
        i += 1
    return text[start:i - 1]


def add_type_expr(expr, targets, dyns):
    expr = expr.strip()
    if expr.startswith('dyn '):
        dyns.add(expr[4:].split('+')[0].strip().split('::')[-1])
        return
    for m in TYPE_RE.finditer(expr.replace('[', ' ').replace(']', ' ')):
        name, args = m.group(1).split('::')[-1], m.group(2) or ''
        if name in ('Ptr', 'Global', 'Vec', 'Option', 'Box', 'String', 'mut', 'const', 'dyn'):
            continue
        targets.add(name + (args.replace(' ', '') if args and PRIMITIVE_ARGS.match(args) else ''))


def module_path(path):
    rel = os.path.relpath(path, SRC)[:-3].split(os.sep)
    if rel[-1] in ('mod', 'lib'):
        rel = rel[:-1]
    return 'crate::' + '::'.join(rel) if rel else 'crate'


def strip_tests(text):
    m = re.search(r'^#\[cfg\(test\)\]', text, re.M)
    return text[:m.start()] if m else text


def scan():
    structs, targets, dyns, impls, generic, copy, reentrant = {}, set(), set(), [], set(), set(), set()
    for root, dirs, files in os.walk(SRC):
        dirs[:] = sorted(d for d in dirs if root != SRC or d in LIB_MODULES)
        for name in sorted(files):
            if not name.endswith('.rs') or os.path.join(root, name) == OUT:
                continue
            if root == SRC and name[:-3] not in LIB_MODULES:
                continue
            path = os.path.join(root, name)
            text = strip_tests(open(path).read())
            for m in STRUCT_RE.finditer(text):
                if m.group(1) and not m.group(1).startswith('pub(super'):
                    structs.setdefault(m.group(2), []).append(module_path(path))
                    if m.group(3):
                        generic.add(m.group(2))
            for m in WRAPPER_RE.finditer(text):
                add_type_expr(wrapped(text, m.end()), targets, dyns)
            for m in STATIC_RE.finditer(text):
                add_type_expr(m.group(1), targets, dyns)
            impls += IMPL_RE.findall(text)
            copy |= {m.group(2) for m in DERIVE_COPY_RE.finditer(text) if 'Copy' in m.group(1)}
            for m in IMPL_BLOCK_RE.finditer(text):
                body = block(text, m.end())
                if SELF_METHOD_RE.search(body) and 'unsafe' in body:
                    reentrant.add(m.group(1))
    targets |= {ty for trait, ty in impls if trait in dyns}
    targets |= reentrant - copy
    return structs, {t for t in targets if '<' in t or t not in generic}


def render(structs, targets):
    rows = []
    for target in sorted(targets):
        name, _, args = target.partition('<')
        if name not in structs or target in ALLOW:
            continue
        for mod in structs[name]:
            rows.append(f'        row!({mod}::{target}),')
    return '\n'.join([
        '//! Generated by tools/alias_audit.py; do not edit. Lists every struct that must carry Aliased.',
        '#![cfg(test)]',
        '',
        'use std::marker::PhantomData;',
        '',
        'trait NotUnpin {',
        '    const UNPIN: bool = false;',
        '}',
        '',
        'struct Probe<T: ?Sized>(PhantomData<T>);',
        'impl<T: ?Sized> NotUnpin for Probe<T> {}',
        'impl<T: ?Sized + Unpin> Probe<T> {',
        '    const UNPIN: bool = true;',
        '}',
        '',
        'macro_rules! row {',
        '    ($t:ty) => {',
        '        if <Probe<$t>>::UNPIN { Some(stringify!($t)) } else { None }',
        '    };',
        '}',
        '',
        '#[test]',
        'fn alias_audit() {',
        '    let unpin: Vec<&str> = [',
        *rows,
        '    ]',
        '    .into_iter()',
        '    .flatten()',
        '    .collect();',
        '    assert!(unpin.is_empty(), "{} types lack Aliased (add `_alias: Aliased`, or a reasoned ALLOW entry in tools/alias_audit.py):\\n  {}", unpin.len(), unpin.join("\\n  "));',
        '}',
        '',
    ])


def main():
    structs, targets = scan()
    text = render(structs, targets)
    if '--check' in sys.argv:
        current = open(OUT).read() if os.path.exists(OUT) else ''
        if current != text:
            print('src/alias_audit.rs is out of date; run tools/alias_audit.py')
            return 1
        return 0
    open(OUT, 'w').write(text)
    return 0


if __name__ == '__main__':
    sys.exit(main())
