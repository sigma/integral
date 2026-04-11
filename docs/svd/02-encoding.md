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
blocks (e.g., Tone Common, MFX, Partial 1, Partial 2). After packing all
parameters in a section, the bitstream is **zero-padded to the next byte
boundary**.

```
Section N:  [param bits...] [0-padding to byte boundary]
Section N+1: [param bits...] [0-padding to byte boundary]
```

This means each section starts at a byte-aligned offset within the entry,
making it possible to compute section boundaries without unpacking all
preceding bits — just sum the padded byte sizes.

## Section Grouping

Some SysEx address blocks that are logically separate share a single SVD
section (i.e., they are packed consecutively without an alignment boundary
between them). The known groupings are:

- **SN Synth**: Common (`00 00 00`) + MFX (`00 02 00`) form one section
- Other tone types: TBD (to be validated)

## End Marker

After the last section, a single `0x0E` byte marks the end of meaningful
data. The remaining bytes in the entry are zero-padded to reach the fixed
entry size.

## Reserve Parameters

Reserve parameters (marked `<*>` in the MIDI Implementation) are included
in the bitstream at their specified bit widths. They typically carry zero
values but **must** be preserved for round-trip fidelity. Omitting them
would cause bit-position drift for all subsequent parameters.
