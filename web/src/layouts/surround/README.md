# layouts/surround

## Overview

The Motional Surround page. The Common surround and Ambience Sections sit on
top, with the Position pad Display below them. Beside the pad is a square
grid of per-part Sections the pad's size, and the Ext Part Section is to its
right. [`../../SurroundPage.tsx`](../../SurroundPage.tsx) renders it.

## Interface

- `sections.ts`:
  - Section layouts: `SURROUND_COMMON_LAYOUT`, `SURROUND_AMBIENCE_LAYOUT`,
    `SURROUND_PAD_LAYOUT` (the 22x22 pad inside a 24-Cell-square Section).
  - `surroundGridSide(partCount)` — `ceil(sqrt(partCount))`.
  - `surroundPartLayout()` and `surroundExtLayout()` (memoized) — Width and
    Ambience Send knobs.
- `screen.ts`: `surroundScreen()` (memoized), `surroundPartIds()`
  (`part-<i>`, 0-based), `SURROUND_IDS`.

## Invariants

- The pad Section is 24 Cells square including its title, so grids of 2, 3,
  4 or 6 parts per side divide it into whole Cells. The part layout throws if
  they don't.
- One Section layout serves both a part and the Ext Part. Its fields are
  `mixerFields([Part, Ext])`: the surround position fields, which the mixer
  strip leaves out, plus Ambience Send, which the part strip also shows as
  its `AMB` knob.
- Parts fill the grid row by row.
- Part placement ids match `surroundPartIds()`.
- The part, Ext and screen layouts read `part_count` from core, so they are
  memoized functions. `registry.ts` lists them through `specSectionLayouts()`
  and `screenLayouts()`.

## Side Effects

- None. Pure data, memoized from `deviceSpec()` on first use.

## Verification

- Run: `just test-web`
