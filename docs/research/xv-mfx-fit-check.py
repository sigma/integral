#!/usr/bin/env python3
"""Check XV MFX types 41-90 against their INTEGRA-7 targets' layouts.

For every distinct XV MFX block (Model 00 10, DT1 at `aa bb 02 00`) of a
type 41-90 found in the `.syx`/`.mid` files under CORPUS, read Parameters
1-32 and test each target slot against its stored range in
`mfx-param-layout.csv`. In-range values don't prove the layout; an
out-of-range one disproves it. A type whose every block fits is checked
against the XV-5050 manual before it gets `copy_slots` in
`foreign-fx-map.json` (see `foreign-fx-map.md`, Unresolved 1).

Controls, for each type that fits: how many blocks still fit with the
Parameters shifted 1 or 2 slots either way, and how many other INTEGRA-7
types every block also fits. A fit that survives those may be coincidental.

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


def parameters(block):
    """MFX Parameters 1-32 of a block, as stored values."""
    return [
        (block[PARAMS + 4 * i] << 12 | block[PARAMS + 4 * i + 1] << 8
         | block[PARAMS + 4 * i + 2] << 4 | block[PARAMS + 4 * i + 3]) - 32768
        for i in range(32)
    ]


def fits(params, limits, shift=0):
    """Whether every slot of `limits` holds a value in range, shifted."""
    def value(n):
        i = n - 1 + shift
        return params[i] if 0 <= i < len(params) else 0
    return all(lo <= value(n) <= hi for n, (lo, hi) in limits.items())


def main(corpus):
    limits, types = ranges(), targets()
    per_type = collections.defaultdict(list)
    for block in blocks(corpus):
        if types[block[0]][0] is not None:
            per_type[block[0]].append(parameters(block))
    for xv in sorted(per_type):
        target, name = types[xv]
        params = per_type[xv]
        fit = sum(fits(p, limits[target]) for p in params)
        line = f"{xv:3} {name:22} -> {target:3}  blocks {len(params):4}  fit {fit:4}"
        if fit == len(params):
            shifted = sum(fits(p, limits[target], s) for p in params for s in (-2, -1, 1, 2))
            others = [t for t in limits if t != target and all(fits(p, limits[t]) for p in params)]
            line += f"  shifted fits {shifted}  other types {others}"
        print(line)
    missing = sorted(t for t, (g, _) in types.items() if g and t not in per_type)
    print("no corpus data:", missing)


if __name__ == "__main__":
    main(sys.argv[1])
