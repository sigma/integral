# MFX Parameter Slot Layout (research for #30)

Answers steps 1–2 of [#30](https://github.com/sigma/integral/issues/30): compare the 68 MFX
type tables in
[`mfx_params.rs`](../../crates/integral-core/src/mfx_params.rs) with Roland's own
per-slot map. The method follows [Chorus / Reverb layout](fx-param-layout.md) (#27).
Per-slot detail is in [`mfx-param-layout.csv`](mfx-param-layout.csv).

## TL;DR

- Roland's INTEGRA-7 Editor defines all 68 MFX types as **852** parameter slots.
  `mfx_params.rs` has **724**. Only **Thru** matches.
- **47 types have the wrong layout**: at least one parameter is at the wrong MFX
  Parameter number. **20 types** have the right layout but wrong stored ranges or
  defaults.
- **Every** table stores **display** values (`-15..15` dB, `200..8000` Hz,
  `-98..98` %). The device stores **indices** from 0: `stored = raw16 − 32768`, and
  `display = table[stored − (leaf.min − 32768)]`. Today a Gain of 0 dB is written as
  stored 0, which is −15 dB on the device.
- Layout causes, most common first:
  1. **Sync triples flattened** (42 types): each msec-or-note or Hz-or-note parameter is
     3 slots on the device (sync switch, value, note) but 1 slot in our table.
  2. **Rotary 1 / Rotary 2 mixed up**: our Rotary 1 has Rotary 2's Brake, Low Gain and
     High Gain, and our Rotary 2 lacks Brake.
  3. **OD block misplaced**: on the device the OD Switch/Gain/Drive block is at the end
     of the list. Our table puts it early: after Brake in Rotary 3 (21), after Treble
     in EP AmpSim → \* (57–61).
  4. **GuitarAmpSim → Phaser (55)**: Rate and Manual are swapped.
  5. **Humanizer (7)**: we lack Low Gain and High Gain, and Manual comes before Input
     Sync on the device.
  6. **Compressor (31)**: slot 7 is hidden (empty name). It follows the last visible slot,
     so it shifts nothing.
- Value tables: 285 slots are plain `0..n`. The other 565 use **58 distinct** index → display
  tables, and 2 have no Roland stringer entry. Most are offset-linear (dB, %, pan, Hz × 0.05, balance), but the
  frequency, Pre Delay and note lists are true lookups.

## Sources

| # | Source | What it gives |
|---|--------|---------------|
| R1 | Roland INTEGRA-7 Editor JS, `model_excmap_mfx.js` ([file](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap_mfx.js)) | Per-type leaf list: desc, id, init, min, max. Leaves 0–3 are Control Assign 1–4, and leaf 4+k is MFX Parameter k+1. |
| R2 | same, `model_excmap.js` ([MFX block L115-L162](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap.js#L115-L162), [bank lookup L1935-L1948](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap.js#L1935-L1948)) | Control Assign 1–4 at `0x0D`–`0x10` are `pos 0–3`. MFX Parameter 1 at `0x11` is `pos 4`, through Parameter 32 at `0x10D` (`pos 35`). |
| R3 | same, `model_stringer.js` ([file](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js)) | Index → display string for every leaf id. |
| X | This repo's `mfx_params.rs` | **Wrong** (see TL;DR and the table below). |

All three Roland files are from
[integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model)
(`Copyright 2014 Roland Corporation`). For Chorus and Reverb, #27 found that the same
files match raw device dumps exactly. For MFX, see [Device checks](#device-checks).

## Encoding

Same as Chorus/Reverb: each MFX Parameter is 4 nibbles, `raw16 = a<<12 | b<<8 | c<<4 | d`,
`stored = raw16 − 32768`. This matches
[`encode_nib_param`](../../crates/integral-core/src/state/parse.rs).

- Every R1 leaf has `min ≥ 32768`, so a real parameter never has a negative stored value.
- Some leaves start above 32768: the Rotary speeds and the Hz rates are `1..200`, read as
  Hz = stored × 0.05. For these, index 0 of the display table is stored value 1.
- Each sync triple has the same shape: `X (sync sw)` `0..1` (msec/Hz, NOTE),
  then `X (msec)` or `X (Hz)`, then `X (note)` `0..21` (the 22-entry NOTE list in
  [fx-param-layout.md](fx-param-layout.md#value-tables-r3)). The device's default for the
  sync switch is often NOTE: Delay Left/Right both start at NOTE, 1/4.
- Control Assign 1–4 (`0x0D`–`0x10`): `max` in R1 depends on the type. It is the
  number of parameters that type can route to MFX Control. `mfx_params.rs` does not
  model this.

### Most-used value tables (R3)

| Slots | Entries | Display |
|------:|--------:|---------|
| 84 | 31 | −15 … +15 dB |
| 63 | 22 | NOTE list (1/64T … 2/1) |
| 47 | 200 | 0.05 … 10.00 Hz (stored 1–200) |
| 40 | 101 | D100:0W … D0:100W |
| 39 | 2 | OFF, ON |
| 34 | 2 | Hz, NOTE |
| 29 | 99 | −98 … +98 % (step 2) |
| 29 | 2 | msec, NOTE |
| 21 | 128 | L64 … 63R |
| 20 | 126 | 0.0 … 100 msec (Pre Delay steps) |
| 16 | 2601 | 0 … 2600 msec |
| 15 | 18 | 200 … 8000 Hz, BYPASS |
| 14 | 1301 | 0 … 1300 msec |
| 10 | 101 | −50 … +50 |
| 7 | 91 | 0 … 180 deg (step 2) |
| 6 | 17 | 200 … 8000 Hz |

The remaining 42 tables are type-specific name lists: amp and speaker types, LoFi types,
filter types and so on.

## Per-type comparison

*Slots* = Roland's parameter count and ours. *First wrong* = first MFX Parameter number
whose parameter differs from Roland's. *Range* / *Default* = slots that hold the same
parameter but a different stored range or default. *Status*: `layout` = parameter numbers
are wrong; `values` = numbers are right, values are wrong.

| Type | Name | Roland | Ours | First wrong | Range | Default | Status |
|-----:|------|-------:|-----:|------------:|------:|--------:|--------|
| 0 | Thru | 0 | 0 | – | 0 | 0 | ok |
| 1 | Equalizer | 11 | 11 | – | 6 | 9 | values |
| 2 | Spectrum | 10 | 10 | – | 8 | 9 | values |
| 3 | Low Boost | 6 | 6 | – | 3 | 4 | values |
| 4 | Step Filter | 25 | 23 | 18 | 1 | 17 | layout |
| 5 | Enhancer | 5 | 5 | – | 2 | 2 | values |
| 6 | Auto Wah | 13 | 11 | 7 | 1 | 5 | layout |
| 7 | Humanizer | 15 | 11 | 6 | 1 | 5 | layout |
| 8 | Speaker Simulator | 5 | 5 | – | 0 | 2 | values |
| 9 | Phaser 1 | 13 | 11 | 4 | 1 | 2 | layout |
| 10 | Phaser 2 | 5 | 5 | – | 2 | 2 | values |
| 11 | Phaser 3 | 4 | 4 | – | 2 | 2 | values |
| 12 | Step Phaser | 16 | 12 | 4 | 1 | 2 | layout |
| 13 | Multi Stage Phaser | 12 | 10 | 4 | 1 | 3 | layout |
| 14 | Infinite Phaser | 8 | 8 | – | 4 | 7 | values |
| 15 | Ring Modulator | 7 | 7 | – | 2 | 4 | values |
| 16 | Tremolo | 8 | 6 | 3 | 1 | 1 | layout |
| 17 | Auto Pan | 8 | 6 | 3 | 1 | 1 | layout |
| 18 | Slicer | 25 | 23 | 18 | 17 | 10 | layout |
| 19 | Rotary 1 | 11 | 14 | 2 | 0 | 0 | layout |
| 20 | Rotary 2 | 16 | 15 | 2 | 0 | 0 | layout |
| 21 | Rotary 3 | 20 | 20 | 3 | 0 | 1 | layout |
| 22 | Chorus | 12 | 10 | 5 | 3 | 4 | layout |
| 23 | Flanger | 13 | 11 | 5 | 3 | 4 | layout |
| 24 | Step Flanger | 16 | 12 | 5 | 3 | 4 | layout |
| 25 | Hexa-Chorus | 10 | 8 | 3 | 3 | 3 | layout |
| 26 | Tremolo Chorus | 12 | 8 | 3 | 2 | 2 | layout |
| 27 | Space-D | 10 | 8 | 3 | 2 | 2 | layout |
| 28 | Overdrive | 8 | 8 | – | 3 | 6 | values |
| 29 | Distortion | 8 | 8 | – | 3 | 7 | values |
| 30 | Guitar Amp Simulator | 17 | 17 | – | 1 | 7 | values |
| 31 | Compressor | 7 | 6 | 7 | 2 | 4 | layout |
| 32 | Limiter | 7 | 7 | – | 2 | 5 | values |
| 33 | Gate | 7 | 7 | – | 0 | 5 | values |
| 34 | Delay | 15 | 11 | 2 | 1 | 1 | layout |
| 35 | Modulation Delay | 18 | 12 | 2 | 1 | 1 | layout |
| 36 | 3Tap Pan Delay | 18 | 12 | 2 | 1 | 1 | layout |
| 37 | 4Tap Pan Delay | 22 | 14 | 2 | 1 | 1 | layout |
| 38 | Multi Tap Delay | 26 | 18 | 2 | 1 | 1 | layout |
| 39 | Reverse Delay | 27 | 19 | 3 | 1 | 2 | layout |
| 40 | Time Ctrl Delay | 10 | 8 | 2 | 1 | 1 | layout |
| 41 | LOFI Compress | 8 | 8 | – | 3 | 6 | values |
| 42 | Bit Crasher | 6 | 6 | – | 2 | 5 | values |
| 43 | Pitch Shifter | 10 | 8 | 4 | 3 | 2 | layout |
| 44 | 2Voice Pitch Shifter | 20 | 16 | 4 | 3 | 3 | layout |
| 45 | Overdrive -> Chorus | 9 | 7 | 5 | 3 | 3 | layout |
| 46 | Overdrive -> Flanger | 10 | 8 | 5 | 3 | 3 | layout |
| 47 | Overdrive -> Delay | 9 | 7 | 4 | 2 | 2 | layout |
| 48 | Distortion -> Chorus | 9 | 7 | 5 | 3 | 4 | layout |
| 49 | Distortion -> Flanger | 10 | 8 | 5 | 3 | 4 | layout |
| 50 | Distortion -> Delay | 9 | 7 | 4 | 2 | 3 | layout |
| 51 | OD/DS -> TouchWah | 16 | 16 | – | 2 | 10 | values |
| 52 | OD/DS -> AutoWah | 18 | 16 | 12 | 1 | 7 | layout |
| 53 | GuitarAmpSim -> Chorus | 16 | 16 | – | 2 | 7 | values |
| 54 | GuitarAmpSim -> Flanger | 17 | 17 | – | 3 | 8 | values |
| 55 | GuitarAmpSim -> Phaser | 17 | 17 | 12 | 0 | 7 | layout |
| 56 | GuitarAmpSim -> Delay | 16 | 16 | – | 3 | 7 | values |
| 57 | EP AmpSim -> Tremolo | 14 | 12 | 4 | 2 | 2 | layout |
| 58 | EP AmpSim -> Chorus | 15 | 13 | 4 | 2 | 2 | layout |
| 59 | EP AmpSim -> Flanger | 16 | 14 | 4 | 2 | 2 | layout |
| 60 | EP AmpSim -> Phaser | 16 | 14 | 4 | 2 | 3 | layout |
| 61 | EP AmpSim -> Delay | 16 | 13 | 4 | 4 | 5 | layout |
| 62 | Enhancer -> Chorus | 9 | 7 | 5 | 2 | 2 | layout |
| 63 | Enhancer -> Flanger | 10 | 8 | 5 | 2 | 2 | layout |
| 64 | Enhancer -> Delay | 9 | 7 | 4 | 1 | 1 | layout |
| 65 | Chorus -> Delay | 13 | 9 | 3 | 2 | 2 | layout |
| 66 | Flanger -> Delay | 14 | 10 | 3 | 2 | 2 | layout |
| 67 | Chorus -> Flanger | 14 | 10 | 3 | 2 | 2 | layout |

Name-only differences such as "Touch Wah Filter Type" vs Roland's "TWah Mode" count as the
same parameter. The CSV column `same_param_at_slot` records the match for each slot.

## Reproducing

[`mfx-param-gen.mjs`](mfx-param-gen.mjs) regenerates the per-type tables in `mfx_params.rs`
(the region between its `BEGIN/END GENERATED` markers):

```sh
node docs/research/mfx-param-gen.mjs crates/integral-core/src/mfx_params.rs
rustfmt --edition 2024 crates/integral-core/src/mfx_params.rs
```

It fetches R1 and R3 from the pinned commit and runs them in a Node `vm` context, so
Roland's own code builds the tables (no parsing). For each leaf it picks the `FxUnit`
whose output matches R3's display string for every stored value; anything else becomes a
name list.

## Device checks

Read-only RQ1 dumps of the MFX block (`<temporary tone> 00 02 00`, 145 bytes), checked
against the corrected tables. Each slot's stored value must lie within its
`stored_min..stored_max`.

- **16 Studio Set parts** as loaded, then **1,257 factory presets**, one at a time, in a
  writable test part. Categories: Organ, E.Piano, guitars, Vox/Choir, FX, then the
  rest, until we stopped. 53 presets got no reply, 47 of them while the MIDI session
  was down.
- **1,273 blocks covering 59 of the 68 types.** The new layout fits all but 8 blocks.
  The old tables don't fit 1,143.
- The 8 misfits:
  - 7 factory presets of **GuitarAmpSim → Delay (56)** store a delay time (300–600) in
    slot 11, Delay Switch (0–1). Slots 12–16 fit, so the layout is right, but a switch
    value above 1 is in the factory data. How the device reads it is open:
    [#235](https://github.com/sigma/integral/issues/235).
  - 1 part held values written through the old tables (e.g. 81 in a sync switch).
- The **Rotary 1 / 2 / 3** findings are confirmed on 28 / 2 / 19 presets, and so are the
  Delay-family triples. Humanizer is confirmed on 4 presets.
- **Not seen on the device:** 17 Auto Pan, 33 Gate, 49 Distortion → Flanger, 57–61
  EP AmpSim → \*, 67 Chorus → Flanger. No scanned factory preset uses them. They rest
  on R1 alone: [#235](https://github.com/sigma/integral/issues/235).
- **Writing only the type byte leaves the old slot values.** Writing just the MFX Type
  of a test part with a DT1 left all 32 slots unchanged, for every type. So a type
  change must also send the type's defaults (`mfx_params::default_slots`). The
  readback can't show the device's own defaults either.

## Follow-ups

- [#234](https://github.com/sigma/integral/issues/234): Control Assign 1–4 take a
  per-type destination list (R1 leaves 0–3), not slot numbers.
- [#235](https://github.com/sigma/integral/issues/235): verify the 9 unseen types and the
  type-56 switch values on the device.
- Foreign-patch MFX conversion
  ([#196](https://github.com/sigma/integral/issues/196)) must take its parameter
  numbers from the corrected table.
