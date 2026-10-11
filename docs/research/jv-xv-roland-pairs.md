# Roland's own JV → XV Patch conversion, read from paired dumps (research for #201)

Fact-finding for [#201](https://github.com/sigma/integral/issues/201) ("Decide the JV/XP
conversions Roland gives no formula for"), part of the Foreign patch map
[#192](https://github.com/sigma/integral/issues/192). Roland published no JV → XV formulas, but it
did ship JV-format Patches converted to XV format. The two versions of the same Patch show how
Roland itself converted each field. The XV Patch layout is the INTEGRA-7 PCM Synth tone layout
([`xv-patch-format.md`](xv-patch-format.md)), so Roland's XV values work directly as Integra
values.

Terms and JV offsets follow [`jv-patch-format.md`](jv-patch-format.md). In this note, `v` is the
JV raw byte and `i` is the XV/Integra raw byte.

## TL;DR

- **The pairs exist, and there are many of them.** 3,617 distinct JV/XV Patch pairs come from
  four converted sets. Roland's 2002 SR-JV80 → XV set alone gives 2,506. The JV side and the XV
  side agree byte for byte on every "identical" field.
- **Velocity sensitivity** (P-Env, Resonance, F-Env, Level): JV −100…+100 % maps
  **linearly onto only ±32**: `i = 64 + trunc((v − 50) × 16/25)`. JV +100…+150 % then **stretches
  over +32…+63** through a fixed table (below). Nothing is clipped. XV values below 32 (−33…−63)
  are never produced.
- **Velocity time sensitivity** (`T14`): `i = 64 + trunc(pct × 63/100)`, so ±100 % = ±63.
- **Controllers:** Roland packs routes **tone by tone, greedily**. Each new
  (source, destination, sens) goes into the first Matrix Control with that source that already
  has it (ON) or its negative (REVERSE). Failing that, it goes into a free slot, and failing that,
  into the next free Matrix Control. Empty controllers take no Matrix Control, and controllers
  that share a source share Matrix Controls. Routes that don't fit once 4 Matrix Controls are in
  use are **dropped**, so a drop always comes from a later tone. This rule reproduces
  **3,603 of 3,617** pairs exactly. It drops routes in 73 pairs (2 %); in the owner's seed corpus
  it would drop routes in 133 of 2,624 Patches.
- **Confirmed:** JV SAW → **SAW-UP**. The implicit start levels are **Pitch L0 = 64, TVF L0 = 0**.
  KEY-INTERVAL/PLAYMATE → NORMAL with delay time 0.
- **The table in `jv-patch-format.md` is wrong in these places:** Volume Control Switch is **not**
  carried (Receive Expression is always ON). Pan Control Switch **is** carried (KEY-ON → Receive
  Pan Mode KEY-ON). CLOCK-SYNC/TAP-SYNC delay and LFO sync **are** converted to note values.
  Velocity Range Switch OFF resets the ranges. JV SYS-CTRL1/2 → **CTRL2/CTRL3**, not CTRL1/2.
  Source indices follow CC numbers (BEND = 96).

## Sources

| # | Source | What it gives |
|---|--------|---------------|
| R | `~/Downloads/integral-corpus/bwalk-com-au/unpacked/XV SRJ to SRX-0{6,7,8,9}/SRJVnnP{A,B}.mid`: Roland's 2002 XV-format SR-JV80 Patch sets (see `integral-corpus/MANIFEST.md`) | The XV side of family **R02** |
| J | `…/archive-org-RolandXVYahooGroup/…/JX-XP-XV-SRJV80-SRX Factory patches/SRJV80-SRX Expansion/SR-JV80-nn */*.syx` (Model `6A`) | The JV side of R02: the original board Patches |
| Y1 | `…/XV Series Patches/JV-2080 Patches (via User XV).zip.unpacked/JV-2080 Pr{A..E} Patch 001-128 (via User XV).syx` | XV side of family **J2XV** (JV-2080 presets in XV format) |
| Y2 | `…/XV Series Patches/XV-5080_JVXP_banks.zip.unpacked/{Roland-Int00,JV2080-User,…}.syx` | XV side of family **JVXP** (Roland "Int00" set and JV-2080 user bank in XV format) |
| Y3 | `…/xv5050 factory patches/bank {a..d}.syx` (and the copy under `XV-5050.rar.unpacked`) | XV side of family **X5050** (XV-5050 banks A–D, which hold JV-2080/1080 preset Patches) |
| C | JV-format dumps anywhere in `~/Downloads/integral-corpus/` and `~/Downloads/JV-1080_SP/` (JV-2080 preset dumps, the jv-patches-onlinehome-de banks, `INT_00.MID`, …) | JV side of J2XV, JVXP, X5050 |
| D | [`jv-patch-format.md`](jv-patch-format.md), [`xv-patch-format.md`](xv-patch-format.md) / [`.csv`](xv-patch-format.csv), [`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md) | Field layouts and the mapping being checked |

Only **R** is Roland-issued conversion output with known provenance. Y1–Y3 are Roland factory
data, but who converted them (Roland, an XV unit's own JV import, or a user) is unknown. They are
used as corroboration. Where they disagree with R02, this note says so.

## Method

1. Read every `.syx`/`.mid` in both trees. SMF files were walked chunk by chunk, with running
   status, `F0` events and `F7` continuations. Raw files were split into `F0…F7` frames. DT1s with
   a bad checksum were dropped. JV Patches (Model `6A`, Common ≥ `0x48` + 4 Tones of 129 bytes)
   were grouped by address. XV Patches (Model `00 10`, Common `0x4F` + TMT + 4 Tones) were grouped
   the same way. This gave 73,438 JV and 12,646 XV Patch instances.
2. Pair them by the 12-character name. For each XV Patch, keep the JV candidate that agrees best
   on 25 per-tone fields that `jv-patch-format.md` calls identical or offset (env times and levels,
   cutoff, resonance, level, pan, coarse +16, fine +14). Keep the pair if ≥ 90 % agree. 4,657 of
   4,662 kept pairs agree on **100 %**. Then deduplicate on (JV bytes, XV bytes).
3. Group them into families by XV source file, and compare field by field, over JV-active tones
   only.

| Family | Distinct pairs | XV files | JV Common length |
|---|---|---|---|
| **R02** (Roland 2002 SR-JV80 → XV) | **2,506** | SRJV02 A/B, 04 A/B, 05 A/B, 08 A/B, 10 A/B (76), 11 A/B, 12 A/B, 13 A (+3 B), 14 B, 16 A/B, 19 A/B | `0x49` (1,869), `0x4A` (637) |
| J2XV (JV-2080 presets via XV) | 538 | PrA 121, PrB 123, PrC 119, PrD 128, PrE 47 | mostly `0x4A` |
| JVXP | 192 | Roland-Int00 128, JV2080-User 64 | mostly `0x4A` |
| X5050 | 381 | banks a/b/c/d | mostly `0x4A` |
| *misc* (user files, unknown tools) | 1,045 | various | — |

*misc* is left out of every conclusion below. Its files disagree among themselves, for example
on the controller source mapping.

No JV dumps were found for SR-JV80-18, the A half of SR-JV80-14, or SR-JV80-17 Country (only 2
name matches), so those boards have no pairs.

## (a) Velocity sensitivities

### P-Env / Resonance / F-Env / Level Velocity Sens

JV offsets `42`, `54`, `57`, `6A` → XV/Integra Partial `3B`, `4E`, `51`, `62`. JV raw `v` is
`2v − 100` %.

All four fields use **one table**, and all four families agree. There are 39,716 observations,
15,574 of them non-neutral, and **one** contradiction. The JV values seen are 109 of 126.

- **`v` ≤ 105 (−100…+110 %):** `i = 64 + trunc((v − 50) × 16 / 25)`, truncating toward zero. All
  89 observed values in this range fit. So −100 % → 32, 0 % → 64, +100 % → 96.
- **`v` ≥ 106 (+112…+150 %):** a convex table that ends at 127:

| JV raw | JV % | XV raw | XV ± | Observations |
|---|---|---|---|---|
| 106 | +112 % | 100 | +36 | 103 |
| 107 | +114 % | 101 | +37 | 44 |
| 108 | +116 % | 101 | +37 | 59 |
| 109 | +118 % | 102 | +38 | 60 |
| 110 | +120 % | 103 | +39 | 318 |
| 111 | +122 % | 104 | +40 | 33 |
| 112 | +124 % | 105 | +41 | 44 |
| 113 | +126 % | 106 | +42 | 68 |
| 114 | +128 % | 108 | +44 | 35 |
| 115 | +130 % | 109 | +45 | 81 |
| 116 | +132 % | 110 | +46 | 76 |
| 117 | +134 % | 111 | +47 | 136 |
| 118 | +136 % | 113 | +49 | 33 |
| 119 | +138 % | 115 | +51 | 30 |
| 120 | +140 % | 116 | +52 | 32 |
| 121 | +142 % | 118 | +54 | 26 |
| 122 | +144 % | 120 | +56 | 23 |
| 123 | +146 % | 122 | +58 | 34 |
| 124 | +148 % | 124 | +60 | 8 |
| 125 | +150 % | 127 | +63 | 381 |

Not observed in the four families: `v` = 2, 4, 8, 10, 12, 17, 21–27, 32, 33, 36, 38. All are below
50, where the formula is linear and covers every observed neighbour.

So Roland does **not** treat ±63 as ±100 %. It treats **±32 as ±100 %** and spends the top half of
the positive range on the JV's +100…+150 % extension. The negative half below −32 is never used.
The candidate in `jv-patch-format.md` (`64 + clamp(round(pct × 63/100))`, clipping above +100 %)
would make every non-zero sensitivity about twice as strong as Roland's conversion.

### P-Env / F-Env / Level Velocity Time 1 / 4 (`T14`)

JV `43`, `44`, `58`, `59`, `6B`, `6C` → XV `3C`, `3D`, `52`, `53`, `63`, `64`.
**`i = 64 + trunc(T14[v] × 63 / 100)`**, so the full ±63 range is used.

| JV `T14` | −100 | −70 | −50 | −40 | −30 | −20 | −10 | 0 | +10 | +20 | +30 | +40 | +50 | +70 | +100 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| XV raw | 1 | 20 | 33 | 39 | 46 | 52 | 58 | 64 | 70 | 76 | 82 | 89 | 95 | 108 | 127 |

There are 59,574 observations (4,942 non-neutral) and 3 contradictions, all singletons. −70 % is
seen only in *misc* (6 tones, consistent).

## (b) Controller routing

JV per-tone Controller 1–3 (`15`–`2C`: Destination/Depth pairs, `sens = depth + 1`) → XV Common
Matrix Control 1–4 (`2B`–`4E`) + Partial Control *m* Switch *k* (`17`–`26`: OFF/ON/REVERSE).

**Source mapping** (JV Controller 1 is fixed to MOD):

| JV source | XV/Integra source index |
|---|---|
| MOD | 1 (CC01). **X5050 uses 98 (CTRL1)** instead. |
| SYS-CTRL1 | **99 (CTRL2)** |
| SYS-CTRL2 | **100 (CTRL3)** |
| BREATH / FOOT / EXPRESSION | 2 / 4 / 11 (CC numbers) |
| BEND / AFT | **96 / 97** |
| VELOCITY / KEYFOLLOW | 102 / 103 |
| LFO1 / LFO2 | 105 / 106 |
| OFF (with routes on tones) | routes dropped (7 pairs) |
| PLAYMATE | not seen |

The EFX Control Source 1/2 → MFX Control Source mapping is the same (SYS-CTRL1 → 99,
SYS-CTRL2 → 100, MOD → 1, BEND → 96, AFT → 97), and EFX depth `+1` holds.

This settles open doubt 6 in `jv-patch-format.md`: the source lists are **indexed by CC number**.
Index 32 is a hole (CC32 is not offered), and that is why the documented ranges (0–97, 0–101,
0–109) are one longer than the name lists. So BEND = 96, AFT = 97, CTRL1–4 = 98–101,
VELOCITY = 102, KEYFOLLOW = 103, TEMPO = 104 (inferred), LFO1 = 105, LFO2 = 106, and PIT/TVF/TVA-ENV
= 107–109 (inferred). Why Roland sent SYS-CTRL1/2 to CTRL2/3 rather than CTRL1/2 is not stated.
It holds in all four families (≈ 3,400 and 3,750 controller matches respectively, with minor
exceptions).

**Packing rule.** This reproduces the switched-on Matrix Control contents and every switch
exactly in R02 2,503/2,506, JVXP 192/192, J2XV 533/538 and X5050 375/381 (with MOD → 98):

```
for each active tone t (1→4):
  for each JV controller c (1→3), source s = src(c); skip if s = OFF:
    for each slot k (1→4), (dest, sens) with dest ≠ OFF and sens ≠ 64:
      1. first Matrix Control with source s having slot (dest, sens)        → switch ON
         or (dest, 128 − sens) → switch REVERSE, unless tone t already uses that slot
      2. else first Matrix Control with source s and a free slot          → append, ON
      3. else allocate the next unused Matrix Control with source s        → slot 1, ON
      4. else (all 4 Matrix Controls in use)                                → route dropped
```

What follows from the rule:

- **Matrix Controls are allocated in order of first use, not by controller number.** A JV
  controller with no routes takes no Matrix Control. MC4 is not reserved as an overflow spare: if
  MOD needs a second block, it gets the next free Matrix Control after the ones already allocated.
- **Controllers with the same source** (e.g. Controller 2 = MOD) share Matrix Controls.
- **REVERSE** is used when a later tone has the negated sens of an existing slot (199 pairs).
- **Overflow drops whatever arrives last.** This is a by-product of tone order, not a priority
  rule. In 73 pairs (R02 49, J2XV 10, X5050 14), 227 routes were dropped: tone 4 = 124, tone 3 =
  72, tone 2 = 31, tone 1 = 0. Applied to the owner's seed corpus (2,624 distinct `JV-1080_SP`
  Patches), the rule drops routes in **133** Patches, close to the 135 overflows that
  `jv-patch-format.md` measured with its own per-controller packing.
- Unused slots and Matrix Controls hold **stale bytes** (leftovers from Roland's edit buffer) with
  every switch OFF. A converter should write OFF/64 instead. The sound is the same.

The 14 mismatches look like source-version differences or hand edits (different depths on the two
sides), not a different rule.

## (c) Unconfirmed details

| Question | Roland's conversion | Support |
|---|---|---|
| LFO **SAW** polarity | **SAW-UP** (2). Full waveform table confirmed: TRI→1, SIN→0, SAW→2, SQR→4, TRP→8, S&H→9, RND→5, CHS→10 | 235 LFOs set to SAW, 0 contradictions |
| Pitch env start level (XV `43`) | **64** (no offset) in every pair | 9,929 active tones (non-*misc*), 0 exceptions |
| TVF env start level (XV `59`) | **0** in every pair | same |
| KEY-INTERVAL / PLAYMATE (Delay Mode 2) | **NORMAL with Delay Time 0**: the delay is removed, not approximated | 25 tones, 0 exceptions |
| HOLD, KEY-OFF-NORMAL, KEY-OFF-DECAY | 1, 2, 3, with Delay Time copied | 67 + 67 + 26 tones |
| CLOCK-SYNC (3), TAP-SYNC (4) delay | **NORMAL with Delay Time = note value** (128–149, table below) | 143 + 2 tones, 0 contradictions |
| LFO External Sync CLOCK/TAP | **LFO Rate = note value** by the same table | 1,115 LFOs |
| Volume Control Switch → Receive Expression | **Not mapped.** Receive Expression is always ON (1), including the 17 tones with Volume Control OFF | 0 exceptions |
| Velocity Cross Fade | Velocity Range Switch ON: ranges copied, **Fade Width Lower = Upper = Cross Fade** (343 tones with fade ≠ 0, 0 exceptions) | 4,815 tones |
| Velocity Range Switch OFF | TMT Velocity Control is **always ON (1)**. Each tone gets range **1–127**, fades 0, so the JV's ignored ranges are discarded | 5,114 tones (260 had non-default JV ranges), 0 exceptions |

The sync note table maps a JV rate/time `v` to an XV note value. It is the same for delay and
both LFOs, with no contradictions:

| XV | 128 | 129 | 130 | 131 | 132 | 133 | 134 | 135 | 136 | 137 | 138 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| note | 1/64T | 1/64 | 1/32T | 1/32 | 1/16T | 1/32. | 1/16 | 1/8T | 1/16. | 1/8 | 1/4T |
| JV `v` seen | 0–2 | 3 | 4 | 6–7 | 8 | 9–10 | 11–14 | 16–17 | 18–21 | 22–27 | 29–33 |

| XV | 139 | 140 | 141 | 142 | 143 | 144 | 145 | 146 | 147 | 148 | 149 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| note | 1/8. | 1/4 | 1/2T | 1/4. | 1/2 | 1/1T | 1/2. | 1/1 | 2/1T | 1/1. | 2/1 |
| JV `v` seen | 36–42 | 45–48 | 53–57 | 59–66 | 67–76 | 80 | 83–90 | 91–100 | 102–104 | 107–113 | 115–127 |

The anchors (12 = 1/16, 24 = 1/8, 48 = 1/4, 72 = 1/2, 96 = 1/1, 120 = 2/1) are exact. The
boundaries in the gaps (5, 15, 28, 34–35, 43–44, …) are unobserved. The XV keeps the Patch Tempo
(JV `2C`–`2D` copied to XV Common `1F`, 0 mismatches), so the note values play at the JV tempo. On
the Integra they follow the Studio Set tempo instead.

## (d) Where Roland differs from `jv-patch-format.md`

| `jv-patch-format.md` row | Roland's conversion | Support |
|---|---|---|
| Velocity sens: "rescale, no published formula" (candidate ±100 % = ±63, clip) | ±100 % = **±32**, +100…+150 % stretched to +63 by table (§a) | 15,574 non-neutral observations |
| Velocity time sens: "rescale" | `64 + trunc(pct × 63/100)` (§a) | 4,942 |
| Tone `11` Volume Control Switch → Receive Expression | **dropped**; Receive Expression = ON always | 17 OFF tones, all → ON |
| Tone `14` Pan Control Switch → **none** | → Partial `15` **Receive Pan Mode**: CONTINUOUS → 0, KEY-ON → 1, OFF → 0 (CONTINUOUS) | 9,929 tones; KEY-ON only 2 tones, OFF 23 |
| Tone `09` CLOCK-SYNC / TAP-SYNC → none (NORMAL) | NORMAL **plus note-valued Delay Time** (§c) | 145 tones |
| LFO External Sync → none (out of scope) | LFO Rate → note value (§c) | 1,115 LFOs |
| Common `3A`/`3B` SYS-CTRLn → CTRLn | SYS-CTRL1 → **CTRL2 (99)**, SYS-CTRL2 → **CTRL3 (100)**; same for EFX → MFX control source | all four families |
| Common `40` Velocity Range Switch → PMT `04` identical | PMT `04` **always ON**. Switch OFF → tone ranges forced 1–127, fades 0 | 1,925 pairs with the switch OFF |
| Controller transform (distinct pairs per controller, MC4 as spare) | Tone-major greedy allocation by source (§b) | 3,603 / 3,617 exact |
| Tone `7E`–`80` sends → `0C`, `0F`, `10` | XV: JV **MIX** → Tone Output Assign **A (1)**, sends in the non-MFX pair `0F`/`10`. JV **EFX** → **MFX (0)**, sends in the MFX pair `0D`/`0E`. The unused pair is set to **127**. On the Integra only `0F`/`10` exist, so `jv-patch-format.md` is right for the Integra, and [`xv-patch-format.md`](xv-patch-format.md)'s send-pair rule applies | 9,929 tones; 0 exceptions in JVXP, 2 in J2XV |
| Common `49` Patch Category → "name table, lists differ" | JV-2080 category index → XV Patch Category **same index** (2 exceptions). JV-1080/XP Patches (no category) got hand-assigned categories | 722 pairs with a JV category |
| Integra-only defaults | Matches the doc: Env Mode SUSTAIN, Cutoff Velocity Curve FIXED / Sens 64, LFO Rate Detune 0, LFO Delay KF 64, Wave R = 0, Keyboard Fade Widths 0, Common offsets 64 | 0 exceptions |
| Legato Retrigger (no JV source) | ON in R02/JVXP/J2XV. X5050 sets it ON only for SOLO + Legato | — |

Rows the pairs **confirm** (0 exceptions in JVXP; in R02 only the hand edits listed below; a few
source-version differences in J2XV/X5050): every "identical" row, every `+1`/`+14`/`+16`/`+52`/`+61` offset,
every `T15`/`T14` keyfollow formula, Key Assign `1 − v`, FXM Depth `+1`, Delay Time for 0–127,
and LFO Rate when not synced.

Small anomalies, all in R02, look like hand edits: Alternate Pan 1 → 64 (5 tones), Wave Gain
+12 → +6 (4), FXM Color 1 → 2 (5), Booster 3&4 when the structure has no booster (37), Octave Shift off by one (7 Patches). **Reverb
sends on SR-JV80-08, -11 and -19 were revoiced:** about 40–60 % of tones there have JV send 0 but
XV send > 0. The other boards copy the reverb send exactly. A converter should copy the send
rather than imitate this.

## Confidence

| Finding | Confidence | Basis |
|---|---|---|
| Velocity sens table / formula | **High** | 4 families, 15,574 non-neutral observations, 1 contradiction. 17 low JV values unobserved but on the linear part |
| Time sens formula | **High** | All 15 `T14` steps seen, 3 singleton contradictions |
| Controller packing rule | **High** | 3,603/3,617 exact (99.6 %); every R02 drop reproduced |
| SYS-CTRL1/2 → CTRL2/3 | **High** as Roland's choice, **unknown** rationale | All families; whether it is right on an Integra depends on its System Control assignments |
| Source index = CC number (BEND 96 …) | **High** for observed entries; TEMPO/ENV inferred | Direct observation |
| SAW = SAW-UP; L0 = 64 / 0 | **High** | Thousands of tones, 0 exceptions |
| KEY-INTERVAL → NORMAL, time 0 | **Medium-high** | 25 tones, 0 exceptions |
| Sync note table | **High** at the anchors, **medium** at the gap boundaries | 1,260 observations, 0 contradictions |
| Volume Control Switch dropped | **Medium-high** | 17 OFF tones, all ON |
| Pan Control KEY-ON → Receive Pan Mode KEY-ON | **Medium** | 2 tones |
| Cross Fade → both widths; Range Switch OFF → 1–127 | **High** | 343 / 260 tones, 0 exceptions |

## Not covered

Wave mapping (SR-JV80 → SRX, INT-A/B → XV INT; [#195](https://github.com/sigma/integral/issues/195))
and EFX → MFX type and parameters ([#196](https://github.com/sigma/integral/issues/196)) are both
present in these pairs. They were not analysed here. The R02 set is a Roland-made SR-JV80 → SRX
Wave mapping for 11 boards.

The scratch scripts (Python stdlib, in `/tmp`) were not kept.
