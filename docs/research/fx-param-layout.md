# Studio Set Chorus / Reverb Parameter Layout (research for #27)

Answers [#27](https://github.com/sigma/integral/issues/27): the per-type slot layout of the
Studio Set **Chorus** (FX1, `18 00 04 04 + 4n`, 20 slots) and **Reverb** (FX2,
`18 00 06 03 + 4n`, 24 slots) nibblized parameters.

## TL;DR

- The authoritative per-slot map is in **Roland's own INTEGRA-7 Editor JavaScript**
  (`Copyright 2014 Roland Corporation`). It is mirrored in
  [SambaGodschynski/integra7-editor-web](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model).
  It matches both raw device dumps exactly.
- Every value is `stored = raw16 − 32768`. The stored value is an **index**,
  `index = stored − leaf.min`, and a per-parameter table turns the index into the display
  value. Only some parameters are linear.
- **Chorus**: Rate is a 3-slot group (Hz/NOTE switch, Hz, note). **Delay**: each of
  L/R/C is a 3-slot group (msec/NOTE switch, msec, note). Chorus and Delay have **no**
  "Send Level to Reverb" slot. Only GM2 Chorus has one.
- **Reverb**: slot 1 of Room/Hall/Plate is a **hidden level** parameter, and Pre Delay
  starts at slot 2. GM2 Reverb has hidden slots 2–3, and Time is at slot 4.
- Base-address off-by-one: **none**. `18 00 06 03` and `18 00 04 04` are right in the MIDI
  Implementation, in Roland's editor, in `device.rs`, and in `web/src/integra.ts`. The
  off-by-one is in the **tables** (`fxParams.ts` / `fx_params.rs` /
  `docs/midi/11-fx-parameters.md`). They put Pre Delay at reverb slot 1.

## Sources (highest trust first)

| # | Source | What it gives |
|---|--------|---------------|
| P1 | Roland *INTEGRA-7 Parameter Guide* e01, p. 98 ([PDF](https://static.roland.com/assets/media/pdf/INTEGRA-7_Parameter_Guide_e01_W.pdf)) | Names, order and display ranges per type. **No slot numbers or raw encodings.** |
| P2 | Roland *INTEGRA-7 MIDI Implementation* v1.00 ([PDF](https://static.roland.com/assets/media/pdf/INTEGRA-7_MI.pdf)) | Block layout: Chorus Parameter 1–20 at `00 04`–`00 50`, Reverb Parameter 1–24 at `00 03`–`00 5F`, each 12768–52768. **No per-type map.** The FA-06/07/08 MIDI Implementation ([PDF](https://static.roland.com/assets/media/pdf/FA-06_07_08_MIDI_Imple_eng01_W.pdf)) is the same: no per-type map. |
| R1 | Roland INTEGRA-7 Editor JS, `model_excmap_chorus.js` ([L9-L65](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap_chorus.js#L9-L65)) | Per-type, per-slot leaf list: id, init, min, max. |
| R2 | same, `model_excmap_reverb.js` ([L9-L103](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap_reverb.js#L9-L103)) | Per-type, per-slot leaf list, including the hidden (`desc: ''`) slots. |
| R3 | same, `model_stringer.js` ([CHO/REV strings L2288-L2366](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L2288-L2366), [tables L4355-L4383](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L4355-L4383)) | Index → display tables (Hz, deg, %, Pre Delay ms, note list, frequency list). |
| R4 | same, `model.js` ([L121](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model.js#L121)), `model_excmap.js` ([FH/FV blocks L226-L281](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap.js#L226-L281), [bank lookup L1935-L1948](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap.js#L1935-L1948)) | Display index = `value − leaf.min`. The block entry with `pos: n` is Parameter n+1 and takes leaf `n` of the current type. |
| O1 | the51craw INTEGRA-7 Librarian & Editor (C#), `StudioSetClasses.cs` ([L329-L1010](https://github.com/the51craw/INTEGRA-7_Xamarin_Android/blob/efaba3e60b6891ba21ce8d4869e44f6978d8bf1f/INTEGRA_7_Xamarin/INTEGRA_7_Xamarin/StudioSetClasses.cs#L329-L1010)), `StudioSetEditor.cs` ([Center FB L5470](https://github.com/the51craw/INTEGRA-7_Xamarin_Android/blob/efaba3e60b6891ba21ce8d4869e44f6978d8bf1f/INTEGRA_7_Xamarin/INTEGRA_7_Xamarin/StudioSetEditor.cs#L5470), [Pre Delay/Hz/note L10061-L10110](https://github.com/the51craw/INTEGRA-7_Xamarin_Android/blob/efaba3e60b6891ba21ce8d4869e44f6978d8bf1f/INTEGRA_7_Xamarin/INTEGRA_7_Xamarin/StudioSetEditor.cs#L10061-L10110)) | Independent, reverse-engineered: same slots. Its per-type default dumps were "read out from I-7 Integra Preview studio set" and match R1/R2 and our device. |
| C1 | Roland SH-4d reference, Reverb Parameters ([page](https://static.roland.com/manuals/sh-4d/eng/66978025.html)) | Same engine, published as "INTEGRA7Rev" (Char + Pre Delay … Tone) and "GM2 Reverb" (6 characters). Cross-check only. |
| X | This repo's `docs/midi/11-fx-parameters.md`, `fx_params.rs`, `web/src/fxParams.ts` | **Wrong** for these tables (see "What's wrong today"). |

P1, R1–R4 and the device dumps are primary. O1 is independent corroboration. C1 is a sibling product.

## Encoding

Each slot is 4 bytes `0000 aaaa 0000 bbbb 0000 cccc 0000 dddd` (P2), so
`raw16 = a<<12 | b<<8 | c<<4 | d` and `stored = raw16 − 32768`.
All chorus/reverb leaves have `min ≥ 32768`, so `stored ≥ 0` for every real parameter.
Display = `table[stored − (leaf.min − 32768)]` (R4). This only matters for leaves whose
`min` is not 32768: Chorus **Rate Hz** (min 1) and Reverb **Time** (min 1).

### Value tables (R3)

| Name | Index range | Mapping |
|------|-------------|---------|
| `SYNC_MS` | 0–1 | 0 = msec, 1 = NOTE |
| `SYNC_HZ` | 0–1 | 0 = Hz, 1 = NOTE |
| `NOTE` | 0–21 | 1/64T, 1/64, 1/32T, 1/32, 1/16T, 1/32., 1/16, 1/8T, 1/16., 1/8, 1/4T, 1/8., 1/4, 1/2T, 1/4., 1/2, 1/1T, 1/2., 1/1, 2/1T, 1/1., 2/1 |
| `FREQ` | 0–16 | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz |
| `FREQ_BYPASS` | 0–17 | `FREQ` + 17 = BYPASS |
| `PREDELAY_CHO` | 0–125 | 0–49 → 0.0–4.9 ms (step 0.1). 50–59 → 5.0–9.5 (step 0.5). 60–99 → 10–49 (step 1). 100–125 → 50–100 (step 2). |
| `RATE_HZ` | stored 1–200 | Hz = stored × 0.05 (0.05–10.00) |
| `PHASE` | 0–90 | deg = 2 × stored (0–180) |
| `FB_PCT` | 0–98 | % = 2 × stored − 98 (−98…+98, step 2; 49 = 0 %) |

O1 agrees on `PREDELAY_CHO` (same piecewise formula), `RATE_HZ` (×20 when writing),
`PHASE` (0–90 sent, shown ×2), `FB_PCT` (`(p/2)+49`), and the `NOTE` list.

## Chorus (FX1): `18 00 04 00`

Core: `00` Type (0–3), `01` Level, `02` **Output Assign (A–D)**, `03` Output Select
(MAIN/REV/MAIN+REV). The MIDI Implementation and our `05-studio-set.md` mark `02` as
reserved. Roland's editor (`NEFH_CHO_OUT_ASGN`, R4 L228) and P1 ("Chorus Output Assign A,
B, C, D") say it is a real parameter. That is outside #27's scope, but worth a follow-up.

Slot *n* is at offset `0x04 + 4(n−1)`.

### Type 0: OFF

Slot 1 is a dummy (`CHO_OFF_DUMMY`, 0). No parameters.

### Type 1: Chorus (R1 L18-L31). Confidence: high (unverified on device)

| Slot | Off | Param (Roland id) | Stored range | Display | Roland init |
|------|-----|-------------------|--------------|---------|-------------|
| 1 | `04` | Filter Type (`CHO_CHO_FILT`) | 0–2 | OFF, LPF, HPF | 2 |
| 2 | `08` | Cutoff Freq (`CHO_CHO_SPLT`) | 0–16 | `FREQ` | 6 (800 Hz) |
| 3 | `0C` | Pre Delay (`CHO_CHO_DELAY`) | 0–125 | `PREDELAY_CHO` | 20 (2.0 ms) |
| 4 | `10` | Rate sync sw (`CHO_CHO_RATE_SYNC`) | 0–1 | `SYNC_HZ` | 0 |
| 5 | `14` | Rate Hz (`CHO_CHO_RATE_HZ`) | 1–200 | `RATE_HZ` | 10 (0.50 Hz) |
| 6 | `18` | Rate note (`CHO_CHO_RATE_NOTE`) | 0–21 | `NOTE` | 18 (1/1) |
| 7 | `1C` | Depth (`CHO_CHO_DEPTH`) | 0–127 | linear | 30 |
| 8 | `20` | Phase (`CHO_CHO_PHS`) | 0–90 | `PHASE` | 90 (180°) |
| 9 | `24` | Feedback (`CHO_CHO_FB`) | 0–127 | linear | 0 |
| 10–20 | | unused | | | |

O1's built-in Chorus default (from the "Integra Preview" studio set) is exactly
2, 6, 20, 0, 10, 18, 30, 90, 0. Not read from our device, because that would mean switching the type.

### Type 2: Delay (R1 L33-L51). Confidence: **verified against device**

| Slot | Off | Param (Roland id) | Stored range | Display | Device |
|------|-----|-------------------|--------------|---------|--------|
| 1 | `04` | Delay Left sync sw (`CHO_DLY_DLY_L_SYNC`) | 0–1 | `SYNC_MS` | 1 → NOTE |
| 2 | `08` | Delay Left msec (`CHO_DLY_DLY_L_MSEC`) | 0–1000 | ms, linear | 200 |
| 3 | `0C` | Delay Left note (`CHO_DLY_DLY_L_NOTE`) | 0–21 | `NOTE` | 6 → 1/16 |
| 4 | `10` | Delay Right sync sw | 0–1 | `SYNC_MS` | 1 → NOTE |
| 5 | `14` | Delay Right msec | 0–1000 | ms | 400 |
| 6 | `18` | Delay Right note | 0–21 | `NOTE` | 9 → 1/8 |
| 7 | `1C` | Delay Center sync sw | 0–1 | `SYNC_MS` | 1 → NOTE |
| 8 | `20` | Delay Center msec | 0–1000 | ms | **600** |
| 9 | `24` | Delay Center note | 0–21 | `NOTE` | 12 → 1/4 |
| 10 | `28` | Center Feedback (`CHO_DLY_FBK`) | 0–98 | `FB_PCT` | 59 → **+20 %** |
| 11 | `2C` | HF Damp (`CHO_DLY_HF_DAMP`) | 0–17 | `FREQ_BYPASS` | 17 → BYPASS |
| 12 | `30` | Left Level | 0–127 | linear | 127 |
| 13 | `34` | Right Level | 0–127 | linear | 127 |
| 14 | `38` | Center Level | 0–127 | linear | 127 |
| 15–20 | | unused (no Send Level to Reverb) | | | 0 |

Correction to the ground truth in #27: slot 8 reads `08 02 05 08` = 0x8258 − 0x8000 =
**600**, not 120. That matches the "Lv C showed 600" symptom and Roland's init (33368 − 32768 = 600).
Re-read on 2026-10-10 with `integral raw-read 18000400 54`:
`02 7F 00 00 | 08 00 00 01 | 08 00 0C 08 | 08 00 00 06 | 08 00 00 01 | 08 01 09 00 | 08 00 00 09 | 08 00 00 01 | 08 02 05 08 | 08 00 00 0C | 08 00 03 0B | 08 00 01 01 | 08 00 07 0F ×3 | 08 00 00 00 …`.
Slot 15 is 0, but Roland's map has only 14 Delay leaves. The issue's "15 = Send Level to
Reverb" is wrong.

### Type 3: GM2 Chorus (R1 L53-L64). Confidence: high (unverified on device)

| Slot | Off | Param | Stored range | Display | Roland init |
|------|-----|-------|--------------|---------|-------------|
| 1 | `04` | Pre-LPF | 0–7 | linear | 0 |
| 2 | `08` | Level | 0–127 | linear | 64 |
| 3 | `0C` | Feedback | 0–127 | linear | 8 |
| 4 | `10` | Delay | 0–127 | linear | 80 |
| 5 | `14` | Rate | 0–127 | linear | 3 |
| 6 | `18` | Depth | 0–127 | linear | 19 |
| 7 | `1C` | Send Level to Reverb | 0–127 | linear | 0 |

The current table already has this order. O1's default dump matches the inits.

## Reverb (FX2): `18 00 06 00`

Core: `00` Type (0–6), `01` Level, `02` Output Assign (A–D). Slot *n* is at
`0x03 + 4(n−1)`. Block size 0x63 = 3 + 24 × 4 (P2, R4 L253-L281).
`requestReverbParams` reads `reverb_address(0x03)` for 0x60 bytes, and
`set_reverb_nib_param` writes `0x03 + 4·index` (`crates/integral-core/src/device.rs:430`).
Both are **correct**.

### Type 0: OFF

Dummy slot 1. No parameters.

### Types 1–5: Room 1, Room 2, Hall 1, Hall 2, Plate (R2 L18-L91). Confidence: **verified against device (Hall 1)**

All five types share the layout. They differ only in their inits.

| Slot | Off | Param (Roland id, RM1 shown) | Stored range | Display | Device (Hall 1) |
|------|-----|------------------------------|--------------|---------|-----------------|
| 1 | `03` | *(hidden)* Level (`REV_REVRM1_LEVEL`, `desc: ''`) | 0–127 | not shown | 95 |
| 2 | `07` | Pre Delay | 0–100 | ms, linear | 0 |
| 3 | `0B` | Time | 1–100 | sec = stored / 10 (0.1–10.0) | 21 → 2.1 s |
| 4 | `0F` | Density | 0–127 | linear | 127 |
| 5 | `13` | Diffusion | 0–127 | linear | 127 |
| 6 | `17` | LF Damp | 0–100 | linear | 50 |
| 7 | `1B` | HF Damp | 0–100 | linear (not a frequency table) | 50 |
| 8 | `1F` | Spread | 0–127 | linear | 127 |
| 9 | `23` | Tone | 0–127 | linear | 64 |
| 10–24 | | unused | | | 0 |

Roland inits: Level 95. Pre Delay 0 (Plate: 20). Time 9 (Room 1/2) or 26 (Hall 1/2, Plate).
Density 127, Diffusion 127, LF/HF Damp 50, Spread 127, Tone 64. O1's per-type defaults have
Time 9/9/21/26/26 and Plate Pre Delay 20. Our device's Hall 1 dump
(95, 0, 21, 127, 127, 50, 50, 127, 64) is identical to O1's Hall 1 default.

### Type 6: GM2 Reverb (R2 L93-L101). Confidence: medium-high (unverified on device)

| Slot | Off | Param | Stored range | Display | Roland init |
|------|-----|-------|--------------|---------|-------------|
| 1 | `03` | Character | 0–5 | numeric 0–5 (P1) | 4 |
| 2 | `07` | *(hidden)* Level (`REV_GM2REV_LEVEL`) | 0–127 | not shown | 64 |
| 3 | `0B` | *(hidden)* Pre Delay (`REV_GM2REV_PREDLY`) | 0–100 | not shown | 0 |
| 4 | `0F` | Time | 0–127 | linear | 64 |

O1 independently writes Character to `18 00 06 03` and GM2 Time to `18 00 06 0F`
(StudioSetEditor.cs L6001 and the GM2 Time handler). Its default dump is 4, 64, 0, 64.
P1 lists Character as "0–5" with no names. C1 (SH-4d) names the six as Small Room, Medium
Room, Large Room, Medium Hall, Large Hall, Plate. That is plausible (GM2 reverb types) but
**not confirmed for the INTEGRA-7**.

## What's wrong today (repo)

`docs/midi/11-fx-parameters.md`, `crates/integral-core/src/fx_params.rs` and
`web/src/fxParams.ts` all share these errors:

- **Chorus/Chorus**: Rate is one slot instead of three (sw/Hz/note), so Depth, Phase and
  Feedback are shifted. Cutoff is treated as Hz 200–8000 instead of an index 0–16. Pre
  Delay is treated as `v/10` instead of `PREDELAY_CHO`. Phase is treated as 0–180 instead
  of 0–90 ×2. There is a bogus slot-8 "Send Level to Reverb".
- **Chorus/Delay**: one slot per delay instead of three. Center Feedback is treated as
  signed % instead of index 0–98. HF Damp is treated as Hz instead of index 0–17. There is
  a bogus slot-9 "Send Level to Reverb".
- **Reverb Room/Hall/Plate**: every parameter is one slot early, because slot 1 is the
  hidden level. Pre Delay must be index 1, Time index 2, and so on.
- **Reverb GM2**: Time is at slot 4 (index 3), not slot 2.
- GM2 Chorus is correct.

The same switch/value/note pattern exists in MFX (R3 has `MFX_*_RATE_SYNC/_HZ/_NOTE`, for example
`MFX_TRECHO_*`). `mfx_params.rs` may have the same flattening bug. That was not
investigated here.

## Unresolved / unverified

1. **Hidden reverb Level (slot 1; GM2 slot 2)**: its function is unknown. On our device it
   equals the core Reverb Level (both 95, `5F`), and Roland's init (95) matches too, so the
   dump can't tell whether it mirrors `00 01` or is independent. Recommendation: don't
   expose it, and preserve it on writes. Settling it needs a device experiment, which this
   research did not do (no writes allowed).
2. **Chorus, GM2 Chorus, GM2 Reverb, and Room/Plate inits** have not been read from our
   device, because reading them would require switching type. The layouts rest on R1/R2,
   corroborated by O1.
3. **GM2 Reverb Character names**: only P1's "0–5" is first-party for the INTEGRA-7.
4. **Chorus core `00 02` Output Assign**: P1 and R4 say it exists, but P2 and our docs say reserved.
5. Hidden-slot behaviour on type change: O1 notes that changing Type via SysEx does not
   reload the per-type defaults into the slot memory ("I-7 will update MIDI data from some
   internal storage only when changed from the front panel"). Roland's editor keeps a
   per-type bank and re-sends all slots on a type change (R4 `model_excmap_bank.put`,
   L1950+). Our UI will need to do the same if it ever switches types.
