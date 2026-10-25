# layouts/drum

## Overview

Grid layouts for the two drum kit editors. The PCM Drum screen has one row
per WMT layer that sums into the per-key Section row, and then the MFX. The
SN Drum screen has the key's TONE and OUTPUT, and then the MFX. Each kit's
KIT layout sits beside the key grid, off the screen: the editors render it
through a separate `kit` prop, outside `GridScreen`. Both kits build their LEVEL / PAN strip with the
same builder. The editors that render them are
[`../../PcmDrumEditor.tsx`](../../PcmDrumEditor.tsx) and
[`../../SnDrumEditor.tsx`](../../SnDrumEditor.tsx).

## Interface

- `levelPanStrip.ts`:
  - `levelPanStrip(spec)` — builds the 4-Cell-wide LEVEL / PAN Section:
    Pan, the FX1 and FX2 sends, then a scaled level fader down to the last
    row.
  - `LevelPanStripSpec<F>` — what the builder takes.
  - `LEVEL_PAN_MIN_H` — the shortest strip, equal to `ENV_CELLS.h`.
- `pcmDrum.ts`:
  - Field space: `PCMD_PARTIAL_FIELDS`.
  - Section layouts: `PCMD_KIT_LAYOUT`, `PCMD_WMT_LAYOUT`,
    `PCMD_WMT_SWITCH_LAYOUT` (`WMT {layer}`), `PCMD_PITCH_ENV_LAYOUT`,
    `PCMD_FILTER_ENV_LAYOUT`, `PCMD_AMP_ENV_LAYOUT`, `PCMD_FILTER_LAYOUT`,
    `PCMD_MISC_LAYOUT`, `PCMD_OUTPUT_LAYOUT`.
  - Families: `PCMD_WMT_FAMILY` and `PCMD_FAMILIES` (border family by key-row
    id).
  - The WMT row: `PcmdWmtRowSection`, `PCMD_WMT_ROW`, and `PCMD_SECTION`
    (placement ids `mfx` and `wmt-<layer>.<section>`), with
    `wmtRowSectionOf(id)` to parse an id back.
  - The screen: `pcmdScreen()` (memoized, WMT count from `deviceSpec()`).
  - `PCMD_SECTIONS` — every PCM Drum Section, for the validator.
- `snDrum.ts`:
  - Section layouts: `SND_KIT_LAYOUT`, `SND_TONE_LAYOUT`,
    `SND_OUTPUT_LAYOUT`.
  - `SND_FAMILIES` — border family by placement id.
  - `SND_SCREEN` — the screen: TONE and OUTPUT, a 2-Cell Lane, then the MFX,
    with an output → mfx arrow.

## Invariants

- Fields are wasm enums (`PcmdCommonField`, `PcmdPartialField`,
  `PcmdWmtField`, `SndCommonField`, `SndNoteField`), and their field spaces
  come from `enumFields` over core's spec functions.
- A LEVEL / PAN strip's `h` is at least `LEVEL_PAN_MIN_H`. Every PCM Drum
  key-row Section is `ENV_CELLS.h` tall, so the row stands even. SN Drum's
  key height is `max(VARIATION_H + 1, LEVEL_PAN_MIN_H)`.
- WMT ids follow `wmt-<layer>.<section>`. Per-key ids must match the keys of
  `PCMD_FAMILIES`. Every WMT row sums (`+`) into the first key-row Section,
  and the last one feeds `mfx` across a 1-Cell Lane.
- Envelope items and `pcmdScreen` read core on first use, not at import.
- Everything here is listed in [`../registry.ts`](../registry.ts). A Section
  that isn't registered is not validated.

## Side Effects

- None. Pure data. `memo()` fills its caches from the wasm core and
  `deviceSpec()` on first call.

## Verification

- Run: `just test-web` (`../layouts.test.ts` validates every registered
  layout)
