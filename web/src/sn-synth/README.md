# sn-synth

## Overview

The parts of the SuperNATURAL Synth tone editor: the COMMON Section, each
partial's switch strip and boxes, the partial Displays, and an OSC waveform
scope. Values, specs, relevance, tempo sync and Display slots come from the
wasm core. The Grid layouts are in
[`../layouts/sn-synth`](../layouts/sn-synth/README.md), and the page that
puts the parts together is [`../SnSynthEditor.tsx`](../SnSynthEditor.tsx).

## Interface

- `types.ts`:
  - Mirrors: `SnSynthCommon`, `SnSynthPartial`, and `SnSynthTone`
    (`{ common, partials }` as cached in core; MFX is read separately).
  - Handlers: `CommonParamHandler` and `PartialParamHandler`. A nibblized
    field's value is sent nibblized.
- `CommonSection.tsx`:
  - `CommonSection` (with `CommonSectionProps`) — renders
    `SNS_COMMON_LAYOUT`.
  - `PartialSwitch` (with `PartialSwitchProps`) — a partial's on/off switch
    strip.
- `PartialBox.tsx`: `PartialBox` (with `PartialBoxProps`) renders one box of
  a partial's row through `GridSection` inside `ScopeGate`.
- `displays.tsx`:
  - `partialSpec` — cached `sns_partial_spec`.
  - `SnsPartialDisplay` (with `SnsPartialDisplayProps`) — envelope, OSC
    wave, filter or LFO scope, chosen by `DisplayKind`.
- `OscWaveScope.tsx`: `OscWaveScope` draws two cycles of the OSC wave. It
  renders `null` for waves with no fixed shape (PCM).

## Invariants

- The partial switches are Common fields. `PartialSwitch` binds its own
  partial's field through `sns_partial_switch_field(idx)`, even though the
  layout names Partial 1's.
- A rate knob with sync info edits `sync.noteField` while
  `sync.switchField` is non-zero, and the free field otherwise. The toggle
  writes 1 or 0 to `switchField`.
- OSC alternatives: the PCM group shows while `OscWave` equals
  `sns_osc_wave_pcm()`. Super Saw shows while core holds `SuperSawDetune`
  active, and Variation otherwise.
- A partial that core holds switched off is dimmed but stays editable. Each
  Control is dimmed or hidden by `sns_partial_relevance`.
- In Display slots, a slot's `fixed` value wins over the field value.
- `OscWaveScope` matches core's formatted OSC Wave labels and clamps its
  0-127 inputs. Its pulse-width and super-saw shapes are marked `shortcut:`
  approximations.

## Side Effects

- React DOM and SVG rendering.
- Synchronous wasm calls, memoized through `cached` / `memo` / `useMemo`.
- No device I/O. Writes leave through the `onChange` / `setP` callbacks.
- `OscWaveScope` builds its noise trace once at module load from a
  fixed-seed generator.

## Verification

- Run: `just test-web` (`OscWaveScope.test.tsx`, plus
  `../SnSynthEditor.test.tsx`)
- Run: `just lint-web`
