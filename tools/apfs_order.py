#!/usr/bin/env python3
"""Copy a directory tree, creating each directory's entries in the order APFS lists them.

Usage: tools/apfs_order.py <src> <dst>

The game and editors list some data directories unsorted (readdir order), and the goldens were made on
APFS, which lists a directory by a hash of each name. FAT lists entries in the order they were created,
so copying data/ onto a FAT image with this tool reproduces the macOS order on Linux.
"""
import os
import shutil
import sys
import unicodedata


def _crc32c_table():
    table = []
    for i in range(256):
        c = i
        for _ in range(8):
            c = (c >> 1) ^ 0x82F63B78 if c & 1 else c >> 1
        table.append(c)
    return table


CRC32C = _crc32c_table()


def apfs_key(name):
    """j_drec_hashed_key_t.name_len_and_hash of a case-insensitive APFS volume."""
    crc = 0xFFFFFFFF
    for b in unicodedata.normalize('NFD', name).casefold().encode('utf-32-le'):
        crc = CRC32C[(crc ^ b) & 0xFF] ^ (crc >> 8)
    return ((crc & 0x3FFFFF) << 10) | ((len(name.encode()) + 1) & 0x3FF)


def copy(src, dst):
    os.mkdir(dst)
    for name in sorted(os.listdir(src), key=apfs_key):
        s, d = os.path.join(src, name), os.path.join(dst, name)
        if os.path.isdir(s) and not os.path.islink(s):
            copy(s, d)
        else:
            shutil.copyfile(s, d)


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit(__doc__.splitlines()[2])
    copy(sys.argv[1], sys.argv[2])
