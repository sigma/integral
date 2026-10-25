# layouts/sn-acoustic

## Overview

The SuperNATURAL Acoustic editor screen. COMMON, VIB, FILTER and AMP
Sections show the tone's Common fields. The INST variant set, keyed by
instrument, shows its Modify Parameter slots. All of them sit beside the
MFX. [`../../SnAcousticEditor.tsx`](../../SnAcousticEditor.tsx) renders it.

## Interface

- Section layouts:
  - `common.ts`: `SNA_COMMON_LAYOUT`
  - `vibrato.ts`: `SNA_VIBRATO_LAYOUT`
  - `filter.ts`: `SNA_FILTER_LAYOUT`
  - `amp.ts`: `SNA_AMP_LAYOUT`
- `fields.ts`:
  - `SNA_COMMON_FIELDS` — every Common field except `ModifyParameter`.
  - `ModifyParamDef`, `modifyParams(variation, number)`, `modifyDefs(key)`,
    `modifyFields(key)` — the Modify Parameter slots from core.
  - `instOf(key)`, `isTwoState(def)`.
- `inst.ts`:
  - `SNA_INST_SET` — 18x10, with keys and variants read lazily.
  - `AC_PIANO_BODY`, `TW_ORGAN_BODY` — hand bodies.
  - `packModifyParams(key)` — the generator for other instruments.
  - `instPage(key)`.
- `screen.ts`: `SNA_SECTION` (placement ids) and `SNA_SCREEN`.

## Invariants

- `ModifyParameter` is left out of `SNA_COMMON_FIELDS`. The INST set names
  its slots by number instead.
- INST keys are 1-based picker numbers, positions in `snaInsts()`. Modify
  Parameter definitions come from core's `sna_inst_params`.
- A hand body is chosen by core's page kind (`SnaInstPage.AcPiano` /
  `TwOrgan`), never by instrument number. Every other instrument is
  generated.
- Every INST variant fits one 18x10 footprint and places every slot
  `modifyFields(key)` lists. The `INST` title is a stand-in: the editor
  shows core's page name.
- Relevance (the dimmed Port, Atk/Rel and FILTER offsets) comes from core,
  not from the layout.
- `packModifyParams` is SN-A's own generator. It is due to be folded into a
  shared generator under [#360](https://github.com/sigma/integral/issues/360).
- Everything here stays registered in [`../registry.ts`](../registry.ts).

## Side Effects

- None. Pure data, with the INST keys and variants memoized from the wasm
  core on first use.

## Verification

- Run: `just test-web`
