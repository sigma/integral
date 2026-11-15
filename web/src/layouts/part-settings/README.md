# layouts/part-settings

## Overview

The Part settings drawer's Sections: one per group of core's Part settings
registry (`integral_core::part_settings`), stacked in one column.
[`../../PartSettingsDrawer.tsx`](../../PartSettingsDrawer.tsx) renders them.

## Interface

- `sections.ts`:
  - `PART_SETTINGS_W` — every Section's content width, eight small knobs.
  - `partSettingFields(group)` — the group's fields, from core.
  - `partSettingsLayout(group)` (memoized) — the group's Section, titled as
    core names the group; `undefined` for a group with no layout yet.
  - `partSettingsLayouts()` — every layout, for `registry.ts`.

## Invariants

- KBD, PITCH, OFFSET, MIDI and VOICE have layouts. SCALE has none until
  [#600](https://github.com/sigma/integral/issues/600): the drawer skips a
  group without one.
- A Control's label may shorten the registry's; its accessible name is the
  registry's label, so two "Fade Lo" knobs stay apart.
- The layouts read core for their titles and fields, so they are built on
  first call. `registry.ts` lists them through `specSectionLayouts()`.

## Side Effects

- None. Pure data, memoized from core on first use.

## Verification

- Run: `just test-web` (`layouts.test.ts` validates them,
  `PartSettingsDrawer.test.tsx` draws them)
