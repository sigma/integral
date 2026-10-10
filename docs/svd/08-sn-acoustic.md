# SN Acoustic Tone — SVD Mapping (SNTa)

Chunk type: `SNTa` | Entry size: **138 bytes** | Model: `MI69` |
SysEx block: SuperNATURAL Acoustic Tone (`19 02 00 00` for the Part 1 Temporary
Tone)

**Confidence: INIT-only.** The 4,608 occurrences in the 18 official packs
(18 × 256, BACKUP files only) contain only 3 distinct entries, all INIT. No pack
ships a real SN Acoustic tone. The section boundaries are pinned by the MFX
header and by the INIT defaults listed under [Evidence](#evidence). Fields that
hold the same value in all three entries are placed by MIDI Implementation order
only.

## Entry Layout (corrected)

| Byte Range | Section                         | Bits | Bytes |
|------------|---------------------------------|------|-------|
| 0–59       | Common (`00 00 00`, size `46`)  | 464  | **60** (58 data + 2 pad) |
| 60–137     | MFX (`00 02 00`, size `01 11`)  | 618  | 78    |
| **Total**  |                                 |      | **138** |

The two sections fill the entry exactly. There is **no** `0x0E` end marker:
byte 137 is `00` in all 4,608 entries.

> Current code diverges: see [#70](https://github.com/sigma/integral/issues/70). `SNA_TONE_SPEC` in
> `crates/integral-core/src/svd_specs.rs` sets Common to 59 bytes and
> `has_end_marker: true`. That puts MFX at byte 59 and expects `0x0E` at byte 137.
> With that layout MFX Control Sens decodes as 0 and all 32 MFX parameters as 128
> (out of range) in every entry. Common 60 bytes with no end marker gives 0
> violations.

## Common (entry bytes 0–59)

The tone name is 12 × 7-bit ASCII followed by 4 reserve characters stored as
7-bit spaces. SN Drum Kit uses the same scheme.

| SysEx | Parameter | Bits | Section bit | Entry byte.bit | Range | Encoding | INIT |
|---|---|---|---|---|---|---|---|
| `00 00` | Tone Name 1 | 7 | 0 | 0.0 | 32–127 | raw | 73 |
| `00 01` | Tone Name 2 | 7 | 7 | 0.7 | 32–127 | raw | 78 |
| `00 02` | Tone Name 3 | 7 | 14 | 1.6 | 32–127 | raw | 73 |
| `00 03` | Tone Name 4 | 7 | 21 | 2.5 | 32–127 | raw | 84 |
| `00 04` | Tone Name 5 | 7 | 28 | 3.4 | 32–127 | raw | 32 |
| `00 05` | Tone Name 6 | 7 | 35 | 4.3 | 32–127 | raw | 84 |
| `00 06` | Tone Name 7 | 7 | 42 | 5.2 | 32–127 | raw | 79 |
| `00 07` | Tone Name 8 | 7 | 49 | 6.1 | 32–127 | raw | 78 |
| `00 08` | Tone Name 9 | 7 | 56 | 7.0 | 32–127 | raw | 69 |
| `00 09` | Tone Name 10 | 7 | 63 | 7.7 | 32–127 | raw | 32 |
| `00 0A` | Tone Name 11 | 7 | 70 | 8.6 | 32–127 | raw | 32 |
| `00 0B` | Tone Name 12 | 7 | 77 | 9.5 | 32–127 | raw | 32 |
| `00 0C` | (reserve name 13) | 7 | 84 | 10.4 | 32–127 | raw | 32 |
| `00 0D` | (reserve name 14) | 7 | 91 | 11.3 | 32–127 | raw | 32 |
| `00 0E` | (reserve name 15) | 7 | 98 | 12.2 | 32–127 | raw | 32 |
| `00 0F` | (reserve name 16) | 7 | 105 | 13.1 | 32–127 | raw | 32 |
| `00 10` | Tone Level | 7 | 112 | 14.0 | 0–127 | raw | 127 |
| `00 11` | Mono/Poly | 1 | 119 | 14.7 | 0–1 | raw | 1 |
| `00 12` | Portamento Time Offset | 7 | 120 | 15.0 | 0–127 | raw | 64 |
| `00 13` | Cutoff Offset | 7 | 127 | 15.7 | 0–127 | raw | 64 |
| `00 14` | Resonance Offset | 7 | 134 | 16.6 | 0–127 | raw | 64 |
| `00 15` | Attack Time Offset | 7 | 141 | 17.5 | 0–127 | raw | 64 |
| `00 16` | Release Time Offset | 7 | 148 | 18.4 | 0–127 | raw | 64 |
| `00 17` | Vibrato Rate | 7 | 155 | 19.3 | 0–127 | raw | 64 |
| `00 18` | Vibrato Depth | 7 | 162 | 20.2 | 0–127 | raw | 64 |
| `00 19` | Vibrato Delay | 7 | 169 | 21.1 | 0–127 | raw | 64 |
| `00 1A` | Octave Shift | 3 | 176 | 22.0 | 61–67 | bias | 64 |
| `00 1B` | Category | 7 | 179 | 22.3 | 0–127 | raw | 0 |
| `00 1C` | Phrase Number | 8 | 186 | 23.2 | 0–255 | 2 nibbles, MSB first | 0 |
| `00 1E` | Phrase Octave Shift | 3 | 194 | 24.2 | 61–67 | bias | 64 |
| `00 1F` | TFX Switch | 1 | 197 | 24.5 | 0–1 | raw | 1 |
| `00 20` | Inst Variation | 7 | 198 | 24.6 | 0–127 | raw | 64 |
| `00 21` | Inst Number | 7 | 205 | 25.5 | 0–127 | raw | 0 |
| `00 22` | Modify Parameter 1 | 7 | 212 | 26.4 | 0–127 | raw | 64 |
| `00 23` | Modify Parameter 2 | 7 | 219 | 27.3 | 0–127 | raw | 64 |
| `00 24` | Modify Parameter 3 | 7 | 226 | 28.2 | 0–127 | raw | 64 |
| `00 25` | Modify Parameter 4 | 7 | 233 | 29.1 | 0–127 | raw | 63 |
| `00 26` | Modify Parameter 5 | 7 | 240 | 30.0 | 0–127 | raw | 0 |
| `00 27` | Modify Parameter 6 | 7 | 247 | 30.7 | 0–127 | raw | 64 |
| `00 28` | Modify Parameter 7 | 7 | 254 | 31.6 | 0–127 | raw | 0 |
| `00 29` | Modify Parameter 8 | 7 | 261 | 32.5 | 0–127 | raw | 0 |
| `00 2A` | Modify Parameter 9 | 7 | 268 | 33.4 | 0–127 | raw | 0 |
| `00 2B` | Modify Parameter 10 | 7 | 275 | 34.3 | 0–127 | raw | 0 |
| `00 2C` | Modify Parameter 11 | 7 | 282 | 35.2 | 0–127 | raw | 0 |
| `00 2D` | Modify Parameter 12 | 7 | 289 | 36.1 | 0–127 | raw | 0 |
| `00 2E` | Modify Parameter 13 | 7 | 296 | 37.0 | 0–127 | raw | 0 |
| `00 2F` | Modify Parameter 14 | 7 | 303 | 37.7 | 0–127 | raw | 0 |
| `00 30` | Modify Parameter 15 | 7 | 310 | 38.6 | 0–127 | raw | 0 |
| `00 31` | Modify Parameter 16 | 7 | 317 | 39.5 | 0–127 | raw | 0 |
| `00 32` | Modify Parameter 17 | 7 | 324 | 40.4 | 0–127 | raw | 0 |
| `00 33` | Modify Parameter 18 | 7 | 331 | 41.3 | 0–127 | raw | 0 |
| `00 34` | Modify Parameter 19 | 7 | 338 | 42.2 | 0–127 | raw | 0 |
| `00 35` | Modify Parameter 20 | 7 | 345 | 43.1 | 0–127 | raw | 0 |
| `00 36` | Modify Parameter 21 | 7 | 352 | 44.0 | 0–127 | raw | 0 |
| `00 37` | Modify Parameter 22 | 7 | 359 | 44.7 | 0–127 | raw | 0 |
| `00 38` | Modify Parameter 23 | 7 | 366 | 45.6 | 0–127 | raw | 0 |
| `00 39` | Modify Parameter 24 | 7 | 373 | 46.5 | 0–127 | raw | 0 |
| `00 3A` | Modify Parameter 25 | 7 | 380 | 47.4 | 0–127 | raw | 0 |
| `00 3B` | Modify Parameter 26 | 7 | 387 | 48.3 | 0–127 | raw | 0 |
| `00 3C` | Modify Parameter 27 | 7 | 394 | 49.2 | 0–127 | raw | 0 |
| `00 3D` | Modify Parameter 28 | 7 | 401 | 50.1 | 0–127 | raw | 0 |
| `00 3E` | Modify Parameter 29 | 7 | 408 | 51.0 | 0–127 | raw | 0 |
| `00 3F` | Modify Parameter 30 | 7 | 415 | 51.7 | 0–127 | raw | 0 |
| `00 40` | Modify Parameter 31 | 7 | 422 | 52.6 | 0–127 | raw | 0 |
| `00 41` | Modify Parameter 32 | 7 | 429 | 53.5 | 0–127 | raw | 0 |
| `00 42` | (reserve) | 7 | 436 | 54.4 | – | raw (reserve) | 0 |
| `00 43` | (reserve) | 7 | 443 | 55.3 | – | raw (reserve) | 0 |
| `00 44` | (reserve) | 7 | 450 | 56.2 | – | raw (reserve) | 0 |
| `00 45` | (reserve) | 7 | 457 | 57.1 | – | raw (reserve) | 0 |

"bias" means `sysex = svd + 64 − 2^(bits−1)`. See
[Signed Parameter Bias](02-encoding.md#signed-parameter-bias).

## MFX (entry bytes 60–137)

Same 618-bit layout as every other tone type. See
[PCM Synth Tone, Common MFX](06-pcm-synth.md#common-mfx-entry-bytes-56133).

## Evidence

- **MFX position**: the MFX header pattern `01 FC 00 00 04 …` sits at byte 60. It
  is byte-identical to the one at SN Synth byte 30 and SN Drum byte 18.
- **Common alignment**, from INIT data only:
  - Octave Shift = 4 (SysEx 64) at bit 176 and Phrase Octave Shift = 4 at bit 194.
    This fixes Category (7 bits) + Phrase Number (2 nibbles = 8 bits) at 15 bits.
  - TFX Switch = 1 at bit 197.
  - Modify Parameters 1–6 = 64, 64, 64, 63, 0, 64. These are the Concert Grand
    modify defaults (Stereo Width max 63, Nuance TYPE1).
  - Inst Variation = 64 and Inst Number = 0.
- **No end marker**: byte 137 is `00` in every entry.

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| Everything beyond INIT values (only 3 distinct entries exist, all INIT) | Save a few SN Acoustic tones with non-default Category, Phrase Number and Modify values, back up, decode with the layout above, and compare against RQ1 `19 02 00 00` (Common) and `19 02 02 00` (MFX). |
