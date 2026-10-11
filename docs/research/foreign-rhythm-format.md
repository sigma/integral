# JV and XV Rhythm Sets vs the PCM Drum Kit (research for #205)

Answers [#205](https://github.com/sigma/integral/issues/205), part of the Foreign patch map
[#192](https://github.com/sigma/integral/issues/192). It covers how a JV/XP **Rhythm Setup** and an
XV-series **Rhythm Set** are framed, and where each parameter lands on the INTEGRA-7 PCM Drum Kit
([`docs/midi/07-pcm-drum-kit.md`](../midi/07-pcm-drum-kit.md)). It then checks the result against
the local corpus. The row-by-row table for both sources is in the sibling
[`foreign-rhythm-format.csv`](foreign-rhythm-format.csv).

The Patch-side notes this builds on:
[`jv-patch-format.md`](jv-patch-format.md) ([#194](https://github.com/sigma/integral/issues/194)),
[`xv-patch-format.md`](xv-patch-format.md) ([#193](https://github.com/sigma/integral/issues/193)),
[`jv-wave-map.md`](jv-wave-map.md) ([#195](https://github.com/sigma/integral/issues/195)) and
[`foreign-fx-map.md`](foreign-fx-map.md) ([#196](https://github.com/sigma/integral/issues/196)).
Terms follow `CONTEXT.md` (**Foreign patch**, **Conversion report**, **Wave**, **Wave Group**,
**Wave Number**).

## TL;DR

- **XV Rhythm Set is the Integra PCM Drum Kit with the same offsets.** The Rhythm Common
  (`0x12` bytes), Common MFX (`0x111`) and the per-key Rhythm Tone at `00 10 00` + 2 × (key − 21)
  sit where the Integra puts Kit Common, MFX and Partials. The keys are the same 21–108. Of the
  157 Rhythm Tone parameters, **149 copy byte for byte**, 5 need a table and 3 have no target.
  The Integra Partial is 2 bytes longer (`0x143` vs `0x141`). It adds a per-key **One Shot
  Mode** at `01 41`, which the XV keeps once per kit in Common `00 10`. The Integra's unnamed
  Common bytes `00 0D`/`00 0E`/`00 10`/`00 11` are the XV Clock Source, Rhythm Tempo (120),
  One Shot Mode and Output Assign (13).
- **One trap breaks the byte copy:** Tone Output Assign `00 1B`. On the XV it is MFX, A–D, 1–8
  (0–12). On the Integra it is PART, COMP+EQ1–6 (0–6). In the corpus **98.6 % of XV Rhythm
  Tones are on output A (1)**. Copied unchanged, they would all land on COMP+EQ1. Write 0 (PART).
- **JV Rhythm Setup is a much smaller model.** It has a name-only Rhythm Common (`0x0C`) and one
  58-byte Rhythm Note per key, for keys **35–98** (64 keys). The Integra key is the same MIDI
  note, so keys 21–34 and 99–108 stay OFF. Each JV Note has one Wave, no velocity ranges, no
  LFO, no FXM and no velocity curves. It has no EFX, Chorus or Reverb of its own; those belong
  to the Performance. Of the 57 Note parameters, 38 are identical (one of them approximate),
  7 are offsets, 3 go through the Wave table, 7 need a rescale and 2 have no target.
- **Roland's own JV → XV conversion is in the corpus.** The SR-JV80-16 "Orch Set" exists as a JV
  dump and as Roland's XV re-issue (`SRJV16R.mid`). It confirms every offset above. It also
  shows how Roland itself carried the parts with no formula:
  - Velocity Sensitivity uses a non-linear table (+100 % → +32, +140 % → +52), not `pct × 63/100`.
  - One JV Velocity Time Sensitivity goes into both Integra Time 1 and Time 4.
  - Volume Control Switch becomes Receive Expression.
  - JV MIX output becomes XV output A.
  - The WMT1 Random and Alternate Pan switches are turned ON.
- **The Waves are the same 448-Wave ROM**, so #195's map covers them. JV Rhythm Notes use the
  same Wave Group table as Patch Tones (INT-A 1–255, INT-B 1–193, PCM card, EXP). In the corpus
  77 % of active Notes map with high or medium confidence. The gap is **SR-JV80-06 Dance**
  (524 Notes, 16 % of active Notes; #195 found no Wave list for it). Roland's own Orch Set agrees
  with `jv-wave-map.json` on 61 of 64 keys. On the other 3, Roland chose a different, newer
  hi-hat.
- **Corpus check:**
  - JV: 184 Rhythm Setups, 50 distinct, all in range.
  - XV: 79 Rhythm Sets, 62 distinct, all in range apart from zeroed data in one unused key.
  - XV Wave Group ID is **1 for INT** and the **board number for SRX** (3, 6, 7, 8, 9).

## Sources

| # | Source | What it gives |
|---|--------|---------------|
| P1 | Roland *JV-1080 Owner's Manual*, MIDI Implementation p. 154 (PDF page 154; [PDF](https://static.roland.com/assets/media/pdf/JV-1080_OM.pdf), scanned; read from page images) | Rhythm setup map `00 00` Common, `23 00`–`62 00` Note for keys 35 (B1)–98 (E7); Rhythm Common `0x0C`; Rhythm Note `0x3A` field by field; RQ1/DT1 examples at `02 09` |
| P2 | Roland *JV-1010 Owner's Manual*, MIDI Implementation pp. 72–80 ([PDF](https://static.roland.com/assets/media/pdf/JV-1010_OM.pdf), text) | Same Rhythm map (§1-4), `10 40`/`10 41` User Rhythm Setup, `02 09` Temporary; Wave Group table; value tables `*1`–`*9` |
| P3 | Roland *XP-30 Owner's Manual*, MIDI Implementation pp. 190–196 ([PDF](https://static.roland.com/assets/media/pdf/XP-30_OM.pdf), text) | Rhythm Setup §1-4 identical to P2 (machine diff) |
| P4 | Roland *XV-5080 Owner's Manual*, Parameter Address Map pp. 290, 297–300 ([PDF](https://static.roland.com/assets/media/pdf/XV-5080_OM.pdf)) | `*1-3-2` Rhythm: Common `0x12`, MFX `0x111`, Chorus `0x34`, Reverb `0x53`, Rhythm Tone `0x141` at `00 10 00`–`01 3E 00` (keys 21–108); User Rhythm 1–4 at `40 00 00 00` + (n−1) × `00 10 00 00`; Temporary Rhythm = Temporary Patch area + `00 10 00 00` |
| P5 | Roland *XV-3080*, *XV-5050*, *XV-2020*, *XV-88* Owner's Manuals ([3080](https://static.roland.com/assets/media/pdf/XV-3080_OM.pdf), [5050](https://static.roland.com/assets/media/pdf/XV-5050_OM.pdf), [2020](https://static.roland.com/assets/media/pdf/XV-2020_OM.pdf), [88](https://static.roland.com/assets/media/pdf/XV-88_OM.pdf)) | Same Rhythm block sizes (`0x12`, `0x141`), the same key range and the same four User Rhythm slots on all five models |
| I1 | [`docs/midi/07-pcm-drum-kit.md`](../midi/07-pcm-drum-kit.md), [`docs/svd/07-pcm-drum.md`](../svd/07-pcm-drum.md) | Target layout; values seen in Integra reserves |
| C1 | `~/Downloads/integral-corpus/` and `~/Downloads/JV-1080_SP/` (read-only) | Validation; Roland's own JV → XV conversion of SR-JV80-16 "Orch Set" |

P1–P5 are primary. The XV Rhythm Tone table was extracted from the P4 text layer and compared,
offset by offset, with the Integra Partial table in I1. The JV Note table was read from the
P1 page images and diffed as text between P2 and P3.

## 1. Framing and addressing

Message framing is the same as for Patches: JV DT1 `F0 41 dev 6A 12 aa bb cc dd … sum F7`, and
XV DT1 `F0 41 dev 00 10 12 aa bb cc dd … sum F7`. Both use the Roland checksum. See
[`jv-patch-format.md`](jv-patch-format.md#framing) and [`xv-patch-format.md`](xv-patch-format.md#1-message-framing).
In the corpus every block is one DT1. No Rhythm block is split or merged.

### JV/XP Rhythm Setup (P1, P2, P3)

| Start address | Area |
|---|---|
| `02 09 00 00` | Temporary Rhythm Setup (the rhythm of Performance Part 10) |
| `10 40 00 00`, `10 41 00 00` | User Rhythm Setup 1 / 2 |
| `20 40 00 00` | Data-card Rhythm Setup (JV-1080 only) |

| Offset | Block | Size | DT1 length |
|---|---|---|---|
| `00 00` | Rhythm Common: name only | `0x0C` (12) | 23 |
| `23 00` … `62 00` | Rhythm Note for key 35 … 98; key = offset byte 2 | `0x3A` (58) | 69 |

A full Rhythm Setup is 65 DT1s. The Note layout is the same on every JV/XP model, so it says
nothing about the source model (P2 = P3 by machine diff, and P1 has the same fields).

### XV Rhythm Set (P4, P5)

| Start address | Area |
|---|---|
| `40 00 00 00` + (n−1) × `00 10 00 00`, n = 1–4 | User Rhythm 001–004 (all five models) |
| `1F 10 00 00` | Temporary Rhythm, Patch mode |
| `11 10 00 00` + (p−1) × `00 20 00 00` | Temporary Rhythm of Performance part p (7-bit carry, so part 10 is `13 30 00 00`) |

| Offset | Block | Size | DT1 length | Integra block at the same offset |
|---|---|---|---|---|
| `00 00 00` | Rhythm Common | `0x12` (18) | 30 | Kit Common, `0x12` |
| `00 02 00` | Rhythm Common MFX | `0x111` (145) | 157 | Kit Common MFX, `0x111` |
| `00 04 00` | Rhythm Common Chorus | `0x34` (52) | 64 | — |
| `00 06 00` | Rhythm Common Reverb | `0x53` (83) | 95 | — |
| — | — | — | — | Kit Common Comp/EQ `00 08 00` (Integra only) |
| `00 10 00` + 2 × (key − 21), keys 21–108 | Rhythm Tone | `0x141` (193) | 205 | Kit Partial, `0x143` (same base and stride) |
| — | — | — | — | Kit Common 2 `02 00 00` (Integra only) |

A full Rhythm Set is 92 DT1s. To group a stream, use the slot: address bytes 0 and 1, masked
with `0x70` for byte 1. The block offset is (byte 1 & `0x0F`) × 128 + byte 2. A new set starts
when the slot changes or the Common block comes again.

The XV-5050 manual prints User Rhythm (004) as `41 30 00 00`. That is a misprint for
`40 30 00 00`: the stride is `00 10 00 00` and P4/P5 list four slots.

### Key range

| Source | Keys | On the Integra (keys 21–108) |
|---|---|---|
| JV/XP | 35 (B1)–98 (E7), 64 keys | Same MIDI note. Keys 21–34 and 99–108 get all four WMTs OFF. |
| XV | 21 (A0)–108 (C8), 88 keys | Same MIDI note, one to one. |

No transposition is needed. Corpus: active JV keys span 35–98, active XV keys 28–99.

## 2. JV Rhythm Setup → PCM Drum Kit

The full table is in the CSV (`source = JV`). Notation: `v` = JV raw byte, `i` = Integra raw byte.
Targets are Partial offsets of the same key.

| JV off. | JV parameter (range) | Integra | Mapping |
|---|---|---|---|
| Common `00`–`0B` | Rhythm Name | Common `00`–`0B` Kit Name | identical (also pad into Common 2 long name) |
| `00` | Tone Switch | `00 21` WMT1 Wave Switch | identical |
| `01`, `02`, `03`–`04`# | Wave Group, Group ID, Wave Number (raw v = Wave v+1) | `00 22`, `00 23`#, `00 27`# WMT1 | table: `jv-wave-map.json`. Write 0 to Wave Number R (`00 2B`). |
| `05` | Wave Gain | `00 2F` WMT1 Wave Gain | identical |
| `06` | Bender Range (0–12) | `00 1C` (0–48) | identical |
| `07` | Mute Group | `00 0D` | identical |
| `08` | Envelope Mode | `00 15` | identical |
| `09` | Volume Control Switch (CC7) | `00 1D` Receive Expression | identical value, approximate meaning (Roland did the same) |
| `0A` | Hold-1 Control Switch | `00 1E` | identical |
| `0B` | Pan Control Switch (OFF, CONTINUOUS, KEY-ON) | — (Integra reserve `00 1F` = XV Receive Pan Mode) | **none**; report OFF or KEY-ON |
| `0C` | Source Key (0–127) | `00 0F` Partial Coarse Tune (0–127) | identical |
| `0D` | Fine Tune (0–100) | `00 10` (14–114) | `i = v + 14` |
| `0E` | Random Pitch Depth | `00 11` | identical |
| `0F` | P-ENV Depth (0–24) | `01 15` (52–76) | `i = v + 52` |
| `10`, `1D`, `1F`, `2A` | P-ENV / Resonance / F-ENV / A-ENV Velocity Sens (0–125 = −100…+150 %) | `01 16`, `01 27`, `01 2A`, `01 37` (±63) | **rescale**: Roland's table (§4) |
| `11`, `20`, `2B` | P / F / A-ENV Velocity Time Sens (`T14`, 0–14) | `01 17`+`01 18`, `01 2B`+`01 2C`, `01 38`+`01 39` | **rescale**, the same value into Time 1 and Time 4 (§4) |
| `12`–`15`, `21`–`24`, `2C`–`2F` | P / F / A-ENV Time 1–4 | `01 19`–`1C`, `01 2D`–`30`, `01 3A`–`3D` | identical |
| `16`–`19` | P-ENV Level 1–4 (0–126) | `01 1E`–`01 21` (1–127) | `i = v + 1`; Level 0 `01 1D` = 64 |
| `1A`, `1B`, `1C` | Filter Type, Cutoff, Resonance | `01 22`, `01 23`, `01 26` | identical |
| `1E` | F-ENV Depth (0–126) | `01 28` (1–127) | `i = v + 1` |
| `25`–`28` | F-ENV Level 1–4 | `01 32`–`01 35` | identical; Level 0 `01 31` = 0 |
| `29` | Tone Level | `00 0E` Partial Level | identical (WMT1 Wave Level = 127) |
| `30`–`32` | A-ENV Level 1–3 | `01 3E`–`01 40` | identical |
| `33`, `34`, `35` | Tone Pan, Random Pan Depth, Alternate Pan Depth | `00 12`, `00 13`, `00 14` | identical; set WMT1 Random Pan and Alternate Pan Switch ON |
| `36` | Output Assign (MIX, EFX, OUTPUT1, OUTPUT2) | `00 1B` | **none**: write 0 (PART); report EFX and direct outs |
| `37` | Mix/EFX Send Level | `00 16` Partial Output Level | identical |
| `38`, `39` | Chorus / Reverb Send Level | `00 19`, `00 1A` | identical |

Counts over the 57 Note parameters: identical 38 (37 exact; Volume Control Switch → Receive
Expression is approximate), offset 7, table 3 (the Wave), rescale 7, none 2.

**What a JV Rhythm Setup doesn't carry.** It has no EFX, Chorus, Reverb or level. In
Performance mode those come from the Performance. So the kit MFX is THRU (type 0), Kit Level is
127, and any Note with Output Assign = EFX loses its EFX. Report that per kit: it affects 91
Notes in the corpus.

**Integra fields with no JV source, and how Roland filled them** in its own conversion
(Orch Set, §4):

- Assign Type MULTI, WMT Velocity Control ON.
- WMT1: Wave Level 127, Velocity Range 1–127, fades 0, Coarse/Fine/Pan 64, FXM OFF,
  Tempo Sync OFF, Random Pan Switch ON, Alternate Pan Switch ON. WMT2–4 OFF.
- TVF Cutoff Velocity Curve FIXED with Sens 64. F-ENV and TVA Velocity Curve 1.
- Pitch Env Level 0 = 64, TVF Env Level 0 = 0.
- Integra-only fields: Partial Output Assign PART, One Shot OFF, reserve `01 42` = 64,
  Comp/EQ switches OFF.

## 3. XV Rhythm Set → PCM Drum Kit

### Rhythm Common (`00 00 00`, 17 parameters)

| XV off. | XV parameter | Integra at that offset | Mapping |
|---|---|---|---|
| `00`–`0B` | Rhythm Name 1–12 | Kit Name 1–12 | identical; pad into Common 2 long name |
| `0C` | Rhythm Level | Kit Level | identical |
| `0D` | Rhythm Clock Source (RHYTHM, SYSTEM) | unnamed | none: copy verbatim (corpus: always 0) |
| `0E`# | Rhythm Tempo (20–250) | unnamed # | none: copy verbatim. Integra `INIT RHYTHM` holds 120 here, the XV default. |
| `10` | One Shot Mode | unnamed | **table**: copy, and also write it to every key's Partial `01 41` One Shot Mode |
| `11` | Rhythm Output Assign (MFX, A–D, 1–8, TONE) | reserve (always 13) | none: copy verbatim (corpus: always 13 = TONE) |

### MFX, Chorus, Reverb

- **Common MFX** has the same block, size and offset as the Patch Common MFX. It follows
  [`xv-patch-format.md`](xv-patch-format.md) and the type table in
  [`foreign-fx-map.md`](foreign-fx-map.md): MFX Type goes through a table, Dry Send and Output
  Assign have no target, and the other 46 parameters copy.
- **Chorus and Reverb** (16 and 23 parameters) have no home in a Drum Kit, just as in a Patch.
  They are Studio Set-level and appear as `NotApplied` in the Conversion report
  ([#199](https://github.com/sigma/integral/issues/199)).

### Rhythm Tone → Partial (157 parameters)

All offsets match. The machine diff found differences only in these rows:

| Offset | XV | Integra | Status | Conversion |
|---|---|---|---|---|
| `00 16` | Tone Dry Send Level | Partial Output Level | identical | copy |
| `00 17`, `00 18` | Tone Chorus/Reverb Send Level (MFX path) | reserve | none | Copy verbatim. If Tone Output Assign = MFX, write this pair into `00 19`/`00 1A`. |
| `00 19`, `00 1A` | Tone Chorus/Reverb Send Level (non MFX) | Partial Chorus/Reverb Send | identical | copy (the right pair when Output Assign is A–D/1–8) |
| `00 1B` | Tone Output Assign (MFX, A–D, 1–8; 0–12) | Partial Output Assign (PART, COMP+EQ1–6; 0–6) | **table** | Write 0 (PART). Report a bypassed MFX when Output Assign ≠ MFX and MFX Type ≠ THRU. |
| `00 1F` | Tone Receive Pan Mode | reserve | none | Copy verbatim; report KEY-ON. |
| `00 22`, `00 3F`, `00 5C`, `00 79` | WMT1–4 Wave Group Type (INT-WAVE, SRJV80-EXP, SRX-EXP, SAMPLE) | (INT, SRX, —, —) | **table** | 0 → 0, 2 → 1. 1 (SR-JV80) goes through `jv-wave-map.json`, else OFF. 3 (SAMPLE) → OFF. |
| — | (kit-level One Shot Mode) | `01 41` One Shot Mode | table | from XV Common `00 10` |
| — | — | `01 42` reserve | Integra only | 64 |

Counts: 149 identical, 5 table, 3 none. Wave Group ID and Wave Number copy for INT and SRX.
Only SR-JV80 Waves need #195's map. The Integra keeps the XV's "(MFX)" send pair as reserves.
This is the same pattern as the Patch ([#193](https://github.com/sigma/integral/issues/193)).

**So, is it byte-identical?** The layout is: same blocks, offsets, widths and ranges. A
byte-for-byte copy is *not* safe, because of Tone Output Assign. Apart from that, a converter
copies the Common, MFX and Tone blocks, then patches 5 kinds of field (`00 1B`, the four Group
Types, the sends when Output Assign = MFX, and One Shot into `01 41`). It drops Chorus and
Reverb, and fills Comp/EQ (all OFF) and Common 2 (long name, Phrase Number 0, TFX Switch).

## 4. Roland's own JV → XV conversion

`archive-org-RolandXVYahooGroup/…/SR-JV80-16 Orchestral II/OrchII-1.syx` holds the SR-JV80-16
board's "Orch Set" as a JV Rhythm Setup. `bwalk-com-au/unpacked/XV SRJ to SRX-06/SRJV16R.mid` is
Roland's 2002 XV-format re-issue of the same set. All 64 keys were compared field by field.

- **Offsets confirmed:** Fine +14, P-ENV Depth +52, P-ENV Levels +1, F-ENV Depth +1, and every
  identical field.
- **Velocity Sensitivity** (JV % = 2v − 100 → Integra/XV i − 64). These are all the data points.
  Every one of them is consistent across the four Velocity Sensitivity fields:

  | JV % | 0 | 30 | 60 | 70 | 80 | 86 | 92 | 100 | 110 | 112 | 140 |
  |---|---|---|---|---|---|---|---|---|---|---|---|
  | Integra | 0 | +9 | +19 | +22 | +25 | +27 | +29 | +32 | +35 | +36 | +52 |

  This is roughly `0.32 × %` up to +112 %, then steeper. It is a lookup table, not
  `pct × 63/100`. Negative values and +114…+150 % were not observed.
- **Velocity Time Sensitivity** (`T14` %): 0 → 0, +10 → +6, +20 → +12, +40 → +25. Roland wrote the
  same value to Time 1 and Time 4.
- **Raw 50 is the neutral value.** It is the most common value of every Velocity Sens field in
  the corpus (e.g. 2,978 of 3,183 P-ENV Velocity Sens). So the JV-1010/XP-30 display
  "−100…+150" is right. The JV-1080 manual (P1, pp. 153–154) prints "−50 − +200", which is a
  misprint.
- **Waves:** 61 of 64 keys agree with `jv-wave-map.json` (52 high, 6 medium and 3 low
  confidence, including the low-confidence INT-B 14 Agogo → INT 498). On the other 3, Roland
  used other hi-hats: INT-B 89 `Cl HiHat 1` → 790 `Cl HiHat 4` (map: 787), INT-B 91 `Op HiHat`
  → 798 `Op HiHat 2` (map: 797), and INT-B 92 `Pedal HiHat` → 804 `Pedal HiHat2` (map: 803). The
  map's same-name targets still stand.
- **Routing:** JV Output Assign MIX became XV output A, and MFX was THRU. Pan Control
  CONTINUOUS became Receive Pan Mode CONTINUOUS. Volume Control Switch became Receive Expression.

## 5. Corpus check

Read-only scan of every `.syx` and `.mid` in both folders (1,458 files). SMF files were walked by
chunk and event, joining `F7` continuation packets. Only checksum-valid DT1s were kept.

### JV/XP

| Measure | Value |
|---|---|
| Rhythm Setups (Common + Notes) | 184, **50 distinct** (32 names), first seen in 45 files |
| By area | Temporary `02 09` 69 · User `10 40` 54 · `10 41` 54 · Card `20 40` 7 |
| Notes per setup | 64 (one setup in `TB303DM.MID` has 61) |
| DT1 lengths | Common always 12, Note always 58: one block per message |
| Range check | every byte of the 3,197 Notes is within P1/P2 ranges |
| Active Notes | 3,183, on keys 35–98 |
| Wave Group (Type, ID) | INT-B 2,265 · INT-A 243 · EXP-06 524 · EXP-05 61 · EXP-01 50 · EXP-16 38 · EXP-04 2 |
| Wave coverage (`jv-wave-map.json`) | high 2,018 · medium 426 · low 207 · **no list 532** (SR-JV80-06 524; SR-JV80-01 Waves 170/171/177 are past its 154-entry list, 8) |
| Kits that map completely | 38 of 50 |
| Targets | INT 2,547 · SRX-09 61 · SRX-06 41 · SRX-07 2 |
| Envelope Mode SUSTAIN | 317 |
| Pan Control ≠ CONTINUOUS | 92 (OFF 66, KEY-ON 26) |
| Output Assign = EFX | 91 |
| Velocity Time Sens ≠ 0 | P 96 · F 54 · A 441 Notes |

### XV

| Measure | Value |
|---|---|
| Rhythm Sets | 79, **62 distinct** (58 names): 11 from the Yahoo group, 51 from bwalk's SR-JV80 → XV re-issues |
| By area | User `40 x0` 71 · Temporary part 10 `13 30` 4 · Patch-mode `1F 10` 2 · parts 6 and 8: 1 each |
| Blocks | all four Common blocks always present; 88 Tones (2 Fantom-build SRX-03 sets carry 12) |
| DT1 lengths | 18 / 145 / 52 / 83 / 193: one block per message |
| Range check | every active key is in range. One inactive key (`XV 5080 All.mid`, key 22) has zeroed WMT3/4 data with Coarse Tune 0. Clamp or default the fields of switched-off WMTs. |
| Active Tones | 3,933, keys 28–99; WMTs on per key: 1 → 3,771, 2 → 112, 3 → 32, 4 → 18 |
| Wave Group (Type, ID) | INT (0, **1**) 1,421 WMTs · SRX (2, board 3/6/7/8/9) 2,742 · SR-JV80 0 · SAMPLE 0 |
| Max Wave Number | within each Integra group's count (INT 1,081; SRX-03 426, -06 457, -07 830, -08 887, -09 852) |
| Tone Output Assign | A 3,878 · MFX 55 |
| Send pairs differ (MFX vs non-MFX) | 64 Tones |
| Receive Pan Mode KEY-ON | 9 Tones |
| Common | Clock Source 0 (all) · Tempo 120 (60), 91 (2) · One Shot ON 2 · Output Assign 13 (all) |
| MFX Type / Dry Send | THRU 58, types 1, 7, 77 (×2) · Dry Send 127 (60), 0 (2) |
| Chorus / Reverb Type | Chorus 1 (59), 2 (3) · Reverb 1–4 |

**Counts vs the map.** [#197](https://github.com/sigma/integral/issues/197) reports "Rhythm sets
82" (JV) and "142" (XV). This scan finds 184 / 79 complete sets with valid checksums, of which
50 / 62 are distinct. #197 doesn't say how it counted Rhythm sets. Its Patch counts used the
first block only, which would explain the gap. The figures here count complete,
checksum-valid sets.

## Open doubts

1. **Velocity Sensitivity table.** Roland's points cover 0…+140 % from one kit. Negative and
   +114…+150 % values need more pairs. Roland's other XV re-issues of SR-JV80 data
   (`bwalk-com-au` `SRJVnnPA/PB/RA/RB.mid`, against JV-format board dumps in the Yahoo-group
   folder) would give more. That same method answers [#194](https://github.com/sigma/integral/issues/194)'s
   open doubt 1 for Patches. #194's candidate `pct × 63/100` disagrees with Roland: Roland maps
   +100 % to +32, where the candidate gives +63.
2. **XV Wave Group ID for INT is 1, not 0.** Every INT WMT in the XV corpus has ID 1, and real
   Integra tones also store 1 (`docs/svd/06-pcm-synth.md`, "Wave Group ID = 1").
   `xv-patch-format.md` and `wave-names.md` once said INT is 0; both are corrected
   ([#207](https://github.com/sigma/integral/issues/207)). Copy the ID verbatim. The SRX ID is the board number, which settles #193's open doubt 3
   for SRX.
3. **One Shot Mode.** It is per kit on the XV (Common `00 10`) and per key on the Integra
   (`01 41`). Whether the Integra still reads its Common `00 10` is unknown; write both.
4. **Partial Output Assign PART and the MFX.** The JV Rhythm Setup has no EFX, and most XV kits
   use output A with MFX THRU, so PART loses nothing for them. Where an XV MFX was set but tones
   bypassed it (output A), PART puts them through the MFX. Report it. Fixing it would be a
   Conversion hook.
5. **Fantom.** Two SRX-03 "Fantom" builds use the XV layout with only 12 keys sent. Treat a
   missing key as OFF.
6. **SR-JV80-01 Wave Numbers 170, 171 and 177** are past #195's 154-entry list for that board.
   Either the list is short or EXP ID 1 is another board in those files.

## Confidence

- **High:**
  - framing, addresses, block sizes and key ranges (five XV manuals and three JV manuals
    agree; the whole corpus checks out);
  - the XV Tone offset identity (machine diff);
  - the JV offsets and identical fields (confirmed by Roland's own conversion).
- **Medium:**
  - the fill values for Integra-only fields (taken from one Roland conversion);
  - the PART choice for Output Assign.
- **Low to medium:** the Velocity Sensitivity and Velocity Time tables. They are Roland's own
  numbers, but come from a single kit.
