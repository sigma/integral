# SVD Bit-Packing Encoding

## Principle

Each SysEx parameter in the MIDI Implementation spec has a **bit mask** that
defines how many significant bits it carries. In the SVD format, parameters
are packed end-to-end using only these significant bits, MSB first.

For example, a sequence of parameters with bit masks `0aaa aaaa` (7 bits),
`0000 000a` (1 bit), and `0000 0aaa` (3 bits) would pack as:

```
SysEx bytes:   [0aaa aaaa] [0000 000b] [0000 0ccc]
SVD bits:       aaaaaaa     b           ccc
Packed:        |aaaaaaa b ccc 00000|   (11 bits → 2 bytes, padded to 16)
```

## Bit Width Rules

The number of bits stored per parameter is derived from the bit mask pattern
in the MIDI Implementation document:

| Bit Mask Pattern | Significant Bits | Example Parameters              |
|------------------|------------------|---------------------------------|
| `0aaa aaaa`      | 7                | Level, Pan, Cutoff, Rate, etc.  |
| `00aa aaaa`      | 6                | OSC Pitch, Filter Keyfollow     |
| `000a aaaa`      | 5                | Pitch Bend Range, Control Assign|
| `0000 aaaa`      | 4                | (nibble of nibblized param)     |
| `0000 0aaa`      | 3                | OSC Wave, Filter Mode, LFO Shape|
| `0000 00aa`      | 2                | Mono Switch, Ring Switch        |
| `0000 000a`      | 1                | ON/OFF switches                 |

## Nibblized Parameters

Parameters marked with `#` in the MIDI Implementation span multiple SysEx
bytes, each carrying a 4-bit nibble (`0000 aaaa`). In SVD, each nibble is
stored as 4 bits:

```
SysEx (4-nibble param): [0000 aaaa] [0000 bbbb] [0000 cccc] [0000 dddd]
SVD bits:                aaaa        bbbb        cccc        dddd
Packed:                 |aaaabbbbccccdddd|   (16 bits = 2 bytes)
```

Common nibblized parameters:
- **Phrase Number**: 4 nibbles (16 bits, values 0–65535)
- **Wave Number**: 4 nibbles (16 bits, values 0–16384)
- **MFX Parameters 1–32**: 4 nibbles each (16 bits, values 12768–52768)
- **Reserve fields**: 3 nibbles (12 bits)

## Section Alignment

Parameters are grouped into **sections** that correspond to SysEx address
blocks (Common, MFX, Partial 1, Partial 2, etc.). Each section is
independently byte-aligned with **fixed byte sizes** that may include
extra zero-padding beyond the mathematical byte ceiling.

```
Section N:  [param bits...] [0-padding to fixed section size]
Section N+1: [param bits...] [0-padding to fixed section size]
```

Each section starts at a byte-aligned offset within the entry.  The
section byte sizes are fixed per tone type and do NOT necessarily equal
`ceil(total_bits / 8)` — there may be extra padding bytes.

Observed sizes show why the sizes have to be tabulated rather than derived.
Every stored size is even, but it isn't simply `ceil(bits / 16) × 2` either:
SN Synth Partial would then be 44 bytes, not 46.

| Type | Section          | Bits | ceil(bits/8) | Stored bytes |
|------|------------------|------|--------------|--------------|
| SHPa | Common           | 228  | 29           | 30           |
| SHPa | Partial (× 3)    | 350  | 44           | 46           |
| SNTa | Common           | 464  | 58           | 60           |
| RFPa | PMT              | 242  | 31           | 32           |
| SDKa | Common           | 134  | 17           | 18           |
| SDKa | Comp/EQ          | 330  | 42           | 42           |
| SDKa | Note (× 62)      | 102  | 13           | 14           |
| all  | MFX              | 618  | 78           | 78           |

**Important:** Common and MFX are **separate** sections (validated against
the device). Earlier analysis incorrectly assumed they were packed together
into a single section.

## Signed Parameter Bias

Parameters with ranges centered at 64 in SysEx (e.g., Octave Shift 61–67,
OSC Pitch 40–88, Keyfollow 54–74) use a **bias encoding** in the SVD:

```
svd_value = sysex_value - 64 + 2^(bits-1)
sysex_value = svd_value + 64 - 2^(bits-1)
```

For 7-bit parameters, the bias is zero (64 - 64 = 0), so no adjustment is
needed. For smaller bit widths, the SVD re-centers the value at the midpoint
of the N-bit range instead of 64.

Examples (device validation):
- Octave Shift (3 bits, SysEx 64 = center): SVD stores 4 (= 64 - 64 + 4)
- OSC Pitch (6 bits, SysEx 64 = center): SVD stores 32 (= 64 - 64 + 32)
- Keyfollow (6 bits, SysEx 67): SVD stores 35 (= 67 - 64 + 32)

Parameters with ranges starting at 0 (Level, switches, enums) store the
raw value directly — no bias adjustment.

A rule that predicts which parameters are biased: a single-byte parameter is
biased when it has fewer than 7 bits and its range's upper bound does not fit
in those bits (`bits < 7` and `range.hi >= 2^bits`). It holds for every biased
field found so far in SHPa, SNTa, PRFb, RFPa, RFRa, SDKa (Brilliance) and SYSa
(Master Key Shift).

## End Marker

Only **SHPa** (SN Synth) entries have an end marker: a single `0x0E` byte
after the last section, at byte 246. SNTa, SDKa and PRFb sections fill their
entries exactly, and RFPa and RFRa end in plain zero padding. See
[Entry Structure](01-container.md#entry-structure).

The SHPa bytes after the marker are not always zero. See
[SN Synth, Bytes After the End Marker](03-sn-synth.md#bytes-after-the-end-marker).

## Reserve Parameters

Reserve parameters (marked `<*>` in the MIDI Implementation) are included
in the bitstream at their specified bit widths. They typically carry zero
values but **must** be preserved for round-trip fidelity. Omitting them
would cause bit-position drift for all subsequent parameters.

Two caveats from the pack data:

- Reserves often carry **non-zero** values. MFX `00 01` is always 127, PCM
  Synth Common `00 27` is always 13, and some reserves hold undocumented data
  such as the PCM long name. Each chunk page lists them.
- Reserve widths in the SVD do not always match the SysEx bit mask. In PRFb,
  Part reserves `00 02`–`00 05` take 1 bit each, and Common `00 10`–`00 38` take
  257 bits rather than 287. See [Studio Set](05-studio-set.md).
