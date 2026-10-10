# SN Drum Kit — SVD Mapping (SDKa)

Chunk type: `SDKa` | Entry size: **1006 bytes** | Model: `MI69` |
SysEx block: SuperNATURAL Drum Kit (`19 03 00 00` for the Part 1 Temporary Tone)

**Confidence: INIT-only.** The 1,152 occurrences in the 18 official packs
(18 × 64, BACKUP files only) contain only 2 distinct entries, `INIT KIT` and
`INITIAL KIT`. No pack ships a real SN Drum Kit. Section sizes are fixed by the
bit counts and the MFX header. The Note fields are range-checked over 71,424
notes, but those notes all hold the same INIT values.

SysEx offsets are relative to each block's start. See
[`docs/midi/10-supernatural-drum-kit.md`](../midi/10-supernatural-drum-kit.md).

## Entry Layout

| Byte Range               | Section               | SysEx block                | Bits | Bytes |
|--------------------------|-----------------------|----------------------------|------|-------|
| 0–17                     | Common                | `00 00 00`, size `14`      | 134  | 18    |
| 18–95                    | MFX                   | `00 02 00`, size `01 11`   | 618  | 78    |
| 96–137                   | Comp/EQ               | `00 08 00`, size `54`      | 330  | 42    |
| 138 + 14(k−27)           | Note, key k (27…88)   | `00 10+(k−27) 00`, size `13` | 102 | 14 each (868 total) |
| **Total**                |                       |                            |      | **1006** |

The sections fill the entry exactly. There is **no** `0x0E` end marker.

Total SysEx size is 20 + 145 + 84 + 62 × 19 = **1,427 bytes**.

## Common (entry bytes 0–17)

The kit name is 12 × 7-bit ASCII followed by 4 reserve characters stored as
7-bit spaces, as in SN Acoustic.

| SysEx | Parameter | Bits | Section bit | Entry byte.bit | Range | Encoding | INIT |
|---|---|---|---|---|---|---|---|
| `00 00` | Kit Name 1 | 7 | 0 | 0.0 | 32–127 | raw | 73 |
| `00 01` | Kit Name 2 | 7 | 7 | 0.7 | 32–127 | raw | 78 |
| `00 02` | Kit Name 3 | 7 | 14 | 1.6 | 32–127 | raw | 73 |
| `00 03` | Kit Name 4 | 7 | 21 | 2.5 | 32–127 | raw | 84 |
| `00 04` | Kit Name 5 | 7 | 28 | 3.4 | 32–127 | raw | 32 |
| `00 05` | Kit Name 6 | 7 | 35 | 4.3 | 32–127 | raw | 75 |
| `00 06` | Kit Name 7 | 7 | 42 | 5.2 | 32–127 | raw | 73 |
| `00 07` | Kit Name 8 | 7 | 49 | 6.1 | 32–127 | raw | 84 |
| `00 08` | Kit Name 9 | 7 | 56 | 7.0 | 32–127 | raw | 32 |
| `00 09` | Kit Name 10 | 7 | 63 | 7.7 | 32–127 | raw | 32 |
| `00 0A` | Kit Name 11 | 7 | 70 | 8.6 | 32–127 | raw | 32 |
| `00 0B` | Kit Name 12 | 7 | 77 | 9.5 | 32–127 | raw | 32 |
| `00 0C` | (reserve name 13) | 7 | 84 | 10.4 | 32–127 | raw | 32 |
| `00 0D` | (reserve name 14) | 7 | 91 | 11.3 | 32–127 | raw | 32 |
| `00 0E` | (reserve name 15) | 7 | 98 | 12.2 | 32–127 | raw | 32 |
| `00 0F` | (reserve name 16) | 7 | 105 | 13.1 | 32–127 | raw | 32 |
| `00 10` | Kit Level | 7 | 112 | 14.0 | 0–127 | raw | 100 |
| `00 11` | Ambience Level | 7 | 119 | 14.7 | 0–127 | raw | 64 |
| `00 12` | Phrase Number | 7 | 126 | 15.6 | 0–127 | raw | 0 |
| `00 13` | TFX Switch | 1 | 133 | 16.5 | 0–1 | raw | 1 |

## MFX (entry bytes 18–95)

Same 618-bit layout as every other tone type. See
[PCM Synth Tone, Common MFX](06-pcm-synth.md#common-mfx-entry-bytes-56133).
INIT values are 32768 (= 0) for every MFX parameter, and MFX `00 01` (reserve)
is stored as 127, as in SN Synth INIT tones.

## Comp/EQ (entry bytes 96–137)

Six groups of 55 bits, then 6 zero pad bits. Group g (1–6) starts at section bit
`55(g−1)` and SysEx offset `00 0E` × (g−1). Group 1 is shown below. The full
six-group table is in
[PCM Drum Kit, Common Comp/EQ](07-pcm-drum.md#common-compeq-entry-bytes-92133),
which uses the same layout.

| SysEx | Parameter | Bits | Section bit | Entry byte.bit | Range | Encoding | INIT |
|---|---|---|---|---|---|---|---|
| `00 00` | Comp1 Switch | 1 | 0 | 96.0 | 0–1 | raw | 0 |
| `00 01` | Comp1 Attack Time | 5 | 1 | 96.1 | 0–31 | raw | 0 |
| `00 02` | Comp1 Release Time | 5 | 6 | 96.6 | 0–23 | raw | 0 |
| `00 03` | Comp1 Threshold | 7 | 11 | 97.3 | 0–127 | raw | 0 |
| `00 04` | Comp1 Ratio | 5 | 18 | 98.2 | 0–19 | raw | 0 |
| `00 05` | Comp1 Output Gain | 5 | 23 | 98.7 | 0–24 | raw | 0 |
| `00 06` | EQ1 Switch | 1 | 28 | 99.4 | 0–1 | raw | 0 |
| `00 07` | EQ1 Low Freq | 1 | 29 | 99.5 | 0–1 | raw | 0 |
| `00 08` | EQ1 Low Gain | 5 | 30 | 99.6 | 0–30 | raw | 15 |
| `00 09` | EQ1 Mid Freq | 5 | 35 | 100.3 | 0–16 | raw | 0 |
| `00 0A` | EQ1 Mid Gain | 5 | 40 | 101.0 | 0–30 | raw | 15 |
| `00 0B` | EQ1 Mid Q | 3 | 45 | 101.5 | 0–4 | raw | 0 |
| `00 0C` | EQ1 High Freq | 2 | 48 | 102.0 | 0–2 | raw | 0 |
| `00 0D` | EQ1 High Gain | 5 | 50 | 102.2 | 0–30 | raw | 15 |

## Note (14 bytes per key, entry byte `138 + 14(k−27)`, k = 27…88)

The "Entry byte.bit" column shows key 27.

| SysEx | Parameter | Bits | Section bit | Entry byte.bit | Range | Encoding | INIT |
|---|---|---|---|---|---|---|---|
| `00 00` | Inst Number | 16 | 0 | 138.0 | 0–512 | 4 nibbles, MSB first | 1 |
| `00 04` | Level | 7 | 16 | 140.0 | 0–127 | raw | 100 |
| `00 05` | Pan | 7 | 23 | 140.7 | 0–127 | raw | 64 |
| `00 06` | Chorus Send Level | 7 | 30 | 141.6 | 0–127 | raw | 0 |
| `00 07` | Reverb Send Level | 7 | 37 | 142.5 | 0–127 | raw | 64 |
| `00 08` | Tune | 16 | 44 | 143.4 | 8–248 | 4 nibbles, MSB first | 128 |
| `00 0C` | Attack | 7 | 60 | 145.4 | 0–100 | raw | 100 |
| `00 0D` | Decay | 7 | 67 | 146.3 | 1–64 | raw | 64 |
| `00 0E` | Brilliance | 5 | 74 | 147.2 | 49–76 | bias | 64 |
| `00 0F` | Variation | 7 | 79 | 147.7 | 0–7 | raw | 0 |
| `00 10` | Dynamic Range | 6 | 86 | 148.6 | 0–63 | raw | 32 |
| `00 11` | Stereo Width | 7 | 92 | 149.4 | 0–127 | raw | 127 |
| `00 12` | Output Assign | 3 | 99 | 150.3 | 0–6 | raw | 0 |

**Brilliance** uses the bias rule (5 bits, `sysex = svd + 48`). The stored 16
decodes to SysEx 64, which displays as 0. Read raw, it would be out of its
documented range 49–76 in all 71,424 notes.

## Evidence

- Each stored section size is fixed by its bit count. Common 134 bits → 18 B,
  Comp/EQ 330 → 42 B, Note 102 → 14 B, and 18 + 78 + 42 + 62 × 14 = 1006 exactly.
- The MFX header pattern sits at byte 18, byte-identical to the one at SN Synth
  byte 30 and SN Acoustic byte 60.
- Every decoded value is within range once Brilliance is biased. The INIT values
  in the tables are plausible defaults (Kit Level 100, EQ gains 15 = 0 dB, Tune
  128, Level 100, Pan 64).

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| Everything beyond INIT values (only 2 distinct entries exist) | Build an SN drum kit with notes edited at both ends (keys 27 and 88), non-zero Comp/EQ, Brilliance at −15 and +12, Variation and Output Assign. Save it, back up, and compare against RQ1 `19 03 00 00` … `19 03 4D 00`. |
| Note-section and Common padding | The same backups: the pad bits must stay 0. |
