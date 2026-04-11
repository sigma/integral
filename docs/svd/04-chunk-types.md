# SVD Chunk Type Reference

## INTEGRA-7 Chunk Types (Model `MI69`)

| Code  | Chunk Type           | Entry Size | SysEx Tone Type      | Status     |
|-------|----------------------|------------|----------------------|------------|
| PRFb  | Studio Set           | 1068 B     | Studio Set           | Unvalidated|
| RFPa  | PCM Synth Tone       | 590 B      | PCM Synth            | Unvalidated|
| RFRa  | PCM Drum Kit         | 10890 B    | PCM Drum Kit         | Unvalidated|
| SHPa  | SN Synth Tone        | 280 B      | SuperNATURAL Synth   | Validated  |
| SNTa  | SN Acoustic Tone     | 138 B      | SuperNATURAL Acoustic| Unvalidated|
| SDKa  | SN Drum Kit          | 1006 B     | SuperNATURAL Drum Kit| Unvalidated|

## Code Mnemonics

The 4-character chunk codes appear to follow a pattern:

- **PRFb** — Performance (Studio Set) block
- **RFPa** — Reference Patch (PCM Synth)
- **RFRa** — Reference Rhythm (PCM Drum Kit)
- **SHPa** — Synth Patch (SuperNATURAL Synth)
- **SNTa** — SuperNATURAL Tone (Acoustic)
- **SDKa** — SuperNATURAL Drum Kit

The trailing `a` or `b` may indicate a format revision.

## Entry Size vs SysEx Size

The SVD entry size is always much smaller than the corresponding SysEx
parameter space because SVD uses bit-packing (only significant bits stored)
while SysEx uses one byte per parameter with the high bit always 0.

| Chunk Type     | SVD Entry | SysEx Total | Compression Ratio |
|----------------|-----------|-------------|-------------------|
| SN Synth       | 280 B     | 520 B       | 1.86×             |
| SN Acoustic    | 138 B     | 343 B       | 2.49×             |
| PCM Synth      | 590 B     | 1582 B      | 2.68×             |
| Studio Set     | 1068 B    | ~2000+ B    | ~2×               |
| SN Drum Kit    | 1006 B    | ~1200+ B    | ~1.2×             |
| PCM Drum Kit   | 10890 B   | ~18000+ B   | ~1.7×             |

## Chunk Ordering

In all observed SVD files, chunks appear in this order:

1. PRFb (Studio Sets)
2. RFPa (PCM Synth Tones)
3. RFRa (PCM Drum Kits)
4. SHPa (SN Synth Tones)
5. SNTa (SN Acoustic Tones)
6. SDKa (SN Drum Kits)

All six chunks are always present in the directory, even when empty (count = 0).

## Known SVD Files

| File                  | PRFb | RFPa | RFRa | SHPa | SNTa | SDKa |
|-----------------------|------|------|------|------|------|------|
| Synth Legends (I7SL)  | 32   | 0    | 0    | 128  | 0    | 0    |
| Euro Attack Synth     | 0    | 0    | 0    | 146  | 0    | 0    |
