# Studio Set — SVD Mapping (PRFb)

Chunk type: `PRFb` | Entry size: **1068 bytes** | Model: `MI69` |
SysEx base: `18 00 00 00` (Temporary Studio Set)

**Confidence: verified on real data, offline.** Section boundaries and every
field marked *verified* below were checked against 1,482 entries from the 18
official Roland packs (18 × 64 in BACKUP files plus 330 in SOUND files). 333 of
these Studio Sets are non-INIT. No entry has been compared with an RQ1 read from
a device yet. Fields marked *inferred* hold the same value in every entry. Their
position comes from a plausible default and a tight fit with their neighbours,
but their width is **not** proven.

SysEx offsets are relative to each block's start. See
[`docs/midi/05-studio-set.md`](../midi/05-studio-set.md).

## Entry Layout

Every section is byte-aligned and packed MSB-first, as in
[Bit-Packing Encoding](02-encoding.md). There is **no** `0x0E` end marker. All
bits outside the mapped fields are 0, except Common bits 112–143 (see below).

| Byte Range        | Section            | SysEx block          | Bytes      |
|-------------------|--------------------|----------------------|------------|
| 0–60              | Common             | `00 00 00`           | 61         |
| 61–103            | Chorus             | `00 04 00`           | 43         |
| 104–153           | Reverb             | `00 06 00`           | 50         |
| 154–167           | Motional Surround  | `00 08 00`           | 14         |
| 168–171           | Master EQ          | `00 09 00`           | 4          |
| 172 + (n−1)       | MIDI, channel n    | `00 0F+n 00`         | 1 each     |
| 188 + 51(n−1)     | Part n             | `00 1F+n 00`         | 51 each    |
| 1004 + 4(n−1)     | Part EQ n          | `00 4F+n 00`         | 4 each     |
| **Total**         |                    |                      | **1068**   |

Entry bit of Part n = `1504 + 408(n−1)`. Entry bit of Part EQ n =
`8032 + 32(n−1)`.

### Where the SVD differs from SysEx order and widths

- **Common `00 10`–`00 38`** (41 reserve and Voice Reserve parameters) take
  **257 bits** (entry bits 112–368). That matches the bit masks in the MIDI
  Implementation PDF: `00 10` 7 bits, `00 11`–`00 13` 6 each, `00 14`–`00 17`
  1 each, Voice Reserve 1–16 and `00 28`–`00 37` 7 each, `00 38` 4 bits. Bits
  112–143 are always `FE 82 08 78`, which decodes as `00 10`–`00 17` = 127, 16,
  16, 16, 1, 1, 1, 1; their meaning is unknown. Bits 144–368 are always 0, so
  Tone Control 1 Source sits at bit 369.
- **Common `00 4B`** (reserve) is 1 bit. `00 50`–`00 53` take 23 zero bits,
  although the MIDI Implementation gives them 7 bits each (28).
- **Chorus header** is `[7 zero bits][Type 4][Level 7][Output 2][3 zero bits]`,
  then 20 × 16-bit parameters. The SysEx order is Type, Level, (reserve),
  Output.
- **Motional Surround** carries its 13 documented fields (77 bits), then zero
  fill to 14 bytes.
- **Part reserves `00 02`–`00 05`** are **1 bit each** and always 1, not 7 bits.
- **Part `00 29` Output Assign + `00 2A` reserve** take 6 bits together. The 4 + 2
  split is assumed because both are always 0.
- **Part reserves `00 45` and `00 47`** are 7 bits and always 64.
- **MIDI Phase Lock** is the MSB of its byte.

## Common (entry bytes 0–60)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00`–`00 0F` | Studio Set Name 1–16 | 7 each | 0 (+7 per char) | 32–127 | verified (334/334 names match) |
| `00 10`–`00 38` | (reserve) and Voice Reserve 1–16 | 7, 6 × 3, 1 × 4, 7 × 32, 4 (257 total) | 112 |  | bits 112–143 always `FE 82 08 78`; 144–368 always 0 |
| `00 39` | Tone Control 1 Source | 7 | 369 | 0–97 | inferred (always 1) |
| `00 3A` | Tone Control 2 Source | 7 | 376 | 0–97 | inferred (always 97) |
| `00 3B` | Tone Control 3 Source | 7 | 383 | 0–97 | inferred (always 2) |
| `00 3C` | Tone Control 4 Source | 7 | 390 | 0–97 | inferred (always 4) |
| `00 3D` | Studio Set Tempo (#, 2 nibbles) | 8 | 397 | 20–250 | inferred (always 120) |
| `00 3F` | Solo Part | 5 | 405 | 0–16 | inferred (always 0) |
| `00 40` | Reverb Switch | 1 | 410 | 0–1 | verified |
| `00 41` | Chorus Switch | 1 | 411 | 0–1 | verified |
| `00 42` | Master EQ Switch | 1 | 412 | 0–1 | verified |
| `00 43` | Drum Comp/EQ Switch | 1 | 413 | 0–1 | inferred (always 1) |
| `00 44` | Drum Comp/EQ Part | 4 | 414 | 0–15 | inferred (always 9) |
| `00 45` | Drum Comp/EQ 1 Output Assign | 4 | 418 | 0–12 | inferred (always 0) |
| `00 46` | Drum Comp/EQ 2 Output Assign | 4 | 422 | 0–12 | inferred (always 0) |
| `00 47` | Drum Comp/EQ 3 Output Assign | 4 | 426 | 0–12 | inferred (always 0) |
| `00 48` | Drum Comp/EQ 4 Output Assign | 4 | 430 | 0–12 | inferred (always 0) |
| `00 49` | Drum Comp/EQ 5 Output Assign | 4 | 434 | 0–12 | inferred (always 0) |
| `00 4A` | Drum Comp/EQ 6 Output Assign | 4 | 438 | 0–12 | inferred (always 0) |
| `00 4B` | (reserve) | 1 | 442 |  | inferred (always 0) |
| `00 4C` | Ext Part Level | 7 | 443 | 0–127 | inferred (always 100) |
| `00 4D` | Ext Part Chorus Send Level | 7 | 450 | 0–127 | inferred (always 0) |
| `00 4E` | Ext Part Reverb Send Level | 7 | 457 | 0–127 | inferred (always 0) |
| `00 4F` | Ext Part Mute Switch | 1 | 464 | 0–1 | inferred (always 0) |
| `00 50`–`00 53` | (reserve) | 23 total | 465 |  | always 0 |

## Chorus (entry bytes 61–103)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| — | (unknown, 7 zero bits) | 7 | 488 |  | inferred (always 0) |
| `00 00` | Chorus Type | 4 | 495 | 0–3 | verified |
| `00 01` | Chorus Level | 7 | 499 | 0–127 | verified |
| `00 03` | Chorus Output Select | 2 | 506 | 0–2 | verified |
| — | (unknown, 3 zero bits) | 3 | 508 |  | inferred (always 0) |
| `00 04` | Chorus Parameter 1 | 16 | 511 | 12768–52768 | verified |
| `00 08`–`00 50` | Chorus Parameter 2–20 | 16 each | 527 (+16 per param) | 12768–52768 | verified (P1); others by stride |

## Reverb (entry bytes 104–153)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | Reverb Type | 4 | 832 | 0–6 | verified |
| `00 01` | Reverb Level | 7 | 836 | 0–127 | verified |
| `00 02` | Reverb Output Assign | 2 | 843 | 0–3 | inferred (always 0) |
| `00 03` | Reverb Parameter 1 | 16 | 845 | 12768–52768 | verified |
| `00 07`–`00 5B` | Reverb Parameter 2–24 | 16 each | 861 (+16 per param) | 12768–52768 | verified (P1); others by stride |

## Motional Surround (entry bytes 154–167)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | Motional Surround Switch | 1 | 1232 | 0–1 | inferred (always 0) |
| `00 01` | Room Type | 2 | 1233 | 0–3 | inferred (always 2) |
| `00 02` | Ambience Level | 7 | 1235 | 0–127 | inferred (always 80) |
| `00 03` | Room Size | 7 | 1242 | 0–2 | inferred (always 1) |
| `00 04` | Ambience Time | 7 | 1249 | 0–100 | inferred (always 40) |
| `00 05` | Ambience Density | 7 | 1256 | 0–100 | inferred (always 100) |
| `00 06` | Ambience HF Damp | 7 | 1263 | 0–100 | inferred (always 50) |
| `00 07` | Ext Part L-R | 7 | 1270 | 0–127 | inferred (always 64) |
| `00 08` | Ext Part F-B | 7 | 1277 | 0–127 | inferred (always 74) |
| `00 09` | Ext Part Width | 6 | 1284 | 0–32 | inferred (always 10) |
| `00 0A` | Ext Part Ambience Send Level | 7 | 1290 | 0–127 | inferred (always 40) |
| `00 0B` | Ext Part Control Channel | 5 | 1297 | 0–16 | inferred (always 15) |
| `00 0C` | Motional Surround Depth | 7 | 1302 | 0–100 | inferred (always 50) |

## Master EQ (entry bytes 168–171)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | EQ Low Freq | 1 | 1344 | 0–1 | inferred (always 1) |
| `00 01` | EQ Low Gain | 5 | 1345 | 0–30 | verified |
| `00 02` | EQ Mid Freq | 5 | 1350 | 0–16 | verified |
| `00 03` | EQ Mid Gain | 5 | 1355 | 0–30 | verified |
| `00 04` | EQ Mid Q | 3 | 1360 | 0–4 | verified |
| `00 05` | EQ High Freq | 2 | 1363 | 0–2 | inferred (always 1) |
| `00 06` | EQ High Gain | 5 | 1365 | 0–30 | verified |

## MIDI (entry bytes 172–187, one byte per channel)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | Phase Lock (channel 1; channel n at +8(n−1)) | 1 | 1376 | 0–1 | inferred (always 0) |

## Part (51 bytes, entry bit `1504 + 408(n−1)`)

"Rel bit" is relative to the start of the Part.

| SysEx | Parameter | Bits | Rel bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | Receive Channel | 4 | 0 | 0–15 | verified |
| `00 01` | Receive Switch | 1 | 4 | 0–1 | verified |
| `00 02` | (reserve) | 1 | 5 |  | inferred (always 1) |
| `00 03` | (reserve) | 1 | 6 |  | inferred (always 1) |
| `00 04` | (reserve) | 1 | 7 |  | inferred (always 1) |
| `00 05` | (reserve) | 1 | 8 |  | inferred (always 1) |
| `00 06` | Tone Bank Select MSB | 7 | 9 | 0–127 | verified |
| `00 07` | Tone Bank Select LSB | 7 | 16 | 0–127 | verified |
| `00 08` | Tone Program Number (0-based) | 7 | 23 | 0–127 | verified |
| `00 09` | Part Level | 7 | 30 | 0–127 | verified |
| `00 0A` | Part Pan | 7 | 37 | 0–127 | verified |
| `00 0B` | Part Coarse Tune | 7 | 44 | 16–112 | verified |
| `00 0C` | Part Fine Tune | 7 | 51 | 14–114 | verified |
| `00 0D` | Part Mono/Poly | 2 | 58 | 0–2 | inferred (always 2) |
| `00 0E` | Part Legato Switch | 2 | 60 | 0–2 | inferred (always 2) |
| `00 0F` | Part Pitch Bend Range | 5 | 62 | 0–25 | verified |
| `00 10` | Part Portamento Switch | 2 | 67 | 0–2 | inferred (always 2) |
| `00 11` | Part Portamento Time (#, 2 nibbles) | 8 | 69 | 0–128 | inferred (always 128) |
| `00 13` | Part Cutoff Offset | 7 | 77 | 0–127 | verified |
| `00 14` | Part Resonance Offset | 7 | 84 | 0–127 | verified |
| `00 15` | Part Attack Time Offset | 7 | 91 | 0–127 | verified |
| `00 16` | Part Decay Time Offset | 7 | 98 | 0–127 | verified |
| `00 17` | Part Release Time Offset | 7 | 105 | 0–127 | verified |
| `00 18` | Part Vibrato Rate | 7 | 112 | 0–127 | inferred (always 64) |
| `00 19` | Part Vibrato Depth | 7 | 119 | 0–127 | inferred (always 64) |
| `00 1A` | Part Vibrato Delay | 7 | 126 | 0–127 | inferred (always 64) |
| `00 1B` | Part Octave Shift (bias) | 3 | 133 | 61–67 | verified |
| `00 1C` | Part Velocity Sens Offset | 7 | 136 | 1–127 | inferred (always 64) |
| `00 1D` | Keyboard Range Lower | 7 | 143 | 0–127 | verified |
| `00 1E` | Keyboard Range Upper | 7 | 150 | 0–127 | verified |
| `00 1F` | Keyboard Fade Width Lower | 7 | 157 | 0–127 | inferred (always 0) |
| `00 20` | Keyboard Fade Width Upper | 7 | 164 | 0–127 | verified |
| `00 21` | Velocity Range Lower | 7 | 171 | 1–127 | inferred (always 1) |
| `00 22` | Velocity Range Upper | 7 | 178 | 0–127 | inferred (always 127) |
| `00 23` | Velocity Fade Width Lower | 7 | 185 | 0–127 | inferred (always 0) |
| `00 24` | Velocity Fade Width Upper | 7 | 192 | 0–127 | inferred (always 0) |
| `00 25` | Mute Switch | 1 | 199 | 0–1 | inferred (always 0) |
| `00 26` | (reserve) | 7 | 200 |  | inferred (always 127) |
| `00 27` | Part Chorus Send Level | 7 | 207 | 0–127 | verified |
| `00 28` | Part Reverb Send Level | 7 | 214 | 0–127 | verified |
| `00 29` | Part Output Assign | 4 | 221 | 0–11 | inferred (always 0) |
| `00 2A` | (reserve) | 2 | 225 |  | inferred (always 0) |
| `00 2B` | Part Scale Tune Type | 7 | 227 | 0–8 | inferred (always 1) |
| `00 2C` | Part Scale Tune Key | 7 | 234 | 0–11 | inferred (always 0) |
| `00 2D` | Part Scale Tune for C | 7 | 241 | 0–127 | inferred (always 64) |
| `00 2E` | Part Scale Tune for C# | 7 | 248 | 0–127 | inferred (always 64) |
| `00 2F` | Part Scale Tune for D | 7 | 255 | 0–127 | inferred (always 64) |
| `00 30` | Part Scale Tune for D# | 7 | 262 | 0–127 | inferred (always 64) |
| `00 31` | Part Scale Tune for E | 7 | 269 | 0–127 | inferred (always 64) |
| `00 32` | Part Scale Tune for F | 7 | 276 | 0–127 | inferred (always 64) |
| `00 33` | Part Scale Tune for F# | 7 | 283 | 0–127 | inferred (always 64) |
| `00 34` | Part Scale Tune for G | 7 | 290 | 0–127 | inferred (always 64) |
| `00 35` | Part Scale Tune for G# | 7 | 297 | 0–127 | inferred (always 64) |
| `00 36` | Part Scale Tune for A | 7 | 304 | 0–127 | inferred (always 64) |
| `00 37` | Part Scale Tune for A# | 7 | 311 | 0–127 | inferred (always 64) |
| `00 38` | Part Scale Tune for B | 7 | 318 | 0–127 | inferred (always 64) |
| `00 39` | Receive Program Change | 1 | 325 | 0–1 | inferred (always 1) |
| `00 3A` | Receive Bank Select | 1 | 326 | 0–1 | inferred (always 1) |
| `00 3B` | Receive Pitch Bend | 1 | 327 | 0–1 | inferred (always 1) |
| `00 3C` | Receive Polyphonic Key Pressure | 1 | 328 | 0–1 | inferred (always 1) |
| `00 3D` | Receive Channel Pressure | 1 | 329 | 0–1 | verified |
| `00 3E` | Receive Modulation | 1 | 330 | 0–1 | inferred (always 1) |
| `00 3F` | Receive Volume | 1 | 331 | 0–1 | inferred (always 1) |
| `00 40` | Receive Pan | 1 | 332 | 0–1 | inferred (always 1) |
| `00 41` | Receive Expression | 1 | 333 | 0–1 | inferred (always 1) |
| `00 42` | Receive Hold-1 | 1 | 334 | 0–1 | inferred (always 1) |
| `00 43` | Velocity Curve Type | 3 | 335 | 0–4 | inferred (always 0) |
| `00 44` | Motional Surround L-R | 7 | 338 | 0–127 | inferred (always 64) |
| `00 45` | (reserve) | 7 | 345 |  | inferred (always 64) |
| `00 46` | Motional Surround F-B | 7 | 352 | 0–127 | inferred (always 74) |
| `00 47` | (reserve) | 7 | 359 |  | inferred (always 64) |
| `00 48` | Motional Surround Width | 6 | 366 | 0–32 | inferred (always 10) |
| `00 49` | Motional Surround Ambience Send Level | 7 | 372 | 0–127 | inferred (always 40) |
| `00 4A`–`00 4C` | (reserve) + section pad | 29 | 379–407 |  | always 0 |

Part Octave Shift uses the 3-bit bias (stored 4 = SysEx 64). See
[Signed Parameter Bias](02-encoding.md#signed-parameter-bias).

## Part EQ (4 bytes, entry bit `8032 + 32(n−1)`)

| SysEx | Parameter | Bits | Entry bit | Range | Status |
|---|---|---|---|---|---|
| `00 00` | EQ Switch | 1 | 0 (+8032 + 32(n−1)) | 0–1 | verified |
| `00 01` | EQ Low Freq | 1 | 1 (+8032 + 32(n−1)) | 0–1 | inferred (always 1) |
| `00 02` | EQ Low Gain | 5 | 2 (+8032 + 32(n−1)) | 0–30 | verified |
| `00 03` | EQ Mid Freq | 5 | 7 (+8032 + 32(n−1)) | 0–16 | inferred (always 7) |
| `00 04` | EQ Mid Gain | 5 | 12 (+8032 + 32(n−1)) | 0–30 | verified |
| `00 05` | EQ Mid Q | 3 | 17 (+8032 + 32(n−1)) | 0–4 | inferred (always 0) |
| `00 06` | EQ High Freq | 2 | 20 (+8032 + 32(n−1)) | 0–2 | inferred (always 1) |
| `00 07` | EQ High Gain | 5 | 22 (+8032 + 32(n−1)) | 0–30 | verified |

## Tone Reference

A Part's tone is the triple at **rel bits +9 (MSB), +16 (LSB), +23 (PC)**, 7 bits
each. For Part 1 that is entry bits 1513, 1520 and 1527, so PC spans bytes
190–191.

- **PC is stored 0-based**: the stored value is the SysEx value, and the display
  number is PC + 1.
- **User slot = `LSB × 128 + PC + 1`** for User banks (for example `95/0/PC`,
  User SN Synth tones). In a SOUND file this number points into the file, not
  into user memory. See [SOUND vs BACKUP](11-sound-vs-backup.md#tone-references-are-renumbered).
- MSB/LSB use the bank numbers in
  [`docs/midi/03-bank-select-tables.md`](../midi/03-bank-select-tables.md).

Evidence that PC is 0-based:

- 3,617 of 4,631 live User SN Synth references have PC = 0. That is impossible if
  PC were 1-based.
- With 0-based PC, all 4,631 SN Synth and 6 PCM Synth User references land on
  non-INIT tones whose names fit the Studio Set. Every Part of `SL-D-50 Set`
  lands on `SL-D50 1`..`10`, and every Part of `SL-Jupiter-8 Set` on an `SL-JP8`
  tone. `SLO-JP6 Brass 2` lands on `SL2-JP6 3` (Syn.BRS) and `SLO-JP6 Synth Ld` on
  `SL2-JP6 4` (Syn.LD). With 1-based PC both land on the wrong category.
- Presets agree. Children of Bodom `MARCATO/HARP` resolves to Marcato, Warm
  Strings, Harp Nail and Harp with 0-based PC, and to Somber Str, St.Strings and
  Gliss C Dori with 1-based PC. `VIBE` resolves to SA Vibe SRX, Vibraphone and
  Vibraphone!, and `HIT` to Orch Hit, OrchHit SRX and OrchestraHit.
- LSB > 0 was checked once. The INIT Studio Sets of Synth Legends reference
  `95/3/127` = slot 512, which is `INIT SYNTH` in that backup.

User tones are always referenced with LSB 0 (slots 1–128) in this corpus.

## Evidence

- **Range check**: all 1,422 mapped fields × 1,482 entries fall inside their
  documented SysEx ranges (0 violations).
- **Uncovered bits**: no bit outside the mapped fields varies anywhere in the
  corpus.
- **Names**: Studio Set names decode as 16 × 7-bit ASCII at bit 0. 334 of 334
  sound-list entries match, excluding slot 64. In 11 lists slot 64 reads
  `INIT SYNTH` where the backup has `INIT STUDIO`, which is a list placeholder.
- **Tone references** resolve to tones whose names and categories fit the Studio
  Set (see above), for User tones and for presets alike.

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| Width and placement of the *inferred* fields (Tone Control sources, Tempo, Part Output/`00 2A` split, Scale Tune Key, Velocity Curve, Motional Surround fields, chorus 7 + 3 pad bits) | Set non-default values in a User Studio Set, back up, and diff the PRFb entry. |
| Meaning of Common bits 112–143 (`FE 82 08 78`) | The same diff while changing Voice Reserve and the other `00 10`–`00 38` items. |
| Tone references in LSB > 0 for User tones | Save a Studio Set whose Part uses a User tone in slot > 128, back up, and check LSB/PC. |
