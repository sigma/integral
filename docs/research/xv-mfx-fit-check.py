#!/usr/bin/env python3
"""Check XV MFX types 41-90 against their INTEGRA-7 targets' layouts.

For every distinct XV MFX block (Model 00 10, DT1 at `aa bb 02 00`) of a
type 41-90 found in the `.syx`/`.mid` files under CORPUS, read Parameters
1-32 and test each target slot against its stored range in
`mfx-param-layout.csv`. A type whose every block fits is marked `copy` in
`foreign-fx-map.json` (see `foreign-fx-map.md`, Unresolved 1). In-range
values don't prove the layout; an out-of-range one disproves it.

Usage: xv-mfx-fit-check.py CORPUS
"""

import collections
import csv
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PARAMS = 0x11


def ranges():
    """Stored (min, max) per INTEGRA-7 type and MFX Parameter number."""
    out = collections.defaultdict(dict)
    with open(os.path.join(HERE, "mfx-param-layout.csv")) as f:
        for row in csv.DictReader(f):
            if row["stored_min"]:
                lo, hi = int(row["stored_min"]), int(row["stored_max"])
                out[int(row["type"])][int(row["slot"])] = (lo, hi)
    return out


def targets():
    """XV type -> (INTEGRA-7 type or None, name), from the map's leading number."""
    with open(os.path.join(HERE, "foreign-fx-map.json")) as f:
        rows = json.load(f)["xv_mfx_41_90"]
    out = {}
    for row in rows:
        digits = ""
        for c in row["integra"]:
            if not c.isdigit():
                break
            digits += c
        out[row["xv_type"]] = (int(digits) if digits else None, row["name"])
    return out


def xv_dt1s(data):
    """Address and data of each XV DT1 in raw bytes (SMF SysEx events too)."""
    i = 0
    while (i := data.find(b"\xf0\x41", i)) >= 0:
        j = data.find(b"\xf7", i)
        if j < 0:
            return
        message = data[i:j]
        if message[3:6] == b"\x00\x10\x12":
            yield message[6:10], message[10:-1]
        i = j


def blocks(corpus):
    """Distinct MFX blocks of types 41-90."""
    found = set()
    for root, _, files in os.walk(corpus):
        for name in files:
            if not name.lower().endswith((".syx", ".mid")):
                continue
            with open(os.path.join(root, name), "rb") as f:
                data = f.read()
            for address, block in xv_dt1s(data):
                if address[2:] == b"\x02\x00" and len(block) >= PARAMS + 128:
                    if 41 <= block[0] <= 90:
                        found.add(bytes(block[: PARAMS + 128]))
    return found


def main(corpus):
    limits, types = ranges(), targets()
    stats = collections.defaultdict(lambda: [0, 0])
    for block in blocks(corpus):
        target, _ = types[block[0]]
        if target is None:
            continue
        params = [
            (block[PARAMS + 4 * i] << 12 | block[PARAMS + 4 * i + 1] << 8
             | block[PARAMS + 4 * i + 2] << 4 | block[PARAMS + 4 * i + 3]) - 32768
            for i in range(32)
        ]
        fits = all(lo <= params[n - 1] <= hi for n, (lo, hi) in limits[target].items())
        stats[block[0]][0] += 1
        stats[block[0]][1] += fits
    for xv in sorted(stats):
        target, name = types[xv]
        total, fit = stats[xv]
        print(f"{xv:3} {name:22} -> {target:3}  blocks {total:4}  fit {fit:4}")
    missing = sorted(t for t, (g, _) in types.items() if g and t not in stats)
    print("no corpus data:", missing)


if __name__ == "__main__":
    main(sys.argv[1])
