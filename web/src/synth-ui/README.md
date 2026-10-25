# synth-ui

## Overview

Hardware-style synth Controls and Displays for the editor screens: knob,
fader, switch, picker and power button, plus envelope curves, LFO and filter
scopes. They are drawn in SVG and styled by one `.module.css` per component.
Each Control has a natural, freely laid-out size and a Grid mode that fills a
whole-Cell footprint from [`grid.ts`](./grid.ts). The
[`../layouts`](../layouts/README.md) renderer places them in Grid mode.
Parameter-family colours come from the wasm core
([ADR 0002](../../../docs/adr/0002-parameter-family-in-core.md)) as CSS custom
properties.

## Interface

Exported from `index.ts`:

- `SynthKnob` — rotary knob (-135° to +135°) with an LED ring lit in the
  family colour. Sizes `sm` and `lg`, or a Grid footprint.
- `SynthFader` — vertical slider. It can be compact, set a track height, sit
  on the Grid (`gridRows`), be read-only, or be a level fader with a scale
  (`scale` / `zeroDb`).
- `SynthSwitch` — a two-state toggle, or a vertical radiogroup option list
  with optional LEDs.
- `SynthPicker` — Grid picker (6x1 by default) that shows the name and the
  current option. It opens a `SearchPopover` that becomes searchable above
  `SHORT_LIST_MAX` (12) options.
- `PowerButton` — lit power toggle (`aria-pressed`).
- `SectionPanel` — labelled panel accented by a family or a named
  `SectionColour`.
- `ADSREnvelope`, `envelopePath`, `EnvelopeShape` — AD / AR / ADSR fader row
  with the shape drawn above it, and its path in a unit box.
- `EnvelopeScope` — envelope shape on a `ScopeScreen`, with no controls.
- `BreakpointCurve` — breakpoint-envelope curve that fills its box, used as a
  Grid Display.
- `FAMILIES`, `familyProperty`, `familyVar`, `accentColour`,
  `installFamilyPalette` — family palette as CSS custom properties
  (`--family-<name>`).
- `CELL` (24 px at zoom 1), `FOOTPRINT`, `cellsPx`, `Cells` — Grid units and
  fixed Control footprints: `knobSm` 2x3, `knobLg` 3x4, `switch` 2x1,
  `power` 1x1, `picker` 6x1, plus fader, option-list and dropdown widths.

Imported from the module file directly:

- `SynthFader.tsx`: `FADER_LABEL_HEIGHT`, `LEVEL`, `LEVEL_FRAME` (fader
  geometry).
- `SynthKnob.tsx`: `knobDialSide`.
- `SectionPanel.tsx`: `SectionColour`.
- `SynthPicker.tsx`: `SHORT_LIST_MAX`, `SynthPickerProps`.
- `BreakpointEnvelope.tsx`: `breakpointPath`.
- `ScopeScreen.tsx`: `ScopeScreen`, `sampled` (oscilloscope frame; traces in
  a 0..1 box).
- `LfoScope.tsx`: `LfoScope` (keyed by the core waveform label).
- `FilterScope.tsx`: `FilterScope`, `FilterResponse`, `SNS_FILTER_RESPONSES`,
  `CutoffModulation` (log-frequency gain plot with a modulation sweep band).
- `grid.ts`: option text metrics and helpers (`OPTION_TEXT`, `GLYPH_EM`,
  `OptionKind`, `optionChars`, `visibleOption`, `optionListRows`), edge
  metrics (`Edge`, `BLEED`), text lines (`NAME`, `READOUT`, `TextLine`,
  `lineStyle`, `BOTTOM_TO_TOP`), and `Px`, `faderCells`, `zoomOf`.
- `faderScale.ts`: `faderScale`, `ScaleMark`, `ScaleOf` (level-fader marks).

## Invariants

- Every value Control is controlled. It renders `value` and reports changes
  only through `onChange`, and holds no value state of its own.
- `SynthKnob` requires a `family: ParameterFamily` prop. Only its lit LEDs
  take the colour. `SectionPanel` takes exactly one of `family` or
  `sectionColour`. No component holds a family colour literal: family
  colours are read with `familyVar` from the custom properties
  `installFamilyPalette` sets. Fixed chrome colours are literals: the
  `drum` section colour, the switch LED default, knob dial and unlit LEDs,
  and the scope traces.
- `installFamilyPalette` needs the wasm core to be initialised first.
- Knob and fader drags round to integers and clamp to `[min, max]`.
  Double-click resets to `defaultValue`. Arrow keys step by 1, or 10 with
  Shift. A range of zero is guarded.
- Grid footprints come from the single `FOOTPRINT` table: fix a wrong
  footprint there, not per layout. A Grid Control fills its footprint (or the
  `box` passed to it) exactly at zoom 1. A fader thumb never leaves it.
- `SynthSwitch` draws a toggle only for exactly 2 options when not vertical:
  `options[0]` is off and `options[1]` is on. A vertical list is never shorter
  than `optionListRows`.
- Option text is drawn and measured from the same `OPTION_TEXT` metrics, so
  `visibleOption` / `optionChars` match what is rendered. The layout validator
  relies on this.
- Scopes and envelopes draw in a unit box with `y = 0` at the top.
  `LfoScope` and `FilterScope` render `null` for an unknown label. LFO random
  shapes are deterministic, so they don't flicker.

## Side Effects

- `SynthFader` and `SynthKnob` (via `useDragControl`) take pointer capture
  while dragging. Key handlers call `preventDefault` on the keys they use.
- `installFamilyPalette` writes CSS custom properties on a DOM element.
- `zoomOf` reads layout from the DOM.
- `SynthPicker` keeps open/closed state for its popover. Everything else is
  pure rendering.

## Verification

- Run: `just test-web` (Vitest + jsdom against the packed WASM)
- Run: `just lint-web` (`tsc -b`) and `just lint-css`
