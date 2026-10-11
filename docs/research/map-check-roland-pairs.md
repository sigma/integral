# Wave and MFX maps checked against Roland's paired conversions (research for #206)

Answers [#206](https://github.com/sigma/integral/issues/206) (parent map
[#192](https://github.com/sigma/integral/issues/192)): do Roland's own JV → XV Patch conversions
agree with the Wave map ([`jv-wave-map.json`](jv-wave-map.json), from
[#195](https://github.com/sigma/integral/issues/195)) and the MFX map
([`foreign-fx-map.json`](foreign-fx-map.json), from
[#196](https://github.com/sigma/integral/issues/196))? Both JSON files were updated in place by
this check. The pairs and their families come from
[`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md).

## TL;DR

- **Wave map: the pairs cover 1,902 of 4,241 source Waves.** 1,860 are confirmed: 1,739 by
  Roland's own SR-JV80 → XV sets (R02), now all **high**, and 121 by the cross-check families
  only. **42 are changed**, and **2,339 are not covered**. The JV/XP internal ROM → INT map holds:
  237 of 255 INT-A and 121 of 193 INT-B Waves are confirmed, with only 2 cross-check
  contradictions.
- **The main Wave error was in MENU Waves.** The SRX-08 and SRX-09 boards merge several SR-JV80
  boards. Each board's `Kick MENU 1`, `Snare MENU 1` and so on became a separately numbered SRX
  Wave (`Kick MENU 5`, `Snare MENU 9`). Matching by name sent them to another board's MENU Wave,
  or left them unmapped. This accounts for 29 of the 42 changes, and 12 of them were graded high. Other fixes:
  `Tron Str A/B/C` (SR-JV80-08) is SRX-07 `Tron Str2`, not `Tron Str`, and 14 Waves the map had
  left unmapped now have Roland's target (source-list typos such as `Hey Tribe` = `How Tribe`
  and `.HornSolo2C`, plus `Org Rhodes` = `Org EP`).
- **MFX map: 37 of the 40 JV EFX types are confirmed** (XV type = JV raw + 1, in 2,506/2,506 R02
  Patches). 3 are not covered: OD→CHORUS, DS→FLANGER and FLANGER/DELAY. No Integra-side row
  could be checked, because the XV keeps the JV encoding (next point), so **0 rows changed**.
- **Correction to the MFX map's XV section:** the XV does **not** use the Integra encoding for
  types 1–40. Roland copies the 12 JV EFX parameter bytes **verbatim** into XV MFX
  Parameters 1–12 (32768 offset), and Parameters 13–32 are 0. XV-native Patches of these types
  stay within the JV raw ranges too. So an XV Patch with MFX type 1–40 converts with the **JV
  rows** (`jv_prm` = XV Parameter *n*). The map's "the JV formulas do not carry over raw" is
  wrong. The XV 41–90 encoding is still unverified.
- **One open contradiction:** STEREO-EQ High Freq. Roland copies JV raw 0/1 unchanged, while the
  map adds 1. XV-native Patches use 0–2 here. Kept as is and flagged in the JSON (see
  [MFX map](#mfx-map)).
- **Chorus and Reverb:** Roland turns the JV Patch Chorus into XV **CHORUS type 1** (raw copy)
  and the JV Reverb into XV **REVERB type 1**, whose Parameter 1 is the JV reverb type. So the
  JV chorus is the XV/Integra *Chorus* type, not the GM2 Chorus substitute that
  `foreign-fx-map.md` suggests for a future Studio Set slice.

## Sources

| # | Source | What it gives |
|---|--------|---------------|
| R02 | `~/Downloads/integral-corpus/bwalk-com-au/unpacked/XV SRJ to SRX-0{6,7,8,9}/SRJVnnP{A,B}.mid`: Roland's 2002 XV-format SR-JV80 Patch sets, paired with the SR-JV80 board dumps from the XV Yahoo-group archive | Roland's own SR-JV80 → SRX Wave choice and JV EFX → XV MFX conversion. **Authoritative.** |
| J2XV, JVXP, X5050 | `…/RolandXVYahooGroup/…/XV Series Patches/…` and `…/xv5050 factory patches/…`, paired with JV dumps from both corpora | Roland factory data in XV format, converter unknown. **Cross-check only.** |
| D | [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md) (families, pairing rule, hand edits), [`jv-patch-format.md`](jv-patch-format.md), [`xv-patch-format.md`](xv-patch-format.md) / [`.csv`](xv-patch-format.csv), [`docs/waves/*.json`](../waves/) | Field offsets, Wave names |

## Method

1. Re-ran the pairing from [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md) with a stdlib
   script: raw `.syx` split into `F0…F7` frames, SMF walked chunk by chunk (running status, `F0`
   and `F7` events), DT1 checksums verified, blocks grouped per Patch address. Pairs are
   matched by name, and the best JV candidate must agree on ≥ 90 % of 25 per-tone fields. The
   result is the same: R02 2,506, J2XV 538, JVXP 192, X5050 381 distinct pairs. *misc* (1,042)
   is ignored.
2. **Waves.** For each JV-active tone: source = JV Wave Group Type/ID/Number (`01`–`04`, number
   + 1). XV side = Partial `27` Wave Group Type, `28` Group ID and `2C` Wave Number (4 nibbles
   each). Each source Wave was compared with its map entry, taking R02's target where R02 has
   one and the cross-check majority otherwise.
3. **Regrade rules** (written into the JSON `confidence` and `evidence_codes`):
   - R02 uses the map's target → **high**, evidence `roland-pair`.
   - Only cross-check families use it → one grade up (low → medium → high), evidence
     `roland-pair-crosscheck`.
   - The pairs use a different target → the target is replaced and the old one kept under
     `previous`. The new grade is **high** with ≥ 2 R02 tones, and **medium** with 1 R02 tone or
     cross-check evidence only.
   - A Wave whose name contains MENU, which another board's same-named MENU Wave already
     takes, is downgraded to low (`target-collision`), unless the pairs confirm it.
   - Every entry the pairs cover carries `roland_pairs`: tones per family and XV target.
4. **Effects.** Compared JV Patch Common `0C` EFX Type and `0D`–`18` Parameters 1–12 with XV
   MFX `00` Type and Parameters 1–32 (4 nibbles − 32768). The script searched every XV
   parameter for each JV parameter, then compared the map's predicted Integra value with the
   XV value. It also compared the sends (`1A`–`1C`), Chorus (`21`–`26` → XV `00 04 00`) and
   Reverb (`27`–`2B` → XV `00 06 00`).

### Hand edits excluded

- **Waves:** in two R02 Patches on SR-JV80-04 (`Prophetic`, `MKS80 Sprang`, `SRJV04PA.mid`),
  Roland replaced 5 JV *internal* Waves with SRX-07 synth Waves (INT-A 233/234 `DualSquare A/C`
  → SRX-07 544/485, INT-B 2 `Marimba Wave` → 543, INT-B 44 `Synth Pulse2` → 516 ×2). These are
  revoicing, not a mapping, and were left out. The same Waves map to INT everywhere else.
- **Effects:** R02 has none. All 2,506 Patches keep the type and all 12 parameter bytes.
- **Reverb sends on boards 08, 11 and 19** ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md))
  are tone send levels. Neither map holds tone send levels, so they don't affect this check.
- **Cross-check noise, not used to change anything:** 3 single tones in X5050/J2XV (`Mini Bs 2`
  → INT 270, `Syn Vox 2` → INT 456), and 5 J2XV/X5050 Patches whose STEREO-EQ became another
  type. Single-parameter differences in 4 J2XV/X5050 Patches look like source-version
  differences.

## Wave map

Counts are source Waves. "Confirmed" means the target is unchanged. "Raised" counts confirmed
entries whose grade went up.

| Source group | Waves | Confirmed by R02 | Confirmed by cross-check only | Changed | Not covered | Raised | Now high/medium/low/none |
|---|---:|---:|---:|---:|---:|---:|---|
| JV-INT-A | 255 | 148 | 89 | 0 | 18 | 10 | 255/0/0/0 |
| JV-INT-B | 193 | 89 | 32 | 2 | 70 | 12 | 168/23/2/0 |
| SR-JV80-01 | 154 | 0 | 0 | 0 | 154 | 0 | 0/59/35/60 |
| SR-JV80-02 | 174 | 125 | 0 | 0 | 49 | 18 | 166/8/0/0 |
| SR-JV80-03 | 73 | 0 | 0 | 0 | 73 | 0 | 10/6/18/39 |
| SR-JV80-04 | 255 | 183 | 0 | 0 | 72 | 1 | 255/0/0/0 |
| SR-JV80-05 | 255 | 179 | 0 | 6 | 70 | 24 | 242/10/1/2 |
| SR-JV80-07 | 255 | 0 | 0 | 0 | 255 | 0 | 73/3/18/161 |
| SR-JV80-08 | 255 | 176 | 0 | 4 | 75 | 8 | 248/6/0/1 |
| SR-JV80-09 | 206 | 0 | 0 | 0 | 206 | 0 | 96/0/3/107 |
| SR-JV80-10 | 241 | 118 | 0 | 1 | 122 | 13 | 241/0/0/0 |
| SR-JV80-11 | 255 | 118 | 0 | 10 | 127 | 5 | 247/8/0/0 |
| SR-JV80-12 | 255 | 153 | 0 | 3 | 99 | 3 | 254/1/0/0 |
| SR-JV80-13 | 82 | 71 | 0 | 0 | 11 | 1 | 82/0/0/0 |
| SR-JV80-14 | 175 | 137 | 0 | 1 | 37 | 4 | 173/2/0/0 |
| SR-JV80-15 | 248 | 0 | 0 | 0 | 248 | 0 | 185/0/2/61 |
| SR-JV80-16 | 153 | 128 | 0 | 1 | 24 | 3 | 153/0/0/0 |
| SR-JV80-17 | 247 | 0 | 0 | 0 | 247 | 0 | 183/5/5/54 |
| SR-JV80-18 | 255 | 0 | 0 | 0 | 255 | 0 | 248/3/3/1 |
| SR-JV80-19 | 255 | 114 | 0 | 14 | 127 | 1 | 247/8/0/0 |
| **Total** | **4,241** | **1,739** | **121** | **42** | **2,339** | **103** | |

R02 adds 6,836 tone observations, and the cross-check families add 3,084. The 11 boards with R02
pairs are 02, 04, 05, 08, 10, 11, 12, 13, 14, 16 and 19. Boards 01, 03, 07, 09, 15, 17 and 18
have no pairs (see [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)), so their partial re-issues
remain unchecked. Within R02, every tone of a confirmed Wave agrees: no Wave got two different
targets, apart from the hand edits above.

### The 42 changes

| Kind | Count | Examples (source → old → Roland's) |
|---|---:|---|
| MENU Wave renumbered on a merged SRX board (12 were high, 7 unmapped) | 29 | SR-JV80-19 `Kick MENU 1` → `Kick MENU 1` 472 → **`Kick MENU 5` 532**. -19 `Snare MENU 1/2` → **`Snare MENU 9/10`**. -19 `Voice MENU 1/2` → **`Voice MENU 4/5`**. -19 `Perc. MENU 1–5` → **`Perc. MENU 2–6`**. -11 `HIT MENU 1/2` → **`Hit MENU 3/4`**. -11 `SNR MENU 1–5` (map: SRX-07 `SNR MENU`, low, or none) → **SRX-08 `Snare MENU 4–8`**. -11/-12 `TOM MENU` (map: SRX-07 809, low) → **SRX-08 `Tom MENU 2`/`Tom MENU 1`**. -05 `TablaBy MENU` → **`TablaByMENU2`** |
| Same name on two boards of one SRX | 4 | SR-JV80-08 `Tron Str A/B/C` → `Tron Str` 274–276 (that is SR-JV80-04's) → **`Tron Str2` 277–279**. -12 `Gtr FX MENU` → `GTR FX MENU` 206 (-11's) → **`Gtr FX MENU` 207** |
| Unmapped → mapped (source-list typo or rename) | 7 | -05 `Aftro Zither` → **`Afro Zither`**, `Hey Tribe` → **`How Tribe`**. -08 `Wurly 1 p C` (repeat) → **`Wurly 1 f C`** 55. -10 `SNR SET 3f` (repeat) → **`SNR SET 8f`**. -12 `Org Rhodes` → **`Org EP`**. -14 `Sitar/Drone` (repeat) → **155**. -16 `.HornSolo2C` → **`F.HornSolo2C`** |
| JV internal → INT (cross-check only) | 2 | INT-B 76 `Dry Stick` → INT 713 `Dry Stick` → **714 `Dry Stick 2`** (J2XV, 2 tones). INT-B 91 `Op HiHat` → 797 → **798 `Op HiHat 2`** (J2XV, 1 tone). Medium: the J2XV converter is unknown. |

Of the 28 entries that were retargeted, 17 had been graded high. That is the cost of name-only
matching on boards that repeat generic MENU names. Five changes move a Wave to another SRX
board (SRX-07 → SRX-08 for the -11/-12 Tom and Snare MENUs). This changes which Expansion the
Conversion report asks for. The previous answer for each change is kept in the JSON under
`previous`.

**What it means for the uncovered entries:** on merged boards, a generic MENU name is not
enough to identify a Wave. 7 uncovered MENU entries on SR-JV80-17/-18 collide with another
board's MENU Wave that the pairs confirm, and are now low (`target-collision`). The 35
uncovered MENU entries still graded high or medium have names unique to one board (`Biwa MENU`,
`Conga Menu`, `EG Bend Menu`, …). They are lower risk, but no pair checks them.

### XV Wave Group fields (side findings)

- **SRX Group ID = SRX board number**: confirmed by every R02 tone (Group Type 2, IDs 6–9). The
  converted R02 Patches never use Group Type 1 (SR-JV80): Roland moved every board Wave onto SRX.
- **SR-JV80 Group ID on the XV = the JV EXP board number**, with the Wave Number unchanged. This
  closes the open doubt in [`jv-wave-map.md`](jv-wave-map.md). It is seen in 278 *misc* and
  JVXP tones (boards 2, 4, 9, 11, 17, plus one stray ID 76 copied as-is): `JV8090-User`,
  `JV1010-User`, `XP30-User`, `srjv17_*` and `Roland-Int00`. Medium confidence, because none of these is a Roland-issued conversion.
- **XV INT tones carry Wave Group ID 1**: 5,896 of the 6,315 INT tones (all four tones per
  Patch) in R02/J2XV/JVXP/X5050. The other 419 (X5050, plus one J2XV) carry 0. This agrees with
  [#207](https://github.com/sigma/integral/issues/207) (on `main`): copy the INT Group ID as is.

## MFX map

| Item | Confirmed by R02 | Changed | Not covered |
|---|---:|---:|---:|
| JV EFX type → XV MFX type (`xv_type` = JV raw + 1) | 37 types, 2,506/2,506 Patches | 0 | 3 (OD→CHORUS, DS→FLANGER, FLANGER/DELAY) |
| JV EFX parameter, source side (raw value and slot on the XV) | 311 of 336 parameter rows: all 12 bytes verbatim | 0 | 25 rows of the 3 uncovered types |
| JV → Integra parameter rows (`integra_params`, `conv`) | 0: not testable, the XV keeps the JV encoding | 0 | all |
| XV 41–90 rows | — | — | 50 |
| EFX Chorus/Reverb Send → MFX `02`/`03` | 2,506/2,506 identical | 0 | — |
| EFX Mix Out Send → XV MFX Dry Send `01` (Integra reserve) | 2,506/2,506 identical, so the loss stands | 0 | — |

What changed in [`foreign-fx-map.json`](foreign-fx-map.json):

- `_about` now states the verbatim rule for XV types 1–40, in place of "XV raw parameter
  encoding is unverified".
- Each JV type has `roland_pairs` (Patches per family, `xv_type`, `xv_params`, exceptions) and
  `confidence` (high for the 37 confirmed types). Each parameter has `xv_param` (= `jv_prm`).
- STEREO-EQ High Frequency carries a `roland_pairs` note on the open contradiction below.
- A new top-level `roland_pairs_sends_and_patch_effects` records the send, Chorus and Reverb
  findings.

**STEREO-EQ High Freq (open).** Roland writes JV raw 0/1 (4 kHz / 8 kHz) unchanged into XV
Parameter 3 (1,790 Patches, all families). The map writes Integra raw + 1 (2/4/8 kHz list).
XV-native Patches use 0, 1 and 2 (21 use 2), so the XV list has three entries.

- If the XV list is ordered 2000/4000/8000 Hz like the Integra's (X1 suggests so), Roland's
  conversion drops the EQ an octave, and the map's +1 is the parameter-faithful choice.
- If not, XV import needs its own rule for this one parameter.

The map is unchanged. Settle this on hardware or from an XV-native Patch with a known setting.

**Chorus and Reverb.** These are not in the JSON rows (the Studio Set slice is future work). In
all 2,506 R02 Patches:

- **Chorus:** XV Chorus Type = 1 CHORUS. Level and Output Select copy. Chorus Parameters 1–4 =
  JV Rate, Depth, Pre-Delay, Feedback raw (127 clamps to 125: 11 and 15 Patches). Parameters 5–8
  are fixed at 0, 7, 90, 0, consistent with filter OFF and phase 90.
- **Reverb:** XV Reverb Type = 1 REVERB. Level copies. Reverb Parameter 1 = JV Reverb Type
  (ROOM1 … PAN-DLY), and Parameters 2–4 = JV Time, HF Damp and Delay Feedback.

`foreign-fx-map.md` classes the JV chorus as a GM2 Chorus substitute, but Roland's route is
JV chorus → XV CHORUS → (map) Integra Chorus type 1. The JV reverb finding agrees with the map.

## Confidence

| Finding | Confidence | Basis |
|---|---|---|
| R02-confirmed Wave targets | **High** | Roland-issued conversion, 6,836 tones, no internal contradiction |
| MENU / same-name corrections with ≥ 2 R02 tones | **High** | Roland's own choice, repeated |
| Corrections from 1 R02 tone (19) | **Medium** | One Roland tone. A hand edit can't be ruled out, but the MENU renumbering pattern is consistent across 29 entries |
| INT-B `Dry Stick` / `Op HiHat` → the "2" Waves | **Medium** | J2XV only (3 tones). Plausible: the XV adds new same-named drum Waves |
| XV types 1–40 store JV EFX bytes verbatim | **High** | 2,506/2,506 R02 Patches, 12/12 bytes. XV-native Patches stay in JV ranges (one exception: EQ High Freq) |
| JV → Integra MFX parameter rows | unchanged | The pairs can't reach the Integra side |
| SR-JV80 Group ID on XV = board number | **Medium** | 278 non-Roland-issued tones, 0 contradictions |

## Not covered

- 2,339 source Waves: every Wave of boards 01, 03, 07, 09, 15, 17 and 18 (no pairs), and Waves
  no paired Patch happens to use.
- 3 JV EFX types and all of XV MFX 41–90. The Integra side of every MFX row.

Scratch scripts (Python stdlib, in `/tmp`) were not kept.
