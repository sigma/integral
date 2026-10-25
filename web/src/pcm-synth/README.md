# pcm-synth

## Overview

The PCM Synth tone editor's screen and its parts. The four partials form two
Structure pairs (1-2 and 3-4), and each pair is routed by a Structure Type
that core defines. `PcmSynthScreen` places every Section on the Grid from
`pcmsScreen` in [`../layouts/pcm-synth`](../layouts/pcm-synth/README.md). The
modules here bind those Sections to the tone mirror and draw the Displays
that are specific to the PCM Synth. Ranges, relevance, tempo sync, Display
bindings and routing all come from the wasm core. The page wrapper is
[`../PcmSynthEditor.tsx`](../PcmSynthEditor.tsx).

## Interface

- `types.ts`:
  - Mirrors: `PcmSynthCommon`, `PcmSynthPmt` (with
    `partialEntries: PmtPartialEntry[]`), `PcmSynthPartial`,
    `PcmSynthCommon2`, and `PartialFieldKey`.
  - Handlers: `CommonParamHandler`, `PmtParamHandler`,
    `PmtEntryParamHandler`, `PartialParamHandler`.
  - Field tables: `PCMS_COMMON_ENTRIES` and `PCMS_PMT_ENTRIES` (partial
    records), and `PCMS_PARTIAL_ENTRIES` (full record).
- `binding.ts`:
  - `partialSpec` — the spec, cached.
  - `partialValue`, `commonValue`, `pmtValue` — read the mirror, falling
    back to the spec default.
  - `partialBinding(field, index, value, onChange)` — a `Binding` with
    `i of n` element names and the tempo-sync toggle.
- `structureRouting.ts`: `routingOf(structure)` wraps
  `pcms_structure_routing`. Types `Side`, `Block`, `Join`, `Routing`.
- `StructureDiagram.tsx`: `StructureDiagram` draws a pair's WG → TVF → TVA
  lanes and the join (`+`, `B` or `R`) in SVG.
- `displays.tsx`:
  - `PartialDisplay` (with `PartialDisplayProps`) — breakpoint envelopes,
    filter scope, LFO / Step LFO, wave picker.
  - `PmtDisplay` — the Structure diagram.
- `PcmSynthScreen.tsx`: `PcmSynthScreen` (with `PcmSynthScreenProps`)
  renders the whole screen: the COMMON, PMT and MFX Sections, plus each pair's
  partial switch, partial boxes, boost and ring.

## Invariants

- Partials are 0-based. Pair 0 is partials 0-1 and pair 1 is partials 2-3.
  The even partial is the upper side and the odd one is the lower side.
- Structure Type values are 0-9. With no join (Type 1), the partials stay
  independent and the boxes are not numbered.
- One field stores both the free and the note values of a tempo-synced
  parameter. Values at or above `sync.noteFrom` are notes, and switching mode
  resets the field to that mode's default.
- A partial that core holds switched off is dimmed through `ScopeGate` but
  stays editable. In a merged pair, an inactive upper-partial control reads
  `Follows partial N` when the same field is active on the lower partial.
- The ring Section has no Controls. The boost Section binds PMT fields with
  `pcms_pmt_relevance`.
- Fields that `PCMS_COMMON_ENTRIES` / `PCMS_PMT_ENTRIES` don't map read the
  core spec default. Never hardcode a default here.

## Side Effects

- React DOM and SVG rendering.
- Synchronous wasm calls for specs, relevance, routing, sync info and
  Display bindings. `partialSpec` and the sync lookup are memoized through
  `cached`.
- No device I/O. Edits leave through the caller's callbacks (`onCommon`,
  `onPmt`, `onPmtEntry`, `onPartial`, `mfxHandlers`).

## Verification

- Run: `just test-web` (`PcmSynthScreen.test.tsx`, `StructureDiagram.test.tsx`,
  `binding.test.ts`)
- Run: `just lint-web`, `just lint-css`
