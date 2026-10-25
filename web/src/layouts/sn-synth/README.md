# layouts/sn-synth

## Overview

The SuperNATURAL Synth editor screen. A COMMON Section sits on top. Under it
is one row per partial: a switch strip, then the OSC, FILTER, AMP, MOD LFO
and LFO boxes. Every row sums into the MFX. The parts that render it are in
[`../../sn-synth`](../../sn-synth/README.md).

## Interface

- `common.ts`: `SNS_COMMON_FIELDS`, `SNS_COMMON_LAYOUT`.
- `partial.ts`:
  - Field space: `SNS_PARTIAL_FIELDS`.
  - Box layouts: `SNS_OSC_LAYOUT`, `SNS_FILTER_LAYOUT`, `SNS_AMP_LAYOUT`,
    `SNS_MOD_LFO_LAYOUT`, `SNS_LFO_LAYOUT`.
  - Boxes in row order: `SNS_BOXES`, `SnsBox`, `boxOf(id)`.
  - `SNS_PARTIAL_SWITCH_LAYOUT` — the switch strip.
  - Alternatives groups: `OSC_WAVE_GROUP` with `OSC_WAVE_ALTERNATIVE`
    (`scope` | `pcm`), and `OSC_VARIATION_GROUP` with
    `OSC_VARIATION_ALTERNATIVE` (`variation` | `super-saw`).
- `screen.ts`:
  - `snsScreen()` — memoized, with the partial count from
    `sns_partial_count()`.
  - `SNS_SECTION` (`common`, `mfx`, `partial-<p>.<section>`), `SNS_ROW`,
    `SnsRowSection`, `rowSectionOf(id)`.

## Invariants

- Every box is 12 content rows tall, and its faders sit at `{y: 7, h: 5}`,
  so the faders line up across the row
  ([#427](https://github.com/sigma/integral/issues/427)). FILTER stacks four
  bands ([#430](https://github.com/sigma/integral/issues/430)).
- Each box keeps its first column and its last row free of pickers and
  Displays.
- Each alternatives group's options share one rectangle. The caller picks
  the option from core. The first option is the default.
- The switch strip names `Partial1Switch`. The caller binds the Scope's
  actual field through `sns_partial_switch_field`.
- Each row's last box sums (`+`) into `mfx` across a 1-Cell Lane.
- Everything here stays registered in [`../registry.ts`](../registry.ts).

## Side Effects

- None. Pure data, memoized from the wasm core on first use.

## Verification

- Run: `just test-web` (`partial.test.ts`, `../layouts.test.ts`,
  `../../SnSynthEditor.test.tsx`)
