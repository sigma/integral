# pcm-drum

## Overview

The data model of the PCM Drum Kit editor. It holds TypeScript mirrors of the
core's serde output for a kit (Common, the per-key Partials, their WMT wave
layers, and Common2), plus tables that tie each wasm field enum to its mirror
key and display name. It has no components: the screen is
[`../PcmDrumEditor.tsx`](../PcmDrumEditor.tsx) and the key picker is
[`../DrumKeyGrid.tsx`](../DrumKeyGrid.tsx). Their Grid layouts are in
[`../layouts/drum`](../layouts/drum/README.md).

## Interface

All exports are in `types.ts`:

- Mirrors: `PcmDrumCommon` (`kitName`, `kitLevel`), `PcmDrumPartial` (one
  key: level, tune, pan, sends, envelopes, `wmt: PcmDrumWmt[]`), `PcmDrumWmt`
  (one WMT wave layer), `PcmDrumCommon2`.
- `PartialFieldKey` — the `PcmDrumPartial` keys holding `number` or
  `number[]`, so not `partialName` or `wmt`.
- Callback types: `WmtParamHandler` `(wmt, field, key, value)`,
  `PartialParamHandler` `(field, key, index, value)`, and `PartialSetter`
  (curried; `index` defaults to 0).
- Field tables (`FieldEntry` from [`../fieldSpec.ts`](../fieldSpec.ts)):
  `PCMD_COMMON_ENTRIES`, `PCMD_WMT_ENTRIES` and `PCMD_PARTIAL_ENTRIES`, keyed
  by the wasm enums `PcmdCommonField`, `PcmdWmtField` and `PcmdPartialField`.
- `noteName(key)` (MIDI note with octave, 60 gives `C4`) and
  `isBlackKey(key)`.

## Invariants

- The interfaces mirror the Rust serde output in camelCase. Rename a field in
  core and here together.
- The entry tables are full `Record`s over the wasm enums, so `tsc` fails
  when a new core field has no entry. `PCMD_COMMON_ENTRIES` maps only
  `KitLevel`, not `kitName`.
- Ranges, defaults and relevance are not encoded here. They come from core
  specs.
- Envelope time and level fields are arrays. Handlers address one element by
  `index`, which is 0 for scalar fields. WMT layers are addressed apart from
  the other Partial fields, through `WmtParamHandler` with a 0-based layer.

## Side Effects

- None. Pure types, constant tables and pure helpers. The only runtime
  import is the wasm enum objects.

## Verification

- Run: `just test-web` (exercised through `PcmDrumEditor.test.tsx`)
- Run: `just lint-web`
