# sn-drum

## Overview

The data model of the SuperNATURAL Drum Kit editor. It holds TypeScript
mirrors of the core's serde output for a kit's Common and per-note
parameters, and tables that tie the wasm field enums to mirror keys and
display names. The screen that uses them is
[`../SnDrumEditor.tsx`](../SnDrumEditor.tsx), and its Grid layouts are in
[`../layouts/drum`](../layouts/drum/README.md).

## Interface

All exports are in `types.ts`:

- `SnDrumCommon` — `kitName`, `kitLevel`, `ambienceLevel`, `phraseNumber`,
  `tfxSwitch`.
- `SnDrumNote` — one key: `instNumber`, level, pan, sends, tune, attack,
  decay, brilliance, variation, dynamic range, stereo width,
  `outputAssign`.
- `SND_COMMON_ENTRIES` — `Record<SndCommonField, FieldEntry<SnDrumCommon>>`.
- `SND_NOTE_ENTRIES` — `Record<SndNoteField, FieldEntry<SnDrumNote>>`, with
  short labels such as `Inst` and `Output`.

## Invariants

- The interfaces mirror the Rust serde output in camelCase.
- The entry tables are full `Record`s over the wasm enums, so every new core
  field needs an entry. `kitName` is not a Common entry.
- Values are plain numbers. Ranges come from core specs.

## Side Effects

- None. Pure types and constant tables.

## Verification

- Run: `just test-web`
- Run: `just lint-web`
