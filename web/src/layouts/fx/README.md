# layouts/fx

## Overview

Effect parameter Sections as variant sets keyed by FX type:

- The MFX set (hand layouts for a few types, generated for the rest) and
  the frame that hosts the MFX panel on the tone editor screens.
- The mixer's staggered Chorus and Reverb Sections.

## Interface

- `set.ts`: `fxVariantSet(block, frame, variants, generate)` builds a
  `VariantSet` over an FX block. It reads the keys (`fxTypes`) and fields
  (`fxFields(block, type)`) lazily.
- `mfx.ts`:
  - `MFX_SET` — side-titled Parameters: 6x4 small-knob places plus a Level
    fader. Hand variants for types 1 (Equalizer), 22 (Chorus), 28/29
    (Drive), 31 (Compressor) and 34 (Delay). Every other type is generated.
  - `MFX_PANEL_FRAME` — the `MFX` frame placed as `mfx` on the drum, SN-A,
    SN-S and PCM-S screens.
  - `MFX_LEVEL` (`"Level"`), `MFX_PARAMS_Y`.
- `strip.ts`:
  - `CHORUS_SET`, `REVERB_SET` — every type generated.
  - `STRIP_CAPACITY` (8), `STRIP_ROWS`.

## Invariants

- Field keys are core's parameter-name strings (`"High Freq"`, `"Level"`). A
  hand layout's strings must match core's names exactly. A test checks them,
  not `tsc` ([ADR 0005](../../../../docs/adr/0005-layouts-as-typescript-data.md)).
- Every variant places every parameter of its type.
- Every MFX variant shares one footprint, with the Level fader on the same
  Cells, so changing the type moves nothing. The generator places Level on
  the fader, other params as knobs in core's order, and runs of two-state
  params as stacked switches.
- Chorus and Reverb Controls zig-zag in core's order from `FIRST_KNOB`. That
  origin comes from the first knob of the mixer's `EQ_LAYOUT`, so each FX knob
  sits level with an EQ knob.
- `STRIP_CAPACITY` is a layout choice. The validator fails if a type needs
  more, and a test fails if no type needs that many.
- Keys and fields are read from core on first use, because the wasm core
  loads after module evaluation.
- `MFX_SET`, `CHORUS_SET` and `REVERB_SET` stay in `VARIANT_SETS` in
  [`../registry.ts`](../registry.ts).

## Side Effects

- None. Pure data, with lazily cached keys.

## Verification

- Run: `just test-web`
