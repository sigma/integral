# layouts

## Overview

The Grid layout system ([ADR 0005](../../../docs/adr/0005-layouts-as-typescript-data.md)).
A Layout is plain TypeScript data that places Sections, Controls, Displays,
alternatives, Lanes and Silkscreen in whole Cells, with Silkscreen in half
Cells. This directory holds:

- the layout format;
- the shared geometry;
- arrow routing, computed from the layout and never measured from the DOM;
- a validator that a test runs over every registered layout;
- the React renderers.

The per-screen declarations live in the subdirectories. A layout names
fields and Displays the wasm core defines, and never defines a parameter or
a predicate itself. Terms (Grid, Cell, Section, Control, Display, Lane,
Scope, Variant set) are defined under "Layout" in
[`CONTEXT.md`](../../../CONTEXT.md).

## Interface

- [`index.ts`](./index.ts) re-exports the public surface: the types, plus
  selected symbols from the modules below and from `fx/`.
- `types.ts` — the layout format (types only):
  - Controls: `KnobLayout`, `SwitchLayout`, `PowerLayout`, `FaderLayout`,
    `OptionListLayout`, `PickerLayout`.
  - Displays and alternatives: `DisplayLayout`, `AlternativesLayout`.
  - Silkscreen: `RuleSilk`, `GroupNameSilk`, `SectionArrowSilk`.
  - Sections: `SectionFrame`, `SectionBody`, `SectionLayout`.
  - Variant sets: `VariantSet`, `RegionVariantSet`.
  - Screens: `ScreenLayout` with `SectionPlacement`, `Lane` and
    `ScreenArrowSilk`.
  - Regions: `ScreenRegion` (what `RegionVariantSet` maps keys to).
  - Arrows and endpoints: `SectionSilk`, `SectionEndpoint`, `ItemEndpoint`,
    `ArrowGlyph`, `Side`, `TitleEdge`.
  - Unions: `ControlLayout`, `ElementLayout`, `ItemLayout`, `Alternative`.
  - Also `Scope`, `Inactive` (`"dim" | "hide"`) and `CellRect`.
- `fields.ts` — the fields a layout may name (`FieldSpace`, `FieldKey`):
  - Builders: `enumFields` (a wasm enum plus its spec lookup),
    `mixerFields`, and `fxFields` / `fxTypes` / `fxParamDefs`.
  - Helpers: `elementCount`, `isTwoState`, and `memo` (lazy, safe to build
    before wasm init).
- `geometry.ts` — footprints and placement:
  - Footprints: `controlCells`, `itemRect`, `sectionCells`, `titleCells`,
    `DISPLAY_MIN_FOOTPRINT`.
  - Border insets: `INSET`, `BORDER`, `edgePadding`.
  - Placement: `placeRow`, `placeLine`, `variantLayout`, `readingOrder`,
    `drawnElements`, `rowSectionLookup`, `sectionTitle`.
- `routing.ts` — `route`, `routeScreenArrow`, `routeSectionArrow`,
  `drawnArrows`, `glyphClashes`.
- `validate.ts` — `validateSection`, `validateVariantSet`, `validateScreen`
  and `validateRegionSet`. Each returns a list of `Problem`s.
- `registry.ts` — `SECTION_LAYOUTS`, `specSectionLayouts`, `VARIANT_SETS`,
  `screenLayouts`, `REGION_VARIANT_SETS`.
- `region.ts` — `freeLanes`, `placeRegion`.
- `switchStrip.ts` — `switchStrip(spec)` with `SwitchStripSpec`: the
  side-titled Switch strip Section.
- `envelope.ts` — `envelopeItems`, `ENV_CELLS`: the breakpoint envelope
  block, with faders from core's Display binding.
- `binding.ts` — `specBinding`: a `Binding` from a core `WasmFieldSpec`.
- `displays.tsx` — `displaySlots`, `slotOf`, `slotField` (core Display
  bindings), `SlotReader`, `SlotValue`, and the `BreakpointDisplay`
  component.
- Renderers:
  - `GridSection` (`GridSectionProps`) — one Section from its layout plus
    a caller `bind(field, index)` returning a `Binding` (with
    `BindingOption` and `SyncBinding`). `loadedRelevance` is its relevance
    helper.
  - `GridScreen` — places a screen's Sections and arrows.
  - `SilkArrows`, `ScopeGate` (dims a group by Scope relevance).
  - `GridZoom` (`GridZoomProps`), with `useFitZoom`, `fitZoom`,
    `roundZoom`, `naturalSize` / `NaturalSize`, `ZOOM_STEP` and `MIN_ZOOM`
    — whole-page Zoom.

Subdirectories (per-screen declarations, all collected by `registry.ts`):

- [`drum`](./drum/README.md) — PCM Drum and SN Drum kits, and the shared
  LEVEL / PAN strip.
- [`fx`](./fx/README.md) — the MFX, Chorus and Reverb variant sets keyed by
  FX type, and the MFX panel frame.
- [`mixer`](./mixer/README.md) — the mixer screen: EQ, strip variant sets,
  name plates.
- [`pcm-synth`](./pcm-synth/README.md) — PCM Synth COMMON, PMT, the
  partial boxes, and the pair region set keyed by Structure Type.
- [`sn-acoustic`](./sn-acoustic/README.md) — SN Acoustic Sections and the
  INST variant set keyed by instrument.
- [`sn-synth`](./sn-synth/README.md) — SN Synth COMMON and the per-partial
  rows.
- [`surround`](./surround/README.md) — the Motional Surround page.

## Invariants

- Lattice: Sections, Controls, Displays, alternatives and Lanes sit on whole
  Cells with a positive size. Silkscreen sits on the half-Cell lattice.
- Overlap: no two items share a Cell. The only exception is the choices of
  one alternatives group, which share the group's Cells.
- Containment: everything stays inside its Section, its variant footprint or
  the screen grid. A Display is at least `DISPLAY_MIN_FOOTPRINT` for its
  kind.
- Fields:
  - Every field a Control names is in its core-provided `FieldSpace`.
  - Switch, power and title-switch Controls bind two-state fields.
  - An array field needs an in-range `index`, and a scalar field has none.
- Placement:
  - A variant places every parameter of its key, each array element on its
    own.
  - A region variant places exactly the Sections its key yields.
  - Variant keys are the keys core enumerates.
- Labels: no two options of a list or switch read the same once cut to their
  width (`visibleOption`).
- Lanes and arrows:
  - No Section covers a Lane.
  - Arrows must route: screen arrows through Lanes, Section arrows through
    free Cells.
  - Routing is deterministic: fewest bends, then shortest, then a fixed step
    order.
- A layout holds no predicates. Relevance and which alternative is drawn come
  from core through the caller. An inactive Control keeps its Cells (dimmed
  or hidden), and nothing reflows.
- Variant sets share one footprint across keys, so changing the key never
  moves a neighbour.
- Field spaces, Display bindings and anything that reads `deviceSpec()` are
  computed on first use, so these modules import safely before `initWasm`.
- Zoom: the page is laid out at zoom 1 and scaled with CSS `zoom`, in 5%
  steps, never below 1. A wider page scrolls.
- A new layout must be added to `registry.ts`. The validator test only sees
  what the registry lists.

## Side Effects

- React DOM and SVG rendering (`GridSection`, `GridScreen`, `SilkArrows`,
  `ScopeGate`, `GridZoom`, `BreakpointDisplay`).
- Synchronous wasm calls for field specs, FX tables, catalog options and
  Display bindings, with module-level caches (`memo`, `cached`, the Display
  bindings map).
- `useFitZoom` attaches a `ResizeObserver`.
- No device writes. Edits leave through the caller's `Binding.onChange`.
- The validator and routing are pure.

## Verification

- Run: `just test-web` (`layouts.test.ts` runs the validator over every
  registered layout and variant key; `routing.test.ts`; `zoom.test.tsx`;
  `switchStrip.test.tsx`; `GridSection.test.tsx`; `envelopeSpan.test.ts`)
- Run: `just lint-web`, `just lint-css`
