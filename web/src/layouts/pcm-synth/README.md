# layouts/pcm-synth

## Overview

The PCM Synth editor screen. It has COMMON and PARTIAL MIX Sections, then
two partial pairs. Each pair is a region variant set keyed by Structure
Type: every partial's switch strip and its boxes from LFO1 to OUTPUT, plus
BOOST and RING. The pairs are stacked beside the MFX.
[`../../pcm-synth/PcmSynthScreen.tsx`](../../pcm-synth/README.md) renders
it.

## Interface

- `sections.ts`:
  - Field spaces: `PCMS_COMMON_FIELDS`, `PCMS_PMT_FIELDS`,
    `PCMS_PARTIAL_FIELDS`.
  - Section layouts: `PCMS_COMMON_LAYOUT`, `PCMS_PMT_LAYOUT`,
    `PARTIAL_LAYOUT` (the `Partial {partial}` switch strip),
    `BOOST_12_LAYOUT` / `BOOST_34_LAYOUT`, and `RING_12_LAYOUT` /
    `RING_34_LAYOUT` (no Controls).
  - Boxes: `PcmsBox`, `PcmsBoxSpec`, `PCMS_BOXES` (every box with its
    numbered twin and family).
  - `ROW_H`.
- `pair.ts`:
  - `PCMS_PAIR` — the region variant set over Structure Types, with
    `pairSectionsOf(structure)`.
  - `PartialBlock`, `PairSection`, `partialBoxOf(section)`.
  - `LANE_Y`, `PATH_X`.
- `screen.ts`:
  - `pcmsScreen(structures)` — the screen for both pairs.
  - `pcmsScreens()` — every Structure, for the validator.
  - `PCMS_IDS`, `PAIR_Y`, `PCMS_SCREEN_CELLS`.
  - `pairSectionId(pair, section)` (prefix `p12.` / `p34.`) and
    `parsePairSectionId(id)`.

## Invariants

- Core's `pcms_structure_routing` decides which blocks a Structure has and
  their order. This directory only decides where they go. A variant places
  exactly the Sections `pairSectionsOf(key)` returns.
- Every Structure shares one footprint (sized for the widest path, Types
  5-8), and each pair sits at a fixed `PAIR_Y`. Changing a Structure moves
  nothing outside its pair.
- Lane heads keep fixed columns, and paths start at `PATH_X`. Arrows run left
  to right. Every `.out` Section sums (`+`) into `mfx` across a 1-Cell Lane.
- Every lane-row box is `BOX_H` tall, and the envelopes start on the same row
  so they line up.
- Envelope times and levels and the Step LFO steps are array fields bound by
  `index`. The number of steps comes from `pcms_partial_count`.
- Structure keys run over `pcms_pmt_spec(StructureType12)`'s range and are
  built lazily.
- Everything here stays registered in [`../registry.ts`](../registry.ts).

## Side Effects

- None. Pure data, with module caches filled from the wasm core on first use.

## Verification

- Run: `just test-web` (`pcm-synth.test.ts`, `../layouts.test.ts`,
  `../../pcm-synth/PcmSynthScreen.test.tsx`)
