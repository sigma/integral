/**
 * The SN Synth partial row: one Section, repeated by Scope (`{ partial }`,
 * 0-based) for partials 1–3, with a side title. Five groups, OSC, FILTER,
 * AMP, MOD LFO and LFO, run left to right, divided by rules and named in
 * the top row. Each group stacks the same three bands: a Display (rows 1–2),
 * knobs (rows 3–5) and faders (rows 6–9), with its option lists down its
 * left edge, so the bands line up across the whole row.
 *
 * The OSC's scope and its PCM Wave picker are `alternatives` on one
 * rectangle; Var and S-Saw hide when core holds them inactive. Neither moves
 * a Control: switching the OSC Wave only changes what those Cells draw.
 */

import { DisplayKind, SnsPartialField as F, sns_partial_spec } from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import type { SectionLayout } from "../types";

/** The `alternatives` group holding the OSC scope or the PCM Wave picker. */
export const OSC_WAVE_GROUP = "osc-wave";

/** Its alternatives: the default draws the OSC Wave's scope. */
export const OSC_WAVE_ALTERNATIVE = { scope: "scope", pcm: "pcm" } as const;

/** The SN Synth Partial fields. */
export const SNS_PARTIAL_FIELDS = enumFields<F>("SnsPartialField", F, sns_partial_spec);

/** Group names, centred in the top row over each group. */
const GROUPS = [
  { text: "OSC", from: 0, to: 16 },
  { text: "FILTER", from: 16, to: 30 },
  { text: "AMP", from: 30, to: 42 },
  { text: "MOD LFO", from: 42, to: 52 },
  { text: "LFO", from: 52, to: 62 },
] as const;

/** Content height: the group names' row, then the three bands. */
const H = 10;

/** One SN Synth partial. */
export const SNS_PARTIAL_LAYOUT: SectionLayout<F> = {
  title: "Partial {partial}",
  titleEdge: "side",
  w: 62,
  h: H,
  fields: SNS_PARTIAL_FIELDS,
  items: [
    // OSC
    { kind: "list", field: F.OscWave, x: 0, y: 1, h: 9, label: "Wave" },
    // Var and S-Saw are never active together, so they share a column.
    { kind: "list", field: F.OscWaveVariation, x: 2, y: 1, h: 4, label: "Var", inactive: "hide" },
    { kind: "knob", field: F.SuperSawDetune, x: 2, y: 5, label: "S-Saw", inactive: "hide" },
    {
      kind: "alternatives",
      group: OSC_WAVE_GROUP,
      x: 4,
      y: 1,
      w: 6,
      h: 2,
      options: [
        { name: OSC_WAVE_ALTERNATIVE.scope, items: [{ kind: "display", display: DisplayKind.OscScope, x: 4, y: 1, w: 6, h: 2 }] },
        { name: OSC_WAVE_ALTERNATIVE.pcm, items: [{ kind: "display", display: DisplayKind.WavePicker, x: 4, y: 1, w: 6, h: 2 }] },
      ],
    },
    { kind: "knob", field: F.OscPitch, x: 4, y: 3, label: "Pitch" },
    { kind: "knob", field: F.HpfCutoff, x: 6, y: 3, label: "HPF" },
    { kind: "knob", field: F.OscDetune, x: 8, y: 3, label: "Detune" },
    { kind: "fader", field: F.OscPwModDepth, x: 4, y: 6, h: 4, label: "PWM" },
    { kind: "fader", field: F.OscPulseWidth, x: 6, y: 6, h: 4, label: "PW" },
    { kind: "fader", field: F.PwShift, x: 8, y: 6, h: 4, label: "Shift" },
    { kind: "display", display: DisplayKind.PitchEnvelope, x: 10, y: 1, w: 4, h: 2 },
    { kind: "fader", field: F.OscPitchEnvAttack, x: 10, y: 6, h: 4, label: "A" },
    { kind: "fader", field: F.OscPitchEnvDecay, x: 12, y: 6, h: 4, label: "D" },
    { kind: "fader", field: F.OscPitchEnvDepth, x: 14, y: 6, h: 4, label: "Dep" },

    // FILTER
    { kind: "list", field: F.FilterMode, x: 16, y: 1, h: 9, label: "Mode" },
    { kind: "list", field: F.FilterSlope, x: 18, y: 1, h: 3, label: "Slope" },
    { kind: "display", display: DisplayKind.FilterEnvelope, x: 20, y: 1, w: 8, h: 2 },
    { kind: "knob", field: F.FilterCutoff, x: 20, y: 3, label: "Cutoff" },
    { kind: "knob", field: F.FilterResonance, x: 22, y: 3, label: "Reso" },
    { kind: "knob", field: F.FilterKeyfollow, x: 24, y: 3, label: "KeyF" },
    { kind: "knob", field: F.FilterEnvVelSens, x: 26, y: 3, label: "Vel Sns" },
    { kind: "knob", field: F.AftertouchCutoff, x: 28, y: 3, label: "Aft Cut" },
    { kind: "fader", field: F.FilterEnvAttack, x: 20, y: 6, h: 4, label: "A" },
    { kind: "fader", field: F.FilterEnvDecay, x: 22, y: 6, h: 4, label: "D" },
    { kind: "fader", field: F.FilterEnvSustain, x: 24, y: 6, h: 4, label: "S" },
    { kind: "fader", field: F.FilterEnvRelease, x: 26, y: 6, h: 4, label: "R" },
    { kind: "fader", field: F.FilterEnvDepth, x: 28, y: 6, h: 4, label: "Dep" },

    // AMP
    { kind: "list", field: F.WaveGain, x: 30, y: 1, h: 5, label: "Gain" },
    { kind: "display", display: DisplayKind.AmpEnvelope, x: 32, y: 1, w: 8, h: 2 },
    { kind: "knob", field: F.AmpLevel, x: 32, y: 3, label: "Level" },
    { kind: "knob", field: F.AmpPan, x: 34, y: 3, label: "Pan" },
    { kind: "knob", field: F.AmpVelSens, x: 36, y: 3, label: "Vel Sns" },
    { kind: "knob", field: F.AftertouchLevel, x: 38, y: 3, label: "Aft Lvl" },
    { kind: "knob", field: F.AmpLevelKeyfollow, x: 40, y: 3, label: "KeyF" },
    { kind: "fader", field: F.AmpEnvAttack, x: 32, y: 6, h: 4, label: "A" },
    { kind: "fader", field: F.AmpEnvDecay, x: 34, y: 6, h: 4, label: "D" },
    { kind: "fader", field: F.AmpEnvSustain, x: 36, y: 6, h: 4, label: "S" },
    { kind: "fader", field: F.AmpEnvRelease, x: 38, y: 6, h: 4, label: "R" },

    // MOD LFO. The rate knob carries its tempo-sync toggle and edits the note while it is on (the caller's binding).
    { kind: "list", field: F.ModLfoShape, x: 42, y: 1, h: 7, label: "Shape" },
    { kind: "display", display: DisplayKind.ModLfoScope, x: 44, y: 1, w: 8, h: 2 },
    { kind: "knob", field: F.ModLfoRate, x: 44, y: 3, label: "Rate" },
    { kind: "knob", field: F.ModLfoRateControl, x: 46, y: 3, label: "Rate Ctrl" },
    { kind: "fader", field: F.ModLfoPitchDepth, x: 44, y: 6, h: 4, label: "Pit" },
    { kind: "fader", field: F.ModLfoFilterDepth, x: 46, y: 6, h: 4, label: "Flt" },
    { kind: "fader", field: F.ModLfoAmpDepth, x: 48, y: 6, h: 4, label: "Amp" },
    { kind: "fader", field: F.ModLfoPanDepth, x: 50, y: 6, h: 4, label: "Pan" },

    // LFO. The rate knob carries its tempo-sync toggle, as the MOD LFO's does.
    { kind: "list", field: F.LfoShape, x: 52, y: 1, h: 7, label: "Shape" },
    { kind: "switch", field: F.LfoKeyTrigger, x: 52, y: 9, label: "KeyTrig" },
    { kind: "display", display: DisplayKind.LfoScope, x: 54, y: 1, w: 8, h: 2 },
    { kind: "knob", field: F.LfoRate, x: 54, y: 3, label: "Rate" },
    { kind: "knob", field: F.LfoFadeTime, x: 56, y: 3, label: "Fade" },
    { kind: "fader", field: F.LfoPitchDepth, x: 54, y: 6, h: 4, label: "Pit" },
    { kind: "fader", field: F.LfoFilterDepth, x: 56, y: 6, h: 4, label: "Flt" },
    { kind: "fader", field: F.LfoAmpDepth, x: 58, y: 6, h: 4, label: "Amp" },
    { kind: "fader", field: F.LfoPanDepth, x: 60, y: 6, h: 4, label: "Pan" },
  ],
  silkscreen: [
    ...GROUPS.slice(1).map(({ from }) => ({ kind: "rule" as const, points: [[from, 0], [from, H]] as const })),
    ...GROUPS.map(({ text, from, to }) => ({ kind: "label" as const, text, x: (from + to) / 2, y: 0.5 })),
  ],
};
