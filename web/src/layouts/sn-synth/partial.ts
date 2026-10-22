/**
 * The SN Synth partial row: a strip with the partial's switch, then five
 * boxes, OSC, FILTER, AMP, MOD LFO and LFO, each its own top-titled Section,
 * repeated by Scope (`{ partial }`, 0-based) for partials 1–3. Every box
 * stacks the same three bands: a Display (rows 0–1), knobs (rows 2–4) and
 * faders (rows 5–9), with its option lists down its left side, so the bands
 * line up across the row. Each box leaves its first column and last row free
 * of option lists and Displays.
 *
 * The OSC's scope and its PCM Wave picker (a Control) are `alternatives` on one
 * rectangle; Var and S-Saw hide when core holds them inactive. Neither moves
 * a Control: switching the OSC Wave only changes what those Cells draw.
 */

import {
  DisplayKind,
  ParameterFamily,
  SnsCommonField,
  SnsPartialField as F,
  sns_partial_spec,
} from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import { switchStrip } from "../switchStrip";
import type { SectionLayout } from "../types";
import { SNS_COMMON_FIELDS } from "./common";

/** The `alternatives` group holding the OSC scope or the PCM Wave picker. */
export const OSC_WAVE_GROUP = "osc-wave";

/** Its alternatives: the default draws the OSC Wave's scope. */
export const OSC_WAVE_ALTERNATIVE = { scope: "scope", pcm: "pcm" } as const;

/** The SN Synth Partial fields. */
export const SNS_PARTIAL_FIELDS = enumFields<F>("SnsPartialField", F, sns_partial_spec);

/** Content height of every box: the three bands. */
const H = 10;

/** Faders' top row and height: the bottom band. */
const FADER = { y: 5, h: 5 } as const;

/** A partial's box: top-titled, `w`×{@link H}, its items given. */
function box(title: string, w: number, items: SectionLayout<F>["items"]): SectionLayout<F> {
  return { title, titleEdge: "top", w, h: H, fields: SNS_PARTIAL_FIELDS, items };
}

/** OSC: wave, variation and S-Saw detune, the wave's scope or picker, pitch and its envelope. */
export const SNS_OSC_LAYOUT = box("OSC", 17, [
  { kind: "list", field: F.OscWave, x: 1, y: 0, h: 9, label: "Wave" },
  // Var and S-Saw are never active together, so they share a column.
  { kind: "list", field: F.OscWaveVariation, x: 3, y: 0, h: 4, label: "Var", inactive: "hide" },
  { kind: "knob", field: F.SuperSawDetune, x: 3, y: 4, label: "S-Saw", inactive: "hide" },
  {
    kind: "alternatives",
    group: OSC_WAVE_GROUP,
    x: 5,
    y: 0,
    w: 6,
    h: 2,
    options: [
      { name: OSC_WAVE_ALTERNATIVE.scope, items: [{ kind: "display", display: DisplayKind.OscScope, x: 5, y: 0, w: 6, h: 2 }] },
      { name: OSC_WAVE_ALTERNATIVE.pcm, items: [{ kind: "picker", field: F.WaveNumber, x: 5, y: 0, label: "PCM" }] },
    ],
  },
  { kind: "knob", field: F.OscPitch, x: 5, y: 2, label: "Pitch" },
  { kind: "knob", field: F.HpfCutoff, x: 7, y: 2, label: "HPF" },
  { kind: "knob", field: F.OscDetune, x: 9, y: 2, label: "Detune" },
  { kind: "fader", field: F.OscPwModDepth, x: 5, ...FADER, label: "PWM" },
  { kind: "fader", field: F.OscPulseWidth, x: 7, ...FADER, label: "PW" },
  { kind: "fader", field: F.PwShift, x: 9, ...FADER, label: "Shift" },
  { kind: "display", display: DisplayKind.PitchEnvelope, x: 11, y: 0, w: 6, h: 2 },
  { kind: "fader", field: F.OscPitchEnvAttack, x: 11, ...FADER, label: "A" },
  { kind: "fader", field: F.OscPitchEnvDecay, x: 13, ...FADER, label: "D" },
  { kind: "fader", field: F.OscPitchEnvDepth, x: 15, ...FADER, label: "Dep" },
]);

/** FILTER: mode and slope, cutoff and its envelope. */
export const SNS_FILTER_LAYOUT = box("FILTER", 15, [
  { kind: "list", field: F.FilterMode, x: 1, y: 0, h: 9, label: "Mode" },
  { kind: "list", field: F.FilterSlope, x: 3, y: 0, h: 3, label: "Slope" },
  { kind: "display", display: DisplayKind.FilterEnvelope, x: 5, y: 0, w: 10, h: 2 },
  { kind: "knob", field: F.FilterCutoff, x: 5, y: 2, label: "Cutoff" },
  { kind: "knob", field: F.FilterResonance, x: 7, y: 2, label: "Reso" },
  { kind: "knob", field: F.FilterKeyfollow, x: 9, y: 2, label: "KeyF" },
  { kind: "knob", field: F.FilterEnvVelSens, x: 11, y: 2, label: "Vel Sns" },
  { kind: "knob", field: F.AftertouchCutoff, x: 13, y: 2, label: "Aft Cut" },
  { kind: "fader", field: F.FilterEnvAttack, x: 5, ...FADER, label: "A" },
  { kind: "fader", field: F.FilterEnvDecay, x: 7, ...FADER, label: "D" },
  { kind: "fader", field: F.FilterEnvSustain, x: 9, ...FADER, label: "S" },
  { kind: "fader", field: F.FilterEnvRelease, x: 11, ...FADER, label: "R" },
  { kind: "fader", field: F.FilterEnvDepth, x: 13, ...FADER, label: "Dep" },
]);

/** AMP: wave gain, level, pan and the amp envelope. */
export const SNS_AMP_LAYOUT = box("AMP", 13, [
  { kind: "list", field: F.WaveGain, x: 1, y: 0, h: 5, label: "Gain" },
  { kind: "display", display: DisplayKind.AmpEnvelope, x: 3, y: 0, w: 8, h: 2 },
  { kind: "knob", field: F.AmpLevel, x: 3, y: 2, label: "Level" },
  { kind: "knob", field: F.AmpPan, x: 5, y: 2, label: "Pan" },
  { kind: "knob", field: F.AmpVelSens, x: 7, y: 2, label: "Vel Sns" },
  { kind: "knob", field: F.AftertouchLevel, x: 9, y: 2, label: "Aft Lvl" },
  { kind: "knob", field: F.AmpLevelKeyfollow, x: 11, y: 2, label: "KeyF" },
  { kind: "fader", field: F.AmpEnvAttack, x: 3, ...FADER, label: "A" },
  { kind: "fader", field: F.AmpEnvDecay, x: 5, ...FADER, label: "D" },
  { kind: "fader", field: F.AmpEnvSustain, x: 7, ...FADER, label: "S" },
  { kind: "fader", field: F.AmpEnvRelease, x: 9, ...FADER, label: "R" },
]);

/** MOD LFO. Its rate knob carries its tempo-sync toggle and edits the note while it is on (the caller's binding). */
export const SNS_MOD_LFO_LAYOUT = box("MOD LFO", 11, [
  { kind: "list", field: F.ModLfoShape, x: 1, y: 0, h: 7, label: "Shape" },
  { kind: "display", display: DisplayKind.ModLfoScope, x: 3, y: 0, w: 7, h: 2 },
  { kind: "knob", field: F.ModLfoRate, x: 3, y: 2, label: "Rate" },
  { kind: "knob", field: F.ModLfoRateControl, x: 5, y: 2, label: "Rate Ctrl" },
  { kind: "fader", field: F.ModLfoPitchDepth, x: 3, ...FADER, label: "Pit" },
  { kind: "fader", field: F.ModLfoFilterDepth, x: 5, ...FADER, label: "Flt" },
  { kind: "fader", field: F.ModLfoAmpDepth, x: 7, ...FADER, label: "Amp" },
  { kind: "fader", field: F.ModLfoPanDepth, x: 9, ...FADER, label: "Pan" },
]);

/** LFO. Its rate knob carries its tempo-sync toggle, as the MOD LFO's does. */
export const SNS_LFO_LAYOUT = box("LFO", 11, [
  { kind: "list", field: F.LfoShape, x: 1, y: 0, h: 7, label: "Shape" },
  { kind: "switch", field: F.LfoKeyTrigger, x: 1, y: 7, label: "KeyTrig" },
  { kind: "display", display: DisplayKind.LfoScope, x: 3, y: 0, w: 7, h: 2 },
  { kind: "knob", field: F.LfoRate, x: 3, y: 2, label: "Rate" },
  { kind: "knob", field: F.LfoFadeTime, x: 5, y: 2, label: "Fade" },
  { kind: "fader", field: F.LfoPitchDepth, x: 3, ...FADER, label: "Pit" },
  { kind: "fader", field: F.LfoFilterDepth, x: 5, ...FADER, label: "Flt" },
  { kind: "fader", field: F.LfoAmpDepth, x: 7, ...FADER, label: "Amp" },
  { kind: "fader", field: F.LfoPanDepth, x: 9, ...FADER, label: "Pan" },
]);

/** A partial's boxes, left to right, each with the family its top border takes. */
export const SNS_BOXES = [
  { id: "osc", layout: SNS_OSC_LAYOUT, family: ParameterFamily.Pitch },
  { id: "filter", layout: SNS_FILTER_LAYOUT, family: ParameterFamily.Filter },
  { id: "amp", layout: SNS_AMP_LAYOUT, family: ParameterFamily.Amp },
  { id: "modLfo", layout: SNS_MOD_LFO_LAYOUT, family: ParameterFamily.Modulation },
  { id: "lfo", layout: SNS_LFO_LAYOUT, family: ParameterFamily.Modulation },
] as const;

/** A box's id within its row. */
export type SnsBox = (typeof SNS_BOXES)[number]["id"];

/** The box `id` names. */
export function boxOf(id: SnsBox): (typeof SNS_BOXES)[number] {
  return SNS_BOXES.find((b) => b.id === id)!;
}

/**
 * One partial's on/off, the switch strip (`switchStrip`) at the head of its
 * row, as in PCM Synth. The switch lives in Common, one field per partial:
 * the layout names Partial 1's, and the caller binds the Scope's
 * (`sns_partial_switch_field`).
 */
export const SNS_PARTIAL_SWITCH_LAYOUT: SectionLayout<SnsCommonField> = switchStrip({
  title: "Partial {partial}",
  field: SnsCommonField.Partial1Switch,
  fields: SNS_COMMON_FIELDS,
  h: H + 1,
});
