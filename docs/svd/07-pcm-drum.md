# PCM Drum Kit — SVD Mapping (RFRa)

Chunk type: `RFRa` | Entry size: **10890 bytes** | Model: `MI69` |
SysEx block: PCM Drum Kit (`19 10 00 00` for the Part 1 Temporary Tone)

**Confidence: INIT-only (medium).** All 576 occurrences in the 18 official
packs (18 × 32, BACKUP files only) are one of 2 INIT kits: `INIT KIT` (16 packs)
and `INIT RHYTHM` (TB-303 and TB-303 Pure backups). No pack ships a real PCM
Drum Kit. Within each kit all 88 keys are identical and most fields hold
defaults. The section boundaries are pinned. The partial names repeat every
122 bytes and the long name sits at byte 10870. Field placement inside a
section rests on the MIDI Implementation order and has not been confirmed with
non-default data.

Parameter lists, bit widths and ranges come from the official INTEGRA-7 MIDI
Implementation v1.00, including the `(reserve)` rows that
[`docs/midi/07-pcm-drum-kit.md`](../midi/07-pcm-drum-kit.md) omits.

## Entry Layout

| Byte Range               | Section                  | SysEx block             | Bits used | Bytes |
|--------------------------|--------------------------|-------------------------|-----------|-------|
| 0–13                     | Common                   | `00 00 00`              | 105       | 14    |
| 14–91                    | Common MFX               | `00 02 00`              | 618       | 78    |
| 92–133                   | Common Comp/EQ           | `00 08 00`              | 330       | 42    |
| 134 + 122n (n = 0…87)    | Partial, key 21 + n      | `00 10 00`…`01 3E 00`   | 969       | 122 each (10736 total) |
| 10870–10888              | Common 2                 | `00 30 00`              | 152       | 19    |
| 10889                    | Zero padding             | —                       | —         | 1     |
| **Total**                |                          |                         |           | **10890** |

There is **no** `0x0E` end marker.

## Names

- **Kit name**: Common `00 00`–`00 0B`, 12 × 7-bit ASCII at bit 0.
- **Long name**: Common 2 `00 00`–`00 0F`, 16 × 7-bit ASCII at byte 10870
  (`INITIAL RHYTHM  ` in `INIT KIT`, `XV Pop Kit      ` in `INIT RHYTHM`).
- **Per-key partial name**: Partial `00 00`–`00 0B`, 12 × 7-bit ASCII at byte
  `134 + 122n` (`INIT PARTIAL` or blanks).
- No category field is documented for drum kits, and none was found.

## Undocumented and Reserve Fields with Content

These fields must round-trip verbatim.

| Where                          | Bits    | Observed |
|--------------------------------|---------|----------|
| Common `00 0D`, `00 10`        | 1, 1    | unnamed in the MIDI Implementation; 0 observed |
| Common `00 0E` (#, 2 nibbles)  | 8       | unnamed; 0 (`INIT KIT`) or 120 (`INIT RHYTHM`) |
| Common `00 11`                 | 4       | always 13 (same as PCM Synth Common `00 27`) |
| MFX `00 01`                    | 7       | always 127 |
| Common 2 `00 00`–`00 0F`       | 16 × 7  | 16-character long name |

## Common (entry bytes 0–13)

"Section bit" is relative to the section start.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | Kit Name 1 | 7 | 0 | 32–127 |  |  |
| `00 01` | Kit Name 2 | 7 | 7 | 32–127 |  |  |
| `00 02` | Kit Name 3 | 7 | 14 | 32–127 |  |  |
| `00 03` | Kit Name 4 | 7 | 21 | 32–127 |  |  |
| `00 04` | Kit Name 5 | 7 | 28 | 32–127 |  |  |
| `00 05` | Kit Name 6 | 7 | 35 | 32–127 |  |  |
| `00 06` | Kit Name 7 | 7 | 42 | 32–127 |  |  |
| `00 07` | Kit Name 8 | 7 | 49 | 32–127 |  |  |
| `00 08` | Kit Name 9 | 7 | 56 | 32–127 |  |  |
| `00 09` | Kit Name 10 | 7 | 63 | 32–127 |  |  |
| `00 0A` | Kit Name 11 | 7 | 70 | 32–127 |  |  |
| `00 0B` | Kit Name 12 | 7 | 77 | 32–127 |  |  |
| `00 0C` | Kit Level | 7 | 84 | 0–127 |  |  |
| `00 0D` | (unnamed) | 1 | 91 |  |  | undocumented (unnamed in MI); 0 observed |
| `00 0E` | (unnamed) | 4+4 | 92 |  |  | undocumented (unnamed in MI) 8-bit; 0 / 120 observed |
| `00 10` | (unnamed) | 1 | 100 |  |  | undocumented (unnamed in MI); 0 observed |
| `00 11` | (reserve) | 4 | 101 |  |  | always 13 (same signature as PCM Synth Common 00 27) |

## Common MFX (entry bytes 14–91)

Same 618-bit layout as every other tone type. See
[PCM Synth Tone, Common MFX](06-pcm-synth.md#common-mfx-entry-bytes-56133).

## Common Comp/EQ (entry bytes 92–133)

Six groups of 55 bits (Comp n + EQ n), then 6 zero pad bits. SN Drum Kit uses
the same layout.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | Comp1 Switch | 1 | 0 | 0–1 |  |  |
| `00 01` | Comp1 Attack Time | 5 | 1 | 0–31 |  |  |
| `00 02` | Comp1 Release Time | 5 | 6 | 0–23 |  |  |
| `00 03` | Comp1 Threshold | 7 | 11 | 0–127 |  |  |
| `00 04` | Comp1 Ratio | 5 | 18 | 0–19 |  |  |
| `00 05` | Comp1 Output Gain | 5 | 23 | 0–24 |  |  |
| `00 06` | EQ1 Switch | 1 | 28 | 0–1 |  |  |
| `00 07` | EQ1 Low Freq | 1 | 29 | 0–1 |  |  |
| `00 08` | EQ1 Low Gain | 5 | 30 | 0–30 |  |  |
| `00 09` | EQ1 Mid Freq | 5 | 35 | 0–16 |  |  |
| `00 0A` | EQ1 Mid Gain | 5 | 40 | 0–30 |  |  |
| `00 0B` | EQ1 Mid Q | 3 | 45 | 0–4 |  |  |
| `00 0C` | EQ1 High Freq | 2 | 48 | 0–2 |  |  |
| `00 0D` | EQ1 High Gain | 5 | 50 | 0–30 |  |  |
| `00 0E` | Comp2 Switch | 1 | 55 | 0–1 |  |  |
| `00 0F` | Comp2 Attack Time | 5 | 56 | 0–31 |  |  |
| `00 10` | Comp2 Release Time | 5 | 61 | 0–23 |  |  |
| `00 11` | Comp2 Threshold | 7 | 66 | 0–127 |  |  |
| `00 12` | Comp2 Ratio | 5 | 73 | 0–19 |  |  |
| `00 13` | Comp2 Output Gain | 5 | 78 | 0–24 |  |  |
| `00 14` | EQ2 Switch | 1 | 83 | 0–1 |  |  |
| `00 15` | EQ2 Low Freq | 1 | 84 | 0–1 |  |  |
| `00 16` | EQ2 Low Gain | 5 | 85 | 0–30 |  |  |
| `00 17` | EQ2 Mid Freq | 5 | 90 | 0–16 |  |  |
| `00 18` | EQ2 Mid Gain | 5 | 95 | 0–30 |  |  |
| `00 19` | EQ2 Mid Q | 3 | 100 | 0–4 |  |  |
| `00 1A` | EQ2 High Freq | 2 | 103 | 0–2 |  |  |
| `00 1B` | EQ2 High Gain | 5 | 105 | 0–30 |  |  |
| `00 1C` | Comp3 Switch | 1 | 110 | 0–1 |  |  |
| `00 1D` | Comp3 Attack Time | 5 | 111 | 0–31 |  |  |
| `00 1E` | Comp3 Release Time | 5 | 116 | 0–23 |  |  |
| `00 1F` | Comp3 Threshold | 7 | 121 | 0–127 |  |  |
| `00 20` | Comp3 Ratio | 5 | 128 | 0–19 |  |  |
| `00 21` | Comp3 Output Gain | 5 | 133 | 0–24 |  |  |
| `00 22` | EQ3 Switch | 1 | 138 | 0–1 |  |  |
| `00 23` | EQ3 Low Freq | 1 | 139 | 0–1 |  |  |
| `00 24` | EQ3 Low Gain | 5 | 140 | 0–30 |  |  |
| `00 25` | EQ3 Mid Freq | 5 | 145 | 0–16 |  |  |
| `00 26` | EQ3 Mid Gain | 5 | 150 | 0–30 |  |  |
| `00 27` | EQ3 Mid Q | 3 | 155 | 0–4 |  |  |
| `00 28` | EQ3 High Freq | 2 | 158 | 0–2 |  |  |
| `00 29` | EQ3 High Gain | 5 | 160 | 0–30 |  |  |
| `00 2A` | Comp4 Switch | 1 | 165 | 0–1 |  |  |
| `00 2B` | Comp4 Attack Time | 5 | 166 | 0–31 |  |  |
| `00 2C` | Comp4 Release Time | 5 | 171 | 0–23 |  |  |
| `00 2D` | Comp4 Threshold | 7 | 176 | 0–127 |  |  |
| `00 2E` | Comp4 Ratio | 5 | 183 | 0–19 |  |  |
| `00 2F` | Comp4 Output Gain | 5 | 188 | 0–24 |  |  |
| `00 30` | EQ4 Switch | 1 | 193 | 0–1 |  |  |
| `00 31` | EQ4 Low Freq | 1 | 194 | 0–1 |  |  |
| `00 32` | EQ4 Low Gain | 5 | 195 | 0–30 |  |  |
| `00 33` | EQ4 Mid Freq | 5 | 200 | 0–16 |  |  |
| `00 34` | EQ4 Mid Gain | 5 | 205 | 0–30 |  |  |
| `00 35` | EQ4 Mid Q | 3 | 210 | 0–4 |  |  |
| `00 36` | EQ4 High Freq | 2 | 213 | 0–2 |  |  |
| `00 37` | EQ4 High Gain | 5 | 215 | 0–30 |  |  |
| `00 38` | Comp5 Switch | 1 | 220 | 0–1 |  |  |
| `00 39` | Comp5 Attack Time | 5 | 221 | 0–31 |  |  |
| `00 3A` | Comp5 Release Time | 5 | 226 | 0–23 |  |  |
| `00 3B` | Comp5 Threshold | 7 | 231 | 0–127 |  |  |
| `00 3C` | Comp5 Ratio | 5 | 238 | 0–19 |  |  |
| `00 3D` | Comp5 Output Gain | 5 | 243 | 0–24 |  |  |
| `00 3E` | EQ5 Switch | 1 | 248 | 0–1 |  |  |
| `00 3F` | EQ5 Low Freq | 1 | 249 | 0–1 |  |  |
| `00 40` | EQ5 Low Gain | 5 | 250 | 0–30 |  |  |
| `00 41` | EQ5 Mid Freq | 5 | 255 | 0–16 |  |  |
| `00 42` | EQ5 Mid Gain | 5 | 260 | 0–30 |  |  |
| `00 43` | EQ5 Mid Q | 3 | 265 | 0–4 |  |  |
| `00 44` | EQ5 High Freq | 2 | 268 | 0–2 |  |  |
| `00 45` | EQ5 High Gain | 5 | 270 | 0–30 |  |  |
| `00 46` | Comp6 Switch | 1 | 275 | 0–1 |  |  |
| `00 47` | Comp6 Attack Time | 5 | 276 | 0–31 |  |  |
| `00 48` | Comp6 Release Time | 5 | 281 | 0–23 |  |  |
| `00 49` | Comp6 Threshold | 7 | 286 | 0–127 |  |  |
| `00 4A` | Comp6 Ratio | 5 | 293 | 0–19 |  |  |
| `00 4B` | Comp6 Output Gain | 5 | 298 | 0–24 |  |  |
| `00 4C` | EQ6 Switch | 1 | 303 | 0–1 |  |  |
| `00 4D` | EQ6 Low Freq | 1 | 304 | 0–1 |  |  |
| `00 4E` | EQ6 Low Gain | 5 | 305 | 0–30 |  |  |
| `00 4F` | EQ6 Mid Freq | 5 | 310 | 0–16 |  |  |
| `00 50` | EQ6 Mid Gain | 5 | 315 | 0–30 |  |  |
| `00 51` | EQ6 Mid Q | 3 | 320 | 0–4 |  |  |
| `00 52` | EQ6 High Freq | 2 | 323 | 0–2 |  |  |
| `00 53` | EQ6 High Gain | 5 | 325 | 0–30 |  |  |

## Partial (122 bytes per key, entry byte `134 + 122n`, key 21 + n)

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | Partial Name 1 | 7 | 0 | 32–127 |  |  |
| `00 01` | Partial Name 2 | 7 | 7 | 32–127 |  |  |
| `00 02` | Partial Name 3 | 7 | 14 | 32–127 |  |  |
| `00 03` | Partial Name 4 | 7 | 21 | 32–127 |  |  |
| `00 04` | Partial Name 5 | 7 | 28 | 32–127 |  |  |
| `00 05` | Partial Name 6 | 7 | 35 | 32–127 |  |  |
| `00 06` | Partial Name 7 | 7 | 42 | 32–127 |  |  |
| `00 07` | Partial Name 8 | 7 | 49 | 32–127 |  |  |
| `00 08` | Partial Name 9 | 7 | 56 | 32–127 |  |  |
| `00 09` | Partial Name 10 | 7 | 63 | 32–127 |  |  |
| `00 0A` | Partial Name 11 | 7 | 70 | 32–127 |  |  |
| `00 0B` | Partial Name 12 | 7 | 77 | 32–127 |  |  |
| `00 0C` | Assign Type | 1 | 84 | 0–1 |  |  |
| `00 0D` | Mute Group | 5 | 85 | 0–31 |  |  |
| `00 0E` | Partial Level | 7 | 90 | 0–127 |  |  |
| `00 0F` | Partial Coarse Tune | 7 | 97 | 0–127 |  |  |
| `00 10` | Partial Fine Tune | 7 | 104 | 14–114 |  |  |
| `00 11` | Partial Random Pitch Depth | 5 | 111 | 0–30 |  |  |
| `00 12` | Partial Pan | 7 | 116 | 0–127 |  |  |
| `00 13` | Partial Random Pan Depth | 6 | 123 | 0–63 |  |  |
| `00 14` | Partial Alternate Pan Depth | 7 | 129 | 1–127 |  |  |
| `00 15` | Partial Env Mode | 1 | 136 | 0–1 |  |  |
| `00 16` | Partial Output Level | 7 | 137 | 0–127 |  |  |
| `00 17` | (reserve) | 7 | 144 |  |  |  |
| `00 18` | (reserve) | 7 | 151 |  |  |  |
| `00 19` | Partial Chorus Send Level | 7 | 158 | 0–127 |  |  |
| `00 1A` | Partial Reverb Send Level | 7 | 165 | 0–127 |  |  |
| `00 1B` | Partial Output Assign | 4 | 172 | 0–6 |  |  |
| `00 1C` | Partial Pitch Bend Range | 6 | 176 | 0–48 |  |  |
| `00 1D` | Partial Receive Expression | 1 | 182 | 0–1 |  |  |
| `00 1E` | Partial Receive Hold-1 | 1 | 183 | 0–1 |  |  |
| `00 1F` | (reserve) | 1 | 184 |  |  |  |
| `00 20` | WMT Velocity Control | 2 | 185 | 0–2 |  |  |
| `00 21` | WMT1 Wave Switch | 1 | 187 | 0–1 |  |  |
| `00 22` | WMT1 Wave Group Type | 2 | 188 | 0–3 |  |  |
| `00 23` | WMT1 Wave Group ID | 4+4+4+4 | 190 | 0–16384 |  |  |
| `00 27` | WMT1 Wave Number L (Mono) | 4+4+4+4 | 206 | 0–16384 |  |  |
| `00 2B` | WMT1 Wave Number R | 4+4+4+4 | 222 | 0–16384 |  |  |
| `00 2F` | WMT1 Wave Gain | 2 | 238 | 0–3 |  |  |
| `00 30` | WMT1 Wave FXM Switch | 1 | 240 | 0–1 |  |  |
| `00 31` | WMT1 Wave FXM Color | 2 | 241 | 0–3 |  |  |
| `00 32` | WMT1 Wave FXM Depth | 5 | 243 | 0–16 |  |  |
| `00 33` | WMT1 Wave Tempo Sync | 1 | 248 | 0–1 |  |  |
| `00 34` | WMT1 Wave Coarse Tune | 7 | 249 | 16–112 |  |  |
| `00 35` | WMT1 Wave Fine Tune | 7 | 256 | 14–114 |  |  |
| `00 36` | WMT1 Wave Pan | 7 | 263 | 0–127 |  |  |
| `00 37` | WMT1 Wave Random Pan Switch | 1 | 270 | 0–1 |  |  |
| `00 38` | WMT1 Wave Alternate Pan Switch | 2 | 271 | 0–2 |  |  |
| `00 39` | WMT1 Wave Level | 7 | 273 | 0–127 |  |  |
| `00 3A` | WMT1 Velocity Range Lower | 7 | 280 | 1–127 |  |  |
| `00 3B` | WMT1 Velocity Range Upper | 7 | 287 | 1–127 |  |  |
| `00 3C` | WMT1 Velocity Fade Width Lower | 7 | 294 | 0–127 |  |  |
| `00 3D` | WMT1 Velocity Fade Width Upper | 7 | 301 | 0–127 |  |  |
| `00 3E` | WMT2 Wave Switch | 1 | 308 | 0–1 |  |  |
| `00 3F` | WMT2 Wave Group Type | 2 | 309 | 0–3 |  |  |
| `00 40` | WMT2 Wave Group ID | 4+4+4+4 | 311 | 0–16384 |  |  |
| `00 44` | WMT2 Wave Number L (Mono) | 4+4+4+4 | 327 | 0–16384 |  |  |
| `00 48` | WMT2 Wave Number R | 4+4+4+4 | 343 | 0–16384 |  |  |
| `00 4C` | WMT2 Wave Gain | 2 | 359 | 0–3 |  |  |
| `00 4D` | WMT2 Wave FXM Switch | 1 | 361 | 0–1 |  |  |
| `00 4E` | WMT2 Wave FXM Color | 2 | 362 | 0–3 |  |  |
| `00 4F` | WMT2 Wave FXM Depth | 5 | 364 | 0–16 |  |  |
| `00 50` | WMT2 Wave Tempo Sync | 1 | 369 | 0–1 |  |  |
| `00 51` | WMT2 Wave Coarse Tune | 7 | 370 | 16–112 |  |  |
| `00 52` | WMT2 Wave Fine Tune | 7 | 377 | 14–114 |  |  |
| `00 53` | WMT2 Wave Pan | 7 | 384 | 0–127 |  |  |
| `00 54` | WMT2 Wave Random Pan Switch | 1 | 391 | 0–1 |  |  |
| `00 55` | WMT2 Wave Alternate Pan Switch | 2 | 392 | 0–2 |  |  |
| `00 56` | WMT2 Wave Level | 7 | 394 | 0–127 |  |  |
| `00 57` | WMT2 Velocity Range Lower | 7 | 401 | 1–127 |  |  |
| `00 58` | WMT2 Velocity Range Upper | 7 | 408 | 1–127 |  |  |
| `00 59` | WMT2 Velocity Fade Width Lower | 7 | 415 | 0–127 |  |  |
| `00 5A` | WMT2 Velocity Fade Width Upper | 7 | 422 | 0–127 |  |  |
| `00 5B` | WMT3 Wave Switch | 1 | 429 | 0–1 |  |  |
| `00 5C` | WMT3 Wave Group Type | 2 | 430 | 0–3 |  |  |
| `00 5D` | WMT3 Wave Group ID | 4+4+4+4 | 432 | 0–16384 |  |  |
| `00 61` | WMT3 Wave Number L (Mono) | 4+4+4+4 | 448 | 0–16384 |  |  |
| `00 65` | WMT3 Wave Number R | 4+4+4+4 | 464 | 0–16384 |  |  |
| `00 69` | WMT3 Wave Gain | 2 | 480 | 0–3 |  |  |
| `00 6A` | WMT3 Wave FXM Switch | 1 | 482 | 0–1 |  |  |
| `00 6B` | WMT3 Wave FXM Color | 2 | 483 | 0–3 |  |  |
| `00 6C` | WMT3 Wave FXM Depth | 5 | 485 | 0–16 |  |  |
| `00 6D` | WMT3 Wave Tempo Sync | 1 | 490 | 0–1 |  |  |
| `00 6E` | WMT3 Wave Coarse Tune | 7 | 491 | 16–112 |  |  |
| `00 6F` | WMT3 Wave Fine Tune | 7 | 498 | 14–114 |  |  |
| `00 70` | WMT3 Wave Pan | 7 | 505 | 0–127 |  |  |
| `00 71` | WMT3 Wave Random Pan Switch | 1 | 512 | 0–1 |  |  |
| `00 72` | WMT3 Wave Alternate Pan Switch | 2 | 513 | 0–2 |  |  |
| `00 73` | WMT3 Wave Level | 7 | 515 | 0–127 |  |  |
| `00 74` | WMT3 Velocity Range Lower | 7 | 522 | 1–127 |  |  |
| `00 75` | WMT3 Velocity Range Upper | 7 | 529 | 1–127 |  |  |
| `00 76` | WMT3 Velocity Fade Width Lower | 7 | 536 | 0–127 |  |  |
| `00 77` | WMT3 Velocity Fade Width Upper | 7 | 543 | 0–127 |  |  |
| `00 78` | WMT4 Wave Switch | 1 | 550 | 0–1 |  |  |
| `00 79` | WMT4 Wave Group Type | 2 | 551 | 0–3 |  |  |
| `00 7A` | WMT4 Wave Group ID | 4+4+4+4 | 553 | 0–16384 |  |  |
| `00 7E` | WMT4 Wave Number L (Mono) | 4+4+4+4 | 569 | 0–16384 |  |  |
| `01 02` | WMT4 Wave Number R | 4+4+4+4 | 585 | 0–16384 |  |  |
| `01 06` | WMT4 Wave Gain | 2 | 601 | 0–3 |  |  |
| `01 07` | WMT4 Wave FXM Switch | 1 | 603 | 0–1 |  |  |
| `01 08` | WMT4 Wave FXM Color | 2 | 604 | 0–3 |  |  |
| `01 09` | WMT4 Wave FXM Depth | 5 | 606 | 0–16 |  |  |
| `01 0A` | WMT4 Wave Tempo Sync | 1 | 611 | 0–1 |  |  |
| `01 0B` | WMT4 Wave Coarse Tune | 7 | 612 | 16–112 |  |  |
| `01 0C` | WMT4 Wave Fine Tune | 7 | 619 | 14–114 |  |  |
| `01 0D` | WMT4 Wave Pan | 7 | 626 | 0–127 |  |  |
| `01 0E` | WMT4 Wave Random Pan Switch | 1 | 633 | 0–1 |  |  |
| `01 0F` | WMT4 Wave Alternate Pan Switch | 2 | 634 | 0–2 |  |  |
| `01 10` | WMT4 Wave Level | 7 | 636 | 0–127 |  |  |
| `01 11` | WMT4 Velocity Range Lower | 7 | 643 | 1–127 |  |  |
| `01 12` | WMT4 Velocity Range Upper | 7 | 650 | 1–127 |  |  |
| `01 13` | WMT4 Velocity Fade Width Lower | 7 | 657 | 0–127 |  |  |
| `01 14` | WMT4 Velocity Fade Width Upper | 7 | 664 | 0–127 |  |  |
| `01 15` | Pitch Env Depth | 5 | 671 | 52–76 | +48 |  |
| `01 16` | Pitch Env Velocity Sens | 7 | 676 | 1–127 |  |  |
| `01 17` | Pitch Env Time 1 Velocity Sens | 7 | 683 | 1–127 |  |  |
| `01 18` | Pitch Env Time 4 Velocity Sens | 7 | 690 | 1–127 |  |  |
| `01 19` | Pitch Env Time 1 | 7 | 697 | 0–127 |  |  |
| `01 1A` | Pitch Env Time 2 | 7 | 704 | 0–127 |  |  |
| `01 1B` | Pitch Env Time 3 | 7 | 711 | 0–127 |  |  |
| `01 1C` | Pitch Env Time 4 | 7 | 718 | 0–127 |  |  |
| `01 1D` | Pitch Env Level 0 | 7 | 725 | 1–127 |  |  |
| `01 1E` | Pitch Env Level 1 | 7 | 732 | 1–127 |  |  |
| `01 1F` | Pitch Env Level 2 | 7 | 739 | 1–127 |  |  |
| `01 20` | Pitch Env Level 3 | 7 | 746 | 1–127 |  |  |
| `01 21` | Pitch Env Level 4 | 7 | 753 | 1–127 |  |  |
| `01 22` | TVF Filter Type | 3 | 760 | 0–6 |  |  |
| `01 23` | TVF Cutoff Frequency | 7 | 763 | 0–127 |  |  |
| `01 24` | TVF Cutoff Velocity Curve | 3 | 770 | 0–7 |  |  |
| `01 25` | TVF Cutoff Velocity Sens | 7 | 773 | 1–127 |  |  |
| `01 26` | TVF Resonance | 7 | 780 | 0–127 |  |  |
| `01 27` | TVF Resonance Velocity Sens | 7 | 787 | 1–127 |  |  |
| `01 28` | TVF Env Depth | 7 | 794 | 1–127 |  |  |
| `01 29` | TVF Env Velocity Curve Type | 3 | 801 | 0–7 |  |  |
| `01 2A` | TVF Env Velocity Sens | 7 | 804 | 1–127 |  |  |
| `01 2B` | TVF Env Time 1 Velocity Sens | 7 | 811 | 1–127 |  |  |
| `01 2C` | TVF Env Time 4 Velocity Sens | 7 | 818 | 1–127 |  |  |
| `01 2D` | TVF Env Time 1 | 7 | 825 | 0–127 |  |  |
| `01 2E` | TVF Env Time 2 | 7 | 832 | 0–127 |  |  |
| `01 2F` | TVF Env Time 3 | 7 | 839 | 0–127 |  |  |
| `01 30` | TVF Env Time 4 | 7 | 846 | 0–127 |  |  |
| `01 31` | TVF Env Level 0 | 7 | 853 | 0–127 |  |  |
| `01 32` | TVF Env Level 1 | 7 | 860 | 0–127 |  |  |
| `01 33` | TVF Env Level 2 | 7 | 867 | 0–127 |  |  |
| `01 34` | TVF Env Level 3 | 7 | 874 | 0–127 |  |  |
| `01 35` | TVF Env Level 4 | 7 | 881 | 0–127 |  |  |
| `01 36` | TVA Level Velocity Curve | 3 | 888 | 0–7 |  |  |
| `01 37` | TVA Level Velocity Sens | 7 | 891 | 1–127 |  |  |
| `01 38` | TVA Env Time 1 Velocity Sens | 7 | 898 | 1–127 |  |  |
| `01 39` | TVA Env Time 4 Velocity Sens | 7 | 905 | 1–127 |  |  |
| `01 3A` | TVA Env Time 1 | 7 | 912 | 0–127 |  |  |
| `01 3B` | TVA Env Time 2 | 7 | 919 | 0–127 |  |  |
| `01 3C` | TVA Env Time 3 | 7 | 926 | 0–127 |  |  |
| `01 3D` | TVA Env Time 4 | 7 | 933 | 0–127 |  |  |
| `01 3E` | TVA Env Level 1 | 7 | 940 | 0–127 |  |  |
| `01 3F` | TVA Env Level 2 | 7 | 947 | 0–127 |  |  |
| `01 40` | TVA Env Level 3 | 7 | 954 | 0–127 |  |  |
| `01 41` | One Shot Mode | 1 | 961 | 0–1 |  |  |
| `01 42` | (reserve) | 7 | 962 |  |  |  |

## Common 2 (entry bytes 10870–10888)

`00 00`–`00 0F` hold the 16-character long name.

| SysEx | Parameter | Bits | Section bit | Range | SVD→SysEx | Notes |
|---|---|---|---|---|---|---|
| `00 00` | (reserve) | 7 | 0 |  |  | 00 00-00 0F: **16-char long name**, 7-bit ASCII |
| `00 01`–`00 0E` | (reserve) × 14 | 7 each | 7–104 |  |  | same as the row above |
| `00 0F` | (reserve) | 7 | 105 |  |  |  |
| `00 10` | Phrase Number | 4+4 | 112 | 0–255 |  |  |
| `00 12` | (reserve) | 1 | 120 |  |  |  |
| `00 13`–`00 2F` | (reserve) × 29 | 1 each | 121–149 |  |  | same as the row above |
| `00 30` | (reserve) | 1 | 150 |  |  |  |
| `00 31` | TFX Switch | 1 | 151 | 0–1 |  |  |

## Evidence

| Hypothesis | Values checked | Violations |
|------------|----------------|------------|
| **Layout above** | 7,940,736 | **0** |
| Comp/EQ bit-concatenated after MFX | 7,940,736 | 3,456 |
| Partial stride 121 | 7,940,736 | 1,379,648 |
| Partial stride 123 | — | overruns the entry |
| Common 15 B (everything shifted by +1) | 7,940,736 | 1,697,024 |

- Without the bias rule, every biased value is out of range (50,688 of 50,688).
- The kit, long and partial names decode correctly in both INIT kits.
- INIT defaults look plausible: Comp/EQ gains are 15 (0 dB) and every key has
  Coarse Tune 60.
- The alignment of WMT2–4 inside a Partial is supported only by `INIT KIT`
  carrying wave 721 in all four WMTs.

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| Per-key order (key 21 first is inferred from the MIDI Implementation order) and in-section field placement | Build a kit on the device with distinct values on keys 21, 60 and 108 (wave, level, pan) and non-default Comp/EQ. Save it, back up, and diff the RQ1 of each per-key Partial block against the decode. |
| Meaning of Common `00 0D`, `00 0E`, `00 10`, `00 11` | The same backup, plus edits that change one kit-level setting at a time. |
