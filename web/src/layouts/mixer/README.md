# layouts/mixer

## Overview

The mixer screen. It has one 4-Cell column per strip: the parts, Ext, the
optional Drum Comp+EQ units, Chorus, Reverb and Master. Each column stacks
an EQ Section, a strip from a variant set keyed by `MixerScopeKind` (or by
FX kind), and a name plate. [`../../MixerPage.tsx`](../../MixerPage.tsx)
renders it.

## Interface

- `eq.ts`: `EQ_LAYOUT` — the 4x17 EQ Section for the Part, Master and
  CompEqUnit scopes. Seven zig-zag knobs, with `EqSwitch` as the title
  switch.
- `strip.ts`:
  - `STRIP_SET` — 4x36 hand variants for Part, Ext, Master and
    CompEqUnit. Its `generate` throws.
  - `STRIP_KINDS`, `STRIP_TITLES`, `PART_TOGGLES` (`SoloPart`,
    `DrumCompEqPart`), `stripFields(kind)`.
  - `FX_STRIP_SET` — FX strips (generated), with `FxStripKind`,
    `FxStripFields` and `fxStripFields(kind)`.
- `screen.ts`:
  - `mixerScreen(compEq)` — the memoized `MixerScreen` (`{ layout, slots }`),
    with or without the Comp+EQ columns.
  - `MixerSlot` — what each placement draws.
  - `NAME_FRAME`.

## Invariants

- Every strip shares one 4x36 footprint with the level fader at
  `{y: 15, h: 21}`, so faders line up. FX strips reuse that footprint.
- A strip places every field of its Scope except two groups:
  - The EQ's fields, derived from `EQ_LAYOUT`.
  - The Motional Surround positions, which the Surround page places.

  It adds the `PART_TOGGLES` and the part's `SurroundAmbienceSend`.
- `STRIP_SET` has no generator: every `STRIP_KINDS` key needs a hand variant.
- Each FX strip's block has exactly one Switch, one Type and one Level field
  by core role. `fxStripFields` throws otherwise.
- The part strip's `sends` alternatives group (`fx` | `surround`) shares one
  rectangle. The caller picks which one is drawn.
- Placement ids:
  - `eq.<id>`, `strip.<id>` and `name.<id>`, where `<id>` is `part.N`,
    `ext`, `comp.N` or `master`.
  - `fx.chorus`, `fx.reverb`, `strip.chorus`, `strip.reverb`,
    `name.chorus`, `name.reverb`.

  The `slots` map describes every id.
- Column order: parts (`deviceSpec().part_count`), Ext (no EQ), Comp+EQ
  units, Chorus, Reverb, Master.
- Moving `EQ_LAYOUT`'s knobs moves the Chorus and Reverb knobs too
  ([`../fx`](../fx/README.md)).
- `EQ_LAYOUT`, `STRIP_SET`, `FX_STRIP_SET` and both `mixerScreen` variants
  stay registered in [`../registry.ts`](../registry.ts).

## Side Effects

- None. Pure data, with screens and FX fields memoized from `deviceSpec()`
  and the wasm core on first use.

## Verification

- Run: `just test-web` (`../layouts.test.ts`, `../../MixerPage.test.tsx`)
