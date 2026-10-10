# SVD Chunk Type Reference

## INTEGRA-7 Chunk Types (Model `MI69`)

| Code  | Chunk Type           | Entry Size | SysEx Tone Type      | Status | Page |
|-------|----------------------|------------|----------------------|--------|------|
| PRFb  | Studio Set           | 1068 B     | Studio Set           | Verified on real data (offline) | [05](05-studio-set.md) |
| RFPa  | PCM Synth Tone       | 590 B      | PCM Synth            | Verified on real data (offline) | [06](06-pcm-synth.md) |
| RFRa  | PCM Drum Kit         | 10890 B    | PCM Drum Kit         | INIT-only | [07](07-pcm-drum.md) |
| SHPa  | SN Synth Tone        | 280 B      | SuperNATURAL Synth   | Verified (device + 10,044 entries) | [03](03-sn-synth.md) |
| SNTa  | SN Acoustic Tone     | 138 B      | SuperNATURAL Acoustic| INIT-only | [08](08-sn-acoustic.md) |
| SDKa  | SN Drum Kit          | 1006 B     | SuperNATURAL Drum Kit| INIT-only | [09](09-sn-drum.md) |
| SYSa  | System               | 108 B      | System Common        | Bits 0–220 verified on real data (offline); rest undocumented | [10](10-system.md) |

Status levels, from strongest to weakest:

- **Verified (device)**: decoded entries match RQ1 reads from a physical
  INTEGRA-7.
- **Verified on real data (offline)**: real, non-INIT entries from the official
  Roland packs decode with 0 range violations, and names and categories match the
  packs' sound lists. Not yet compared with a device.
- **INIT-only**: the packs hold only INIT entries of this type. Section
  boundaries are pinned, but most fields have only ever been seen at their
  default value.
- **Inferred**: used per field on the chunk pages. The position fits, but the
  width or meaning is not proven.

## Code Mnemonics

The 4-character chunk codes appear to follow a pattern:

- **PRFb** — Performance (Studio Set) block
- **RFPa** — Reference Patch (PCM Synth)
- **RFRa** — Reference Rhythm (PCM Drum Kit)
- **SHPa** — Synth Patch (SuperNATURAL Synth)
- **SNTa** — SuperNATURAL Tone (Acoustic)
- **SDKa** — SuperNATURAL Drum Kit
- **SYSa** — System

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
| SN Drum Kit    | 1006 B    | 1427 B      | 1.42×             |
| PCM Drum Kit   | 10890 B   | ~18000+ B   | ~1.7×             |

SN Drum Kit: Common 20 + MFX 145 + Comp/EQ 84 + 62 notes × 19 = 1,427 bytes.

## Chunk Ordering

In all observed SVD files, chunks appear in this order:

1. PRFb (Studio Sets)
2. RFPa (PCM Synth Tones)
3. RFRa (PCM Drum Kits)
4. SHPa (SN Synth Tones)
5. SNTa (SN Acoustic Tones)
6. SDKa (SN Drum Kits)
7. SYSa (System), BACKUP files only

The first six chunks are always present in the directory, even when empty
(count = 0). See [SOUND vs BACKUP](11-sound-vs-backup.md).

## Known SVD Files

The 18 official Roland INTEGRA-7 packs each ship one BACKUP file and one SOUND
file. Every BACKUP file has the same shape: 811,292 bytes holding the full user
memory (PRFb 64, RFPa 256, RFRa 32, SHPa 512, SNTa 256, SDKa 64, SYSa 1). The
SOUND files hold only the exported items:

| Pack                                   | SOUND file       | PRFb | RFPa | RFRa | SHPa | SNTa | SDKa |
|----------------------------------------|------------------|------|------|------|------|------|------|
| Children of Bodom                      | `I7COB.SVD`      | 21   | 3    | 0    | 2    | 0    | 0    |
| D-50 Collection                        | `I7SLD50.SVD`    | 10   | 0    | 0    | 11   | 0    | 0    |
| Dream Rack Shorty                      | `I7SLShorty.SVD` | 50   | 0    | 0    | 81   | 0    | 0    |
| Dream Rack Synth Bass                  | `I7SLBass.SVD`   | 50   | 0    | 0    | 62   | 0    | 0    |
| Euro Attack Synth                      | `I7_EAS_E.SVD`   | 0    | 0    | 0    | 146  | 0    | 0    |
| JUNO-106 Collection                    | `I7SLJN106.SVD`  | 20   | 0    | 0    | 18   | 0    | 0    |
| JUNO-60 Collection                     | `I7SLJN60.SVD`   | 27   | 0    | 0    | 28   | 0    | 0    |
| JUPITER-6 Collection                   | `I7SLJP6.SVD`    | 15   | 0    | 0    | 16   | 0    | 0    |
| JUPITER-8 Collection                   | `I7SLJP8.SVD`    | 23   | 0    | 0    | 30   | 0    | 0    |
| NYC Nightlife Remix Collection         | `I7NYCNRC.SVD`   | 0    | 0    | 0    | 29   | 0    | 0    |
| SH-101 Collection                      | `I7SLSH101.SVD`  | 17   | 0    | 0    | 18   | 0    | 0    |
| SH-101 Collection Pure                 | `I7SLPSH101.SVD` | 17   | 0    | 0    | 17   | 0    | 0    |
| Sounds of Summer 2014                  | `I7SOS2014.SVD`  | 0    | 0    | 0    | 24   | 0    | 0    |
| Super Super Saw                        | `I7SSSAW.SVD`    | 0    | 0    | 0    | 40   | 0    | 0    |
| Synth Legends                          | `I7SL.SVD`       | 32   | 0    | 0    | 128  | 0    | 0    |
| Synth Legends Pure                     | `I7SLP.SVD`      | 0    | 0    | 0    | 128  | 0    | 0    |
| TB-303 Collection                      | `I7SLTB303.SVD`  | 24   | 0    | 0    | 25   | 0    | 0    |
| TB-303 Collection Pure                 | `I7SLPTB303.SVD` | 24   | 0    | 0    | 25   | 0    | 0    |
| **Total**                              |                  | 330  | 3    | 0    | 828  | 0    | 0    |

No pack ships a real PCM Drum Kit, SN Acoustic tone or SN Drum Kit. Those
chunks are known only from the INIT entries in the BACKUP files.
