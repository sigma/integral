# PCM Synth Tone — SVD Mapping (RFPa)

Chunk type: `RFPa` | Entry size: **590 bytes** | Model: `MI69` |
SysEx block: PCM Synth Tone (`19 00 00 00` for the Part 1 Temporary Tone)

**Confidence: verified on real data, offline (high).** The layout decodes all
4,611 occurrences in the 18 official packs (18 × 256 in BACKUP files plus 3 in
the Children of Bodom SOUND file) with 0 range violations. Only 5 distinct
entries exist: 2 INIT variants and 3 real tones (`Full Stops`, `D-50 Stack`,
`Retro Lead`, Children of Bodom slots 1–3). No entry has been compared with an
RQ1 read from a device yet.

Parameter lists, bit widths and ranges come from the official INTEGRA-7 MIDI
Implementation v1.00, **including the `(reserve)` rows** that
[`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md) omits. The
reserves occupy bits in the SVD stream, so they matter.

## Entry Layout

| Byte Range | Section      | SysEx block | Bits used | Bytes |
|------------|--------------|-------------|-----------|-------|
| 0–55       | Common       | `00 00 00`  | 446       | 56    |
| 56–133     | Common MFX   | `00 02 00`  | 618       | 78    |
| 134–165    | PMT          | `00 10 00`  | 242       | **32** (1 extra pad byte) |
| 166–264    | Partial 1    | `00 20 00`  | 790       | 99    |
| 265–363    | Partial 2    | `00 22 00`  | 790       | 99    |
| 364–462    | Partial 3    | `00 24 00`  | 790       | 99    |
| 463–561    | Partial 4    | `00 26 00`  | 790       | 99    |
| 562–587    | Common 2     | `00 30 00`  | 206       | 26    |
| 588–589    | Zero padding | —           | —         | 2     |
| **Total**  |              |             |           | **590** |

There is **no** `0x0E` end marker. Section padding and the trailing bytes are 0
in every entry.

Like SN Synth Common and Partials, PMT stores one more byte than
`ceil(bits / 8)`. Section sizes have to be tabulated per type. See
[Section Alignment](02-encoding.md#section-alignment).

## Names and Category

- **Name**: Common `00 00`–`00 0B`, 12 × 7-bit ASCII at entry bit 0.
- **Long name**: Common 2 `00 00`–`00 0F` (documented as reserve), 16 × 7-bit
  ASCII at byte 562. It holds the name padded to 16 characters
  (`INITIAL TONE    ` for INIT).
- **Category**: Common 2 `00 10` uses the INTEGRA category numbering shared with
  SN Synth Common `00 36` (see `docs/sounds/categories.json`). The values observed
  are 6 = E.ORG, 37 = BELLPAD and 34 = Syn.LD, which match the Children of Bodom
  sound list ("Organ", "Synth Bellpad", "Synth Lead").
- Common `00 0C` (reserve, 7 bits) also carries a category, apparently from the
  legacy XV/Fantom list: 6 = ORG, 4 = BEL, 22 = HLD, and 0 for INIT. **Inferred**
  from 3 samples only.

## Bias

The bias rule from [Signed Parameter Bias](02-encoding.md#signed-parameter-bias)
applies to a single-byte parameter when `bits < 7` and `range.hi >= 2^bits`:
`sysex = svd + 64 − 2^(bits−1)`. The "SVD→SysEx" column gives the offset to
add. Without the bias, every biased value is out of range (193,662 of 193,662).

## Undocumented and Reserve Fields with Content

These fields must round-trip verbatim.

| Where                                   | Bits    | Observed |
|-----------------------------------------|---------|----------|
| Common `00 0C`                          | 7       | legacy category (0/6/4/22) |
| Common `00 1F` (#, 2 nibbles)           | 8       | 0 (INIT), 100 (Full Stops), 120 (D-50 Stack, Retro Lead) |
| Common `00 27`                          | 4       | always 13 |
| MFX `00 01`                             | 7       | always 127 |
| Partial `00 0D`, `00 0E`                | 7, 7    | 0 in INIT; 53/60/72/127 in real tones |
| Partial `00 11`                         | 4       | 0 or 1 |
| Common 2 `00 00`–`00 0F`                | 16 × 7  | 16-character long name |

## Common (entry bytes 0–55)

"Section bit" is relative to the section start. Add the section's byte offset
× 8 to get the entry bit.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | PCM Synth Tone Name 1 | 7 | 0 | 32–127 |  |  |
| `00 01` | PCM Synth Tone Name 2 | 7 | 7 | 32–127 |  |  |
| `00 02` | PCM Synth Tone Name 3 | 7 | 14 | 32–127 |  |  |
| `00 03` | PCM Synth Tone Name 4 | 7 | 21 | 32–127 |  |  |
| `00 04` | PCM Synth Tone Name 5 | 7 | 28 | 32–127 |  |  |
| `00 05` | PCM Synth Tone Name 6 | 7 | 35 | 32–127 |  |  |
| `00 06` | PCM Synth Tone Name 7 | 7 | 42 | 32–127 |  |  |
| `00 07` | PCM Synth Tone Name 8 | 7 | 49 | 32–127 |  |  |
| `00 08` | PCM Synth Tone Name 9 | 7 | 56 | 32–127 |  |  |
| `00 09` | PCM Synth Tone Name 10 | 7 | 63 | 32–127 |  |  |
| `00 0A` | PCM Synth Tone Name 11 | 7 | 70 | 32–127 |  |  |
| `00 0B` | PCM Synth Tone Name 12 | 7 | 77 | 32–127 |  |  |
| `00 0C` | (reserve) | 7 | 84 |  |  | **undocumented**: legacy XV-style category (6=ORG, 4=BEL, 22=HLD observed) |
| `00 0D` | (reserve) | 1 | 91 |  |  |  |
| `00 0E` | PCM Synth Tone Level | 7 | 92 | 0–127 |  |  |
| `00 0F` | PCM Synth Tone Pan | 7 | 99 | 0–127 |  |  |
| `00 10` | PCM Synth Tone Priority | 1 | 106 | 0–1 |  |  |
| `00 11` | PCM Synth Tone Coarse Tune | 7 | 107 | 16–112 |  |  |
| `00 12` | PCM Synth Tone Fine Tune | 7 | 114 | 14–114 |  |  |
| `00 13` | Octave Shift | 3 | 121 | 61–67 | +60 |  |
| `00 14` | Stretch Tune Depth | 2 | 124 | 0–3 |  |  |
| `00 15` | Analog Feel | 7 | 126 | 0–127 |  |  |
| `00 16` | Mono/Poly | 1 | 133 | 0–1 |  |  |
| `00 17` | Legato Switch | 1 | 134 | 0–1 |  |  |
| `00 18` | Legato Retrigger | 1 | 135 | 0–1 |  |  |
| `00 19` | Portamento Switch | 1 | 136 | 0–1 |  |  |
| `00 1A` | Portamento Mode | 1 | 137 | 0–1 |  |  |
| `00 1B` | Portamento Type | 1 | 138 | 0–1 |  |  |
| `00 1C` | Portamento Start | 1 | 139 | 0–1 |  |  |
| `00 1D` | Portamento Time | 7 | 140 | 0–127 |  |  |
| `00 1E` | (reserve) | 1 | 147 |  |  |  |
| `00 1F` | (reserve) | 4+4 | 148 |  |  | **undocumented** 8-bit value: 0 (INIT), 100, 120 observed |
| `00 21` | (reserve) | 1 | 156 |  |  |  |
| `00 22` | Cutoff Offset | 7 | 157 | 1–127 |  |  |
| `00 23` | Resonance Offset | 7 | 164 | 1–127 |  |  |
| `00 24` | Attack Time Offset | 7 | 171 | 1–127 |  |  |
| `00 25` | Release Time Offset | 7 | 178 | 1–127 |  |  |
| `00 26` | Velocity Sens Offset | 7 | 185 | 1–127 |  |  |
| `00 27` | (reserve) | 4 | 192 |  |  | always 13 in all entries |
| `00 28` | PMT Control Switch | 1 | 196 | 0–1 |  |  |
| `00 29` | Pitch Bend Range Up | 6 | 197 | 0–48 |  |  |
| `00 2A` | Pitch Bend Range Down | 6 | 203 | 0–48 |  |  |
| `00 2B` | Matrix Control 1 Source | 7 | 209 | 0–109 |  |  |
| `00 2C` | Matrix Control 1 Destination 1 | 6 | 216 | 0–33 |  |  |
| `00 2D` | Matrix Control 1 Sens 1 | 7 | 222 | 1–127 |  |  |
| `00 2E` | Matrix Control 1 Destination 2 | 6 | 229 | 0–33 |  |  |
| `00 2F` | Matrix Control 1 Sens 2 | 7 | 235 | 1–127 |  |  |
| `00 30` | Matrix Control 1 Destination 3 | 6 | 242 | 0–33 |  |  |
| `00 31` | Matrix Control 1 Sens 3 | 7 | 248 | 1–127 |  |  |
| `00 32` | Matrix Control 1 Destination 4 | 6 | 255 | 0–33 |  |  |
| `00 33` | Matrix Control 1 Sens 4 | 7 | 261 | 1–127 |  |  |
| `00 34` | Matrix Control 2 Source | 7 | 268 | 0–109 |  |  |
| `00 35` | Matrix Control 2 Destination 1 | 6 | 275 | 0–33 |  |  |
| `00 36` | Matrix Control 2 Sens 1 | 7 | 281 | 1–127 |  |  |
| `00 37` | Matrix Control 2 Destination 2 | 6 | 288 | 0–33 |  |  |
| `00 38` | Matrix Control 2 Sens 2 | 7 | 294 | 1–127 |  |  |
| `00 39` | Matrix Control 2 Destination 3 | 6 | 301 | 0–33 |  |  |
| `00 3A` | Matrix Control 2 Sens 3 | 7 | 307 | 1–127 |  |  |
| `00 3B` | Matrix Control 2 Destination 4 | 6 | 314 | 0–33 |  |  |
| `00 3C` | Matrix Control 2 Sens 4 | 7 | 320 | 1–127 |  |  |
| `00 3D` | Matrix Control 3 Source | 7 | 327 | 0–109 |  |  |
| `00 3E` | Matrix Control 3 Destination 1 | 6 | 334 | 0–33 |  |  |
| `00 3F` | Matrix Control 3 Sens 1 | 7 | 340 | 1–127 |  |  |
| `00 40` | Matrix Control 3 Destination 2 | 6 | 347 | 0–33 |  |  |
| `00 41` | Matrix Control 3 Sens 2 | 7 | 353 | 1–127 |  |  |
| `00 42` | Matrix Control 3 Destination 3 | 6 | 360 | 0–33 |  |  |
| `00 43` | Matrix Control 3 Sens 3 | 7 | 366 | 1–127 |  |  |
| `00 44` | Matrix Control 3 Destination 4 | 6 | 373 | 0–33 |  |  |
| `00 45` | Matrix Control 3 Sens 4 | 7 | 379 | 1–127 |  |  |
| `00 46` | Matrix Control 4 Source | 7 | 386 | 0–109 |  |  |
| `00 47` | Matrix Control 4 Destination 1 | 6 | 393 | 0–33 |  |  |
| `00 48` | Matrix Control 4 Sens 1 | 7 | 399 | 1–127 |  |  |
| `00 49` | Matrix Control 4 Destination 2 | 6 | 406 | 0–33 |  |  |
| `00 4A` | Matrix Control 4 Sens 2 | 7 | 412 | 1–127 |  |  |
| `00 4B` | Matrix Control 4 Destination 3 | 6 | 419 | 0–33 |  |  |
| `00 4C` | Matrix Control 4 Sens 3 | 7 | 425 | 1–127 |  |  |
| `00 4D` | Matrix Control 4 Destination 4 | 6 | 432 | 0–33 |  |  |
| `00 4E` | Matrix Control 4 Sens 4 | 7 | 438 | 1–127 |  |  |
| `00 4F` | (reserve) | 1 | 445 |  |  |  |

## Common MFX (entry bytes 56–133)

This MFX section is bit-identical in every tone type: SN Synth (byte 30), SN
Acoustic (byte 60), SN Drum (byte 18), PCM Synth (byte 56) and PCM Drum
(byte 14). The other pages link here rather than repeat it.

MFX Parameter n (1–32) sits at section bit `106 + 16(n−1)` and SysEx offset
`00 11 + 4(n−1)`, with 7-bit address carry.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | MFX Type | 7 | 0 | 0–67 |  |  |
| `00 01` | (reserve) | 7 | 7 |  |  | always 127 |
| `00 02` | MFX Chorus Send Level | 7 | 14 | 0–127 |  |  |
| `00 03` | MFX Reverb Send Level | 7 | 21 | 0–127 |  |  |
| `00 04` | (reserve) | 2 | 28 |  |  |  |
| `00 05` | MFX Control 1 Source | 7 | 30 | 0–101 |  |  |
| `00 06` | MFX Control 1 Sens | 7 | 37 | 1–127 |  |  |
| `00 07` | MFX Control 2 Source | 7 | 44 | 0–101 |  |  |
| `00 08` | MFX Control 2 Sens | 7 | 51 | 1–127 |  |  |
| `00 09` | MFX Control 3 Source | 7 | 58 | 0–101 |  |  |
| `00 0A` | MFX Control 3 Sens | 7 | 65 | 1–127 |  |  |
| `00 0B` | MFX Control 4 Source | 7 | 72 | 0–101 |  |  |
| `00 0C` | MFX Control 4 Sens | 7 | 79 | 1–127 |  |  |
| `00 0D` | MFX Control Assign 1 | 5 | 86 | 0–16 |  |  |
| `00 0E` | MFX Control Assign 2 | 5 | 91 | 0–16 |  |  |
| `00 0F` | MFX Control Assign 3 | 5 | 96 | 0–16 |  |  |
| `00 10` | MFX Control Assign 4 | 5 | 101 | 0–16 |  |  |
| `00 11` | MFX Parameter 1 | 4+4+4+4 | 106 | 12768–52768 |  |  |
| `00 15` | MFX Parameter 2 | 4+4+4+4 | 122 | 12768–52768 |  |  |
| `00 19` | MFX Parameter 3 | 4+4+4+4 | 138 | 12768–52768 |  |  |
| `00 1D` | MFX Parameter 4 | 4+4+4+4 | 154 | 12768–52768 |  |  |
| `00 21` | MFX Parameter 5 | 4+4+4+4 | 170 | 12768–52768 |  |  |
| `00 25` | MFX Parameter 6 | 4+4+4+4 | 186 | 12768–52768 |  |  |
| `00 29` | MFX Parameter 7 | 4+4+4+4 | 202 | 12768–52768 |  |  |
| `00 2D` | MFX Parameter 8 | 4+4+4+4 | 218 | 12768–52768 |  |  |
| `00 31` | MFX Parameter 9 | 4+4+4+4 | 234 | 12768–52768 |  |  |
| `00 35` | MFX Parameter 10 | 4+4+4+4 | 250 | 12768–52768 |  |  |
| `00 39` | MFX Parameter 11 | 4+4+4+4 | 266 | 12768–52768 |  |  |
| `00 3D` | MFX Parameter 12 | 4+4+4+4 | 282 | 12768–52768 |  |  |
| `00 41` | MFX Parameter 13 | 4+4+4+4 | 298 | 12768–52768 |  |  |
| `00 45` | MFX Parameter 14 | 4+4+4+4 | 314 | 12768–52768 |  |  |
| `00 49` | MFX Parameter 15 | 4+4+4+4 | 330 | 12768–52768 |  |  |
| `00 4D` | MFX Parameter 16 | 4+4+4+4 | 346 | 12768–52768 |  |  |
| `00 51` | MFX Parameter 17 | 4+4+4+4 | 362 | 12768–52768 |  |  |
| `00 55` | MFX Parameter 18 | 4+4+4+4 | 378 | 12768–52768 |  |  |
| `00 59` | MFX Parameter 19 | 4+4+4+4 | 394 | 12768–52768 |  |  |
| `00 5D` | MFX Parameter 20 | 4+4+4+4 | 410 | 12768–52768 |  |  |
| `00 61` | MFX Parameter 21 | 4+4+4+4 | 426 | 12768–52768 |  |  |
| `00 65` | MFX Parameter 22 | 4+4+4+4 | 442 | 12768–52768 |  |  |
| `00 69` | MFX Parameter 23 | 4+4+4+4 | 458 | 12768–52768 |  |  |
| `00 6D` | MFX Parameter 24 | 4+4+4+4 | 474 | 12768–52768 |  |  |
| `00 71` | MFX Parameter 25 | 4+4+4+4 | 490 | 12768–52768 |  |  |
| `00 75` | MFX Parameter 26 | 4+4+4+4 | 506 | 12768–52768 |  |  |
| `00 79` | MFX Parameter 27 | 4+4+4+4 | 522 | 12768–52768 |  |  |
| `00 7D` | MFX Parameter 28 | 4+4+4+4 | 538 | 12768–52768 |  |  |
| `01 01` | MFX Parameter 29 | 4+4+4+4 | 554 | 12768–52768 |  |  |
| `01 05` | MFX Parameter 30 | 4+4+4+4 | 570 | 12768–52768 |  |  |
| `01 09` | MFX Parameter 31 | 4+4+4+4 | 586 | 12768–52768 |  |  |
| `01 0D` | MFX Parameter 32 | 4+4+4+4 | 602 | 12768–52768 |  |  |

## PMT — Partial Mix Table (entry bytes 134–165)

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | Structure Type 1 & 2 | 4 | 0 | 0–9 |  |  |
| `00 01` | Booster 1 & 2 | 2 | 4 | 0–3 |  |  |
| `00 02` | Structure Type 3 & 4 | 4 | 6 | 0–9 |  |  |
| `00 03` | Booster 3 & 4 | 2 | 10 | 0–3 |  |  |
| `00 04` | PMT Velocity Control | 2 | 12 | 0–3 |  |  |
| `00 05` | PMT1 Partial Switch | 1 | 14 | 0–1 |  |  |
| `00 06` | PMT1 Keyboard Range Lower | 7 | 15 | 0–127 |  |  |
| `00 07` | PMT1 Keyboard Range Upper | 7 | 22 | 0–127 |  |  |
| `00 08` | PMT1 Keyboard Fade Width Lower | 7 | 29 | 0–127 |  |  |
| `00 09` | PMT1 Keyboard Fade Width Upper | 7 | 36 | 0–127 |  |  |
| `00 0A` | PMT1 Velocity Range Lower | 7 | 43 | 1–127 |  |  |
| `00 0B` | PMT1 Velocity Range Upper | 7 | 50 | 1–127 |  |  |
| `00 0C` | PMT1 Velocity Fade Width Lower | 7 | 57 | 0–127 |  |  |
| `00 0D` | PMT1 Velocity Fade Width Upper | 7 | 64 | 0–127 |  |  |
| `00 0E` | PMT2 Partial Switch | 1 | 71 | 0–1 |  |  |
| `00 0F` | PMT2 Keyboard Range Lower | 7 | 72 | 0–127 |  |  |
| `00 10` | PMT2 Keyboard Range Upper | 7 | 79 | 0–127 |  |  |
| `00 11` | PMT2 Keyboard Fade Width Lower | 7 | 86 | 0–127 |  |  |
| `00 12` | PMT2 Keyboard Fade Width Upper | 7 | 93 | 0–127 |  |  |
| `00 13` | PMT2 Velocity Range Lower | 7 | 100 | 1–127 |  |  |
| `00 14` | PMT2 Velocity Range Upper | 7 | 107 | 1–127 |  |  |
| `00 15` | PMT2 Velocity Fade Width Lower | 7 | 114 | 0–127 |  |  |
| `00 16` | PMT2 Velocity Fade Width Upper | 7 | 121 | 0–127 |  |  |
| `00 17` | PMT3 Partial Switch | 1 | 128 | 0–1 |  |  |
| `00 18` | PMT3 Keyboard Range Lower | 7 | 129 | 0–127 |  |  |
| `00 19` | PMT3 Keyboard Range Upper | 7 | 136 | 0–127 |  |  |
| `00 1A` | PMT3 Keyboard Fade Width Lower | 7 | 143 | 0–127 |  |  |
| `00 1B` | PMT3 Keyboard Fade Width Upper | 7 | 150 | 0–127 |  |  |
| `00 1C` | PMT3 Velocity Range Lower | 7 | 157 | 1–127 |  |  |
| `00 1D` | PMT3 Velocity Range Upper | 7 | 164 | 1–127 |  |  |
| `00 1E` | PMT3 Velocity Fade Width Lower | 7 | 171 | 0–127 |  |  |
| `00 1F` | PMT3 Velocity Fade Width Upper | 7 | 178 | 0–127 |  |  |
| `00 20` | PMT4 Partial Switch | 1 | 185 | 0–1 |  |  |
| `00 21` | PMT4 Keyboard Range Lower | 7 | 186 | 0–127 |  |  |
| `00 22` | PMT4 Keyboard Range Upper | 7 | 193 | 0–127 |  |  |
| `00 23` | PMT4 Keyboard Fade Width Lower | 7 | 200 | 0–127 |  |  |
| `00 24` | PMT4 Keyboard Fade Width Upper | 7 | 207 | 0–127 |  |  |
| `00 25` | PMT4 Velocity Range Lower | 7 | 214 | 1–127 |  |  |
| `00 26` | PMT4 Velocity Range Upper | 7 | 221 | 1–127 |  |  |
| `00 27` | PMT4 Velocity Fade Width Lower | 7 | 228 | 0–127 |  |  |
| `00 28` | PMT4 Velocity Fade Width Upper | 7 | 235 | 0–127 |  |  |

## Partial (99 bytes each, entry byte `166 + 99(n−1)`)

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | Partial Level | 7 | 0 | 0–127 |  |  |
| `00 01` | Partial Coarse Tune | 7 | 7 | 16–112 |  |  |
| `00 02` | Partial Fine Tune | 7 | 14 | 14–114 |  |  |
| `00 03` | Partial Random Pitch Depth | 5 | 21 | 0–30 |  |  |
| `00 04` | Partial Pan | 7 | 26 | 0–127 |  |  |
| `00 05` | Partial Pan Keyfollow | 5 | 33 | 54–74 | +48 |  |
| `00 06` | Partial Random Pan Depth | 6 | 38 | 0–63 |  |  |
| `00 07` | Partial Alternate Pan Depth | 7 | 44 | 1–127 |  |  |
| `00 08` | Partial Env Mode | 1 | 51 | 0–1 |  |  |
| `00 09` | Partial Delay Mode | 2 | 52 | 0–3 |  |  |
| `00 0A` | Partial Delay Time | 4+4 | 54 | 0–149 |  |  |
| `00 0C` | Partial Output Level | 7 | 62 | 0–127 |  |  |
| `00 0D` | (reserve) | 7 | 69 |  |  | non-zero in real tones (0/53/60/127) |
| `00 0E` | (reserve) | 7 | 76 |  |  | non-zero in real tones (0/72/127) |
| `00 0F` | Partial Chorus Send Level | 7 | 83 | 0–127 |  |  |
| `00 10` | Partial Reverb Send Level | 7 | 90 | 0–127 |  |  |
| `00 11` | (reserve) | 4 | 97 |  |  | 0 or 1 observed |
| `00 12` | Partial Receive Bender | 1 | 101 | 0–1 |  |  |
| `00 13` | Partial Receive Expression | 1 | 102 | 0–1 |  |  |
| `00 14` | Partial Receive Hold-1 | 1 | 103 | 0–1 |  |  |
| `00 15` | (reserve) | 1 | 104 |  |  |  |
| `00 16` | Partial Redamper Switch | 1 | 105 | 0–1 |  |  |
| `00 17` | Partial Control 1 Switch 1 | 2 | 106 | 0–2 |  |  |
| `00 18` | Partial Control 1 Switch 2 | 2 | 108 | 0–2 |  |  |
| `00 19` | Partial Control 1 Switch 3 | 2 | 110 | 0–2 |  |  |
| `00 1A` | Partial Control 1 Switch 4 | 2 | 112 | 0–2 |  |  |
| `00 1B` | Partial Control 2 Switch 1 | 2 | 114 | 0–2 |  |  |
| `00 1C` | Partial Control 2 Switch 2 | 2 | 116 | 0–2 |  |  |
| `00 1D` | Partial Control 2 Switch 3 | 2 | 118 | 0–2 |  |  |
| `00 1E` | Partial Control 2 Switch 4 | 2 | 120 | 0–2 |  |  |
| `00 1F` | Partial Control 3 Switch 1 | 2 | 122 | 0–2 |  |  |
| `00 20` | Partial Control 3 Switch 2 | 2 | 124 | 0–2 |  |  |
| `00 21` | Partial Control 3 Switch 3 | 2 | 126 | 0–2 |  |  |
| `00 22` | Partial Control 3 Switch 4 | 2 | 128 | 0–2 |  |  |
| `00 23` | Partial Control 4 Switch 1 | 2 | 130 | 0–2 |  |  |
| `00 24` | Partial Control 4 Switch 2 | 2 | 132 | 0–2 |  |  |
| `00 25` | Partial Control 4 Switch 3 | 2 | 134 | 0–2 |  |  |
| `00 26` | Partial Control 4 Switch 4 | 2 | 136 | 0–2 |  |  |
| `00 27` | Wave Group Type | 2 | 138 | 0–3 |  |  |
| `00 28` | Wave Group ID | 4+4+4+4 | 140 | 0–16384 |  |  |
| `00 2C` | Wave Number L (Mono) | 4+4+4+4 | 156 | 0–16384 |  |  |
| `00 30` | Wave Number R | 4+4+4+4 | 172 | 0–16384 |  |  |
| `00 34` | Wave Gain | 2 | 188 | 0–3 |  |  |
| `00 35` | Wave FXM Switch | 1 | 190 | 0–1 |  |  |
| `00 36` | Wave FXM Color | 2 | 191 | 0–3 |  |  |
| `00 37` | Wave FXM Depth | 5 | 193 | 0–16 |  |  |
| `00 38` | Wave Tempo Sync | 1 | 198 | 0–1 |  |  |
| `00 39` | Wave Pitch Keyfollow | 6 | 199 | 44–84 | +32 |  |
| `00 3A` | Pitch Env Depth | 5 | 205 | 52–76 | +48 |  |
| `00 3B` | Pitch Env Velocity Sens | 7 | 210 | 1–127 |  |  |
| `00 3C` | Pitch Env Time 1 Velocity Sens | 7 | 217 | 1–127 |  |  |
| `00 3D` | Pitch Env Time 4 Velocity Sens | 7 | 224 | 1–127 |  |  |
| `00 3E` | Pitch Env Time Keyfollow | 5 | 231 | 54–74 | +48 |  |
| `00 3F` | Pitch Env Time 1 | 7 | 236 | 0–127 |  |  |
| `00 40` | Pitch Env Time 2 | 7 | 243 | 0–127 |  |  |
| `00 41` | Pitch Env Time 3 | 7 | 250 | 0–127 |  |  |
| `00 42` | Pitch Env Time 4 | 7 | 257 | 0–127 |  |  |
| `00 43` | Pitch Env Level 0 | 7 | 264 | 1–127 |  |  |
| `00 44` | Pitch Env Level 1 | 7 | 271 | 1–127 |  |  |
| `00 45` | Pitch Env Level 2 | 7 | 278 | 1–127 |  |  |
| `00 46` | Pitch Env Level 3 | 7 | 285 | 1–127 |  |  |
| `00 47` | Pitch Env Level 4 | 7 | 292 | 1–127 |  |  |
| `00 48` | TVF Filter Type | 3 | 299 | 0–6 |  |  |
| `00 49` | TVF Cutoff Frequency | 7 | 302 | 0–127 |  |  |
| `00 4A` | TVF Cutoff Keyfollow | 6 | 309 | 44–84 | +32 |  |
| `00 4B` | TVF Cutoff Velocity Curve | 3 | 315 | 0–7 |  |  |
| `00 4C` | TVF Cutoff Velocity Sens | 7 | 318 | 1–127 |  |  |
| `00 4D` | TVF Resonance | 7 | 325 | 0–127 |  |  |
| `00 4E` | TVF Resonance Velocity Sens | 7 | 332 | 1–127 |  |  |
| `00 4F` | TVF Env Depth | 7 | 339 | 1–127 |  |  |
| `00 50` | TVF Env Velocity Curve | 3 | 346 | 0–7 |  |  |
| `00 51` | TVF Env Velocity Sens | 7 | 349 | 1–127 |  |  |
| `00 52` | TVF Env Time 1 Velocity Sens | 7 | 356 | 1–127 |  |  |
| `00 53` | TVF Env Time 4 Velocity Sens | 7 | 363 | 1–127 |  |  |
| `00 54` | TVF Env Time Keyfollow | 5 | 370 | 54–74 | +48 |  |
| `00 55` | TVF Env Time 1 | 7 | 375 | 0–127 |  |  |
| `00 56` | TVF Env Time 2 | 7 | 382 | 0–127 |  |  |
| `00 57` | TVF Env Time 3 | 7 | 389 | 0–127 |  |  |
| `00 58` | TVF Env Time 4 | 7 | 396 | 0–127 |  |  |
| `00 59` | TVF Env Level 0 | 7 | 403 | 0–127 |  |  |
| `00 5A` | TVF Env Level 1 | 7 | 410 | 0–127 |  |  |
| `00 5B` | TVF Env Level 2 | 7 | 417 | 0–127 |  |  |
| `00 5C` | TVF Env Level 3 | 7 | 424 | 0–127 |  |  |
| `00 5D` | TVF Env Level 4 | 7 | 431 | 0–127 |  |  |
| `00 5E` | Bias Level | 5 | 438 | 54–74 | +48 |  |
| `00 5F` | Bias Position | 7 | 443 | 0–127 |  |  |
| `00 60` | Bias Direction | 2 | 450 | 0–3 |  |  |
| `00 61` | TVA Level Velocity Curve | 3 | 452 | 0–7 |  |  |
| `00 62` | TVA Level Velocity Sens | 7 | 455 | 1–127 |  |  |
| `00 63` | TVA Env Time 1 Velocity Sens | 7 | 462 | 1–127 |  |  |
| `00 64` | TVA Env Time 4 Velocity Sens | 7 | 469 | 1–127 |  |  |
| `00 65` | TVA Env Time Keyfollow | 5 | 476 | 54–74 | +48 |  |
| `00 66` | TVA Env Time 1 | 7 | 481 | 0–127 |  |  |
| `00 67` | TVA Env Time 2 | 7 | 488 | 0–127 |  |  |
| `00 68` | TVA Env Time 3 | 7 | 495 | 0–127 |  |  |
| `00 69` | TVA Env Time 4 | 7 | 502 | 0–127 |  |  |
| `00 6A` | TVA Env Level 1 | 7 | 509 | 0–127 |  |  |
| `00 6B` | TVA Env Level 2 | 7 | 516 | 0–127 |  |  |
| `00 6C` | TVA Env Level 3 | 7 | 523 | 0–127 |  |  |
| `00 6D` | LFO1 Waveform | 4 | 530 | 0–12 |  |  |
| `00 6E` | LFO1 Rate | 4+4 | 534 | 0–149 |  |  |
| `00 70` | LFO1 Offset | 3 | 542 | 0–4 |  |  |
| `00 71` | LFO1 Rate Detune | 7 | 545 | 0–127 |  |  |
| `00 72` | LFO1 Delay Time | 7 | 552 | 0–127 |  |  |
| `00 73` | LFO1 Delay Time Keyfollow | 5 | 559 | 54–74 | +48 |  |
| `00 74` | LFO1 Fade Mode | 2 | 564 | 0–3 |  |  |
| `00 75` | LFO1 Fade Time | 7 | 566 | 0–127 |  |  |
| `00 76` | LFO1 Key Trigger | 1 | 573 | 0–1 |  |  |
| `00 77` | LFO1 Pitch Depth | 7 | 574 | 1–127 |  |  |
| `00 78` | LFO1 TVF Depth | 7 | 581 | 1–127 |  |  |
| `00 79` | LFO1 TVA Depth | 7 | 588 | 1–127 |  |  |
| `00 7A` | LFO1 Pan Depth | 7 | 595 | 1–127 |  |  |
| `00 7B` | LFO2 Waveform | 4 | 602 | 0–12 |  |  |
| `00 7C` | LFO2 Rate | 4+4 | 606 | 0–149 |  |  |
| `00 7E` | LFO2 Offset | 3 | 614 | 0–4 |  |  |
| `00 7F` | LFO2 Rate Detune | 7 | 617 | 0–127 |  |  |
| `01 00` | LFO2 Delay Time | 7 | 624 | 0–127 |  |  |
| `01 01` | LFO2 Delay Time Keyfollow | 5 | 631 | 54–74 | +48 |  |
| `01 02` | LFO2 Fade Mode | 2 | 636 | 0–3 |  |  |
| `01 03` | LFO2 Fade Time | 7 | 638 | 0–127 |  |  |
| `01 04` | LFO2 Key Trigger | 1 | 645 | 0–1 |  |  |
| `01 05` | LFO2 Pitch Depth | 7 | 646 | 1–127 |  |  |
| `01 06` | LFO2 TVF Depth | 7 | 653 | 1–127 |  |  |
| `01 07` | LFO2 TVA Depth | 7 | 660 | 1–127 |  |  |
| `01 08` | LFO2 Pan Depth | 7 | 667 | 1–127 |  |  |
| `01 09` | LFO Step Type | 4 | 674 | 0–1 |  |  |
| `01 0A` | LFO Step1 | 7 | 678 | 28–100 |  |  |
| `01 0B` | LFO Step2 | 7 | 685 | 28–100 |  |  |
| `01 0C` | LFO Step3 | 7 | 692 | 28–100 |  |  |
| `01 0D` | LFO Step4 | 7 | 699 | 28–100 |  |  |
| `01 0E` | LFO Step5 | 7 | 706 | 28–100 |  |  |
| `01 0F` | LFO Step6 | 7 | 713 | 28–100 |  |  |
| `01 10` | LFO Step7 | 7 | 720 | 28–100 |  |  |
| `01 11` | LFO Step8 | 7 | 727 | 28–100 |  |  |
| `01 12` | LFO Step9 | 7 | 734 | 28–100 |  |  |
| `01 13` | LFO Step10 | 7 | 741 | 28–100 |  |  |
| `01 14` | LFO Step11 | 7 | 748 | 28–100 |  |  |
| `01 15` | LFO Step12 | 7 | 755 | 28–100 |  |  |
| `01 16` | LFO Step13 | 7 | 762 | 28–100 |  |  |
| `01 17` | LFO Step14 | 7 | 769 | 28–100 |  |  |
| `01 18` | LFO Step15 | 7 | 776 | 28–100 |  |  |
| `01 19` | LFO Step16 | 7 | 783 | 28–100 |  |  |

## Common 2 (entry bytes 562–587)

`00 00`–`00 0F` hold the 16-character long name.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | (reserve) | 7 | 0 |  |  | 00 00-00 0F: **16-char long name**, 7-bit ASCII ("INITIAL TONE    ") |
| `00 01`–`00 0E` | (reserve) × 14 | 7 each | 7–104 |  |  | same as the row above |
| `00 0F` | (reserve) | 7 | 105 |  |  |  |
| `00 10` | Tone Category | 7 | 112 | 0–127 |  |  |
| `00 11` | (unnamed) | 4+4 | 119 | 0–255 |  |  |
| `00 13` | Phrase Octave Shift | 3 | 127 | 61–67 | +60 |  |
| `00 14` | (reserve) | 1 | 130 |  |  |  |
| `00 15`–`00 31` | (reserve) × 29 | 1 each | 131–159 |  |  | same as the row above |
| `00 32` | (reserve) | 1 | 160 |  |  |  |
| `00 33` | TFX Switch | 1 | 161 | 0–1 |  |  |
| `00 34` | (reserve) | 7 | 162 |  |  |  |
| `00 35` | (reserve) | 7 | 169 |  |  |  |
| `00 36` | (reserve) | 7 | 176 |  |  |  |
| `00 37` | (reserve) | 7 | 183 |  |  |  |
| `00 38` | Phrase Number | 4+4+4+4 | 190 | 0–65535 |  |  |

## Evidence

Each layout hypothesis was range-checked over every documented parameter in every
occurrence, with the bias rule applied. Reserves are not range-checked.

| Hypothesis | Values checked | Violations |
|------------|----------------|------------|
| **Layout above** | 3,306,087 | **0** |
| PMT unpadded (31 B), partials at `165 + 99n` | 3,306,087 | 318,085 |
| Common + MFX bit-concatenated | 3,306,087 | 147,510 |
| No section alignment at all | 3,306,087 | 839,170 |
| MFX start at 55 / 57 / 58 instead of 56 | — | 162 / 151 / 27 unique-entry violations |
| PMT start at 133 / 135 instead of 134 | — | 39 / 19 |

Other checks:

- All 5 distinct entries decode names that match the sound lists. `Retro Laed` in
  the Children of Bodom sound list is a typo for `Retro Lead`.
- The real tones are self-consistent. Every partial whose PMT switch is ON has
  Wave Group ID = 1 and a non-zero Wave Number L (Full Stops: 89/90; D-50 Stack:
  64/424/435/448; Retro Lead: 543/431). Every partial switched OFF has wave 0.
  The MFX types are 19, 22 and 36.
- INIT defaults look plausible: Tone Level 127; Pan, Coarse and Fine at 64;
  Mono/Poly = POLY; Bend 2/2; Matrix sources CTRL1–4 (98–101); PMT1 ON with key
  range 0–127 and velocity 1–127, PMT2–4 OFF; Partial Level 127, Cutoff 127, LFO
  steps 64.
- The two INIT variants differ only in Partial 1–4 Wave Group Type (0 versus 1).

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| PMT pad byte position, and reserve widths *inside* a section (two wrong reserve widths that cancel out would go unnoticed between full-range fields) | Import the Children of Bodom SOUND file into User PCM Synth tones. RQ1 each Temporary Tone block (Common `00 50`, MFX `01 11`, PMT `00 29`, Partials `01 1A`, Common 2 `00 3C`) and diff it against the decode, reserve bytes included. |
| Meaning of Common `00 0C`, `00 1F`, `00 27` and Partial `00 0D`/`00 0E`/`00 11` | The same RQ1 reads, plus edits on the device that change one parameter at a time. |
