/**
 * The SN Synth partial row: a strip with the partial's switch, then five
 * boxes, OSC, FILTER, AMP, MOD LFO and LFO, each its own top-titled Section,
 * repeated by Scope (`{ partial }`, 0-based) for partials 1–3. Every box
 * but FILTER stacks three bands: dropdowns, scopes, the OSC's Wave knob and
 * AMP's knobs (rows 0–2), an envelope Display or knobs directly over their
 * faders (rows 3–6) and faders (rows 7–11), so the faders line up across the row (#427): an
 * envelope Display sits directly over its faders, OSC's Wave scope over its
 * pitch envelope, and an LFO's scope over its knobs. FILTER stacks four
 * (#430): its response scope, its knobs, its envelope and its faders. A
 * box's dropdowns sit at its left. Each box leaves its first column and last
 * row free of pickers and Displays.
 *
 * Two `alternatives` groups follow the OSC Wave, each on one rectangle: the
 * OSC's scope or its PCM Wave picker (a Control), and right of the Wave dropdown
 * the Var knob or, for SP-SAW, the S-Saw knob; the caller picks each from
 * core. Var hides when core holds it inactive. Neither moves a Control:
 * switching the OSC Wave only changes what those Cells draw.
 */

import {
  DisplayKind,
  ParameterFamily,
  SnsCommonField,
  SnsPartialField as F,
  sns_partial_spec,
} from "../../../pkg/integral_wasm.js";
import { FOOTPRINT } from "../../synth-ui/grid";
import { enumFields } from "../fields";
import { switchStrip } from "../switchStrip";
import type { SectionLayout } from "../types";
import { SNS_COMMON_FIELDS } from "./common";

/** The `alternatives` group holding the OSC scope or the PCM Wave picker. */
export const OSC_WAVE_GROUP = "osc-wave";

/** Its alternatives: the default draws the OSC Wave's scope. */
export const OSC_WAVE_ALTERNATIVE = { scope: "scope", pcm: "pcm" } as const;

/** The `alternatives` group right of the Wave dropdown: the Wave Variation knob, or the Super Saw Detune knob. */
export const OSC_VARIATION_GROUP = "osc-variation";

/** Its alternatives: the default draws the Wave Variation knob. */
export const OSC_VARIATION_ALTERNATIVE = { variation: "variation", superSaw: "super-saw" } as const;

/** The SN Synth Partial fields. */
export const SNS_PARTIAL_FIELDS = enumFields<F>("SnsPartialField", F, sns_partial_spec);

/** Content height of every box: the top band, the middle band and the faders. */
const H = 12;

/** Faders' top row and height: the bottom band. */
const FADER = { y: H - 5, h: 5 } as const;

/** The middle band's top row: an envelope Display there runs down to its faders. */
const MID_Y = 3;

/** Knobs' top row in the boxes whose knobs sit directly over their faders. */
const KNOB_Y = FADER.y - FOOTPRINT.knobSm.h;

/** A partial's box: top-titled, `w`×{@link H}, its items and Silkscreen given. */
function box(
  title: string,
  w: number,
  items: SectionLayout<F>["items"],
  silkscreen?: SectionLayout<F>["silkscreen"],
): SectionLayout<F> {
  return { title, titleEdge: "top", w, h: H, fields: SNS_PARTIAL_FIELDS, items, ...(silkscreen ? { silkscreen } : {}) };
}

/** OSC's pitch envelope column: its Display, the Wave's scope over it, and its A, D and Dep faders. */
const OSC_ENV_X = 8;

/** Its width: the A, D and Dep faders. */
const OSC_ENV_W = 3 * FOOTPRINT.faderW;

/**
 * OSC: on the left, Pitch, HPF and Detune over the PWM, PW and Shift faders;
 * on the right, past a rule, the pitch envelope over its A, D and Dep faders,
 * and the Wave's scope over the envelope, with the Wave dropdown and its
 * variation (or S-Saw detune) knob to the scope's left.
 */
export const SNS_OSC_LAYOUT = box(
  "OSC",
  OSC_ENV_X + OSC_ENV_W,
  [
    { kind: "picker", field: F.OscWave, x: 1, y: 1, w: FOOTPRINT.dropdownW, label: "Wave" },
    {
      kind: "alternatives",
      group: OSC_VARIATION_GROUP,
      x: 5,
      y: 0,
      w: 2,
      h: 3,
      options: [
        {
          name: OSC_VARIATION_ALTERNATIVE.variation,
          // Var hides on PCM, where neither knob applies; S-Saw only shows where it applies.
          items: [{ kind: "knob", field: F.OscWaveVariation, x: 5, y: 0, label: "Var", inactive: "hide" }],
        },
        { name: OSC_VARIATION_ALTERNATIVE.superSaw, items: [{ kind: "knob", field: F.SuperSawDetune, x: 5, y: 0, label: "S-Saw" }] },
      ],
    },
    {
      kind: "alternatives",
      group: OSC_WAVE_GROUP,
      x: OSC_ENV_X,
      y: 0,
      w: OSC_ENV_W,
      h: 2,
      options: [
        {
          name: OSC_WAVE_ALTERNATIVE.scope,
          items: [{ kind: "display", display: DisplayKind.OscScope, x: OSC_ENV_X, y: 0, w: OSC_ENV_W, h: 2 }],
        },
        {
          name: OSC_WAVE_ALTERNATIVE.pcm,
          items: [{ kind: "picker", field: F.WaveNumber, x: OSC_ENV_X, y: 1, w: FOOTPRINT.dropdownW, label: "PCM" }],
        },
      ],
    },
    { kind: "knob", field: F.OscPitch, x: 1, y: KNOB_Y, label: "Pitch" },
    { kind: "knob", field: F.HpfCutoff, x: 3, y: KNOB_Y, label: "HPF" },
    { kind: "knob", field: F.OscDetune, x: 5, y: KNOB_Y, label: "Detune" },
    { kind: "fader", field: F.OscPwModDepth, x: 1, ...FADER, label: "PWM" },
    { kind: "fader", field: F.OscPulseWidth, x: 3, ...FADER, label: "PW" },
    { kind: "fader", field: F.PwShift, x: 5, ...FADER, label: "Shift" },
    { kind: "display", display: DisplayKind.PitchEnvelope, x: OSC_ENV_X, y: MID_Y, w: OSC_ENV_W, h: FADER.y - MID_Y },
    { kind: "fader", field: F.OscPitchEnvAttack, x: OSC_ENV_X, ...FADER, label: "A" },
    { kind: "fader", field: F.OscPitchEnvDecay, x: OSC_ENV_X + FOOTPRINT.faderW, ...FADER, label: "D" },
    { kind: "fader", field: F.OscPitchEnvDepth, x: OSC_ENV_X + 2 * FOOTPRINT.faderW, ...FADER, label: "Dep" },
  ],
  // The pitch envelope's column set apart from the PWM, PW and Shift column, down its free column.
  [{ kind: "rule", points: [[OSC_ENV_X - 0.5, MID_Y], [OSC_ENV_X - 0.5, H]] }],
);

/** FILTER's response scope height: its Mode and Slope dropdowns, stacked. */
const FILTER_SCOPE_H = 2;

/** FILTER's knobs' top row: under the response scope and its dropdowns. */
const FILTER_KNOB_Y = FILTER_SCOPE_H;

/** FILTER's columns, past its free first column: five knobs, each over its fader. */
const FILTER_X = [0, 1, 2, 3, 4].map((i) => 1 + i * FOOTPRINT.knobSm.w) as [number, number, number, number, number];

/**
 * FILTER, five knobs wide: the filter's response scope with the Mode and
 * Slope dropdowns stacked at its left, then Cutoff, Reso, KeyF, Vel Sns and
 * Aft Cut, then the filter envelope over its A, D, S, R and Dep faders.
 */
export const SNS_FILTER_LAYOUT = box("FILTER", 1 + FILTER_X.length * FOOTPRINT.knobSm.w, [
  { kind: "picker", field: F.FilterMode, x: 1, y: 0, w: FOOTPRINT.dropdownW, label: "Mode" },
  { kind: "picker", field: F.FilterSlope, x: 1, y: 1, w: FOOTPRINT.dropdownW, label: "Slope" },
  { kind: "display", display: DisplayKind.FilterScope, x: 1 + FOOTPRINT.dropdownW, y: 0, w: FILTER_X.length * FOOTPRINT.knobSm.w - FOOTPRINT.dropdownW, h: FILTER_SCOPE_H },
  { kind: "knob", field: F.FilterCutoff, x: FILTER_X[0], y: FILTER_KNOB_Y, label: "Cutoff" },
  { kind: "knob", field: F.FilterResonance, x: FILTER_X[1], y: FILTER_KNOB_Y, label: "Reso" },
  { kind: "knob", field: F.FilterKeyfollow, x: FILTER_X[2], y: FILTER_KNOB_Y, label: "KeyF" },
  { kind: "knob", field: F.FilterEnvVelSens, x: FILTER_X[3], y: FILTER_KNOB_Y, label: "Vel Sns" },
  { kind: "knob", field: F.AftertouchCutoff, x: FILTER_X[4], y: FILTER_KNOB_Y, label: "Aft Cut" },
  {
    kind: "display",
    display: DisplayKind.FilterEnvelope,
    x: FILTER_X[0],
    y: FILTER_KNOB_Y + FOOTPRINT.knobSm.h,
    w: FILTER_X.length * FOOTPRINT.knobSm.w,
    h: FADER.y - FILTER_KNOB_Y - FOOTPRINT.knobSm.h,
  },
  { kind: "fader", field: F.FilterEnvAttack, x: FILTER_X[0], ...FADER, label: "A" },
  { kind: "fader", field: F.FilterEnvDecay, x: FILTER_X[1], ...FADER, label: "D" },
  { kind: "fader", field: F.FilterEnvSustain, x: FILTER_X[2], ...FADER, label: "S" },
  { kind: "fader", field: F.FilterEnvRelease, x: FILTER_X[3], ...FADER, label: "R" },
  { kind: "fader", field: F.FilterEnvDepth, x: FILTER_X[4], ...FADER, label: "Dep" },
]);

/**
 * AMP: a row of knobs (wave gain, pan, velocity, aftertouch and key follow),
 * the amp envelope under it, over its A, D, S and R faders, and the level
 * fader right of them.
 */
export const SNS_AMP_LAYOUT = box("AMP", 11, [
  // Gain's values read as core names them.
  { kind: "knob", field: F.WaveGain, x: 1, y: 0, label: "Gain" },
  { kind: "knob", field: F.AmpPan, x: 3, y: 0, label: "Pan" },
  { kind: "knob", field: F.AmpVelSens, x: 5, y: 0, label: "Vel Sns" },
  { kind: "knob", field: F.AftertouchLevel, x: 7, y: 0, label: "Aft Lvl" },
  { kind: "knob", field: F.AmpLevelKeyfollow, x: 9, y: 0, label: "KeyF" },
  { kind: "display", display: DisplayKind.AmpEnvelope, x: 1, y: MID_Y, w: 8, h: FADER.y - MID_Y },
  { kind: "fader", field: F.AmpEnvAttack, x: 1, ...FADER, label: "A" },
  { kind: "fader", field: F.AmpEnvDecay, x: 3, ...FADER, label: "D" },
  { kind: "fader", field: F.AmpEnvSustain, x: 5, ...FADER, label: "S" },
  { kind: "fader", field: F.AmpEnvRelease, x: 7, ...FADER, label: "R" },
  { kind: "fader", field: F.AmpLevel, x: 9, ...FADER, label: "Level" },
]);

/** MOD LFO. Its rate knob carries its tempo-sync toggle and edits the note while it is on (the caller's binding). */
export const SNS_MOD_LFO_LAYOUT = box("MOD LFO", 9, [
  { kind: "picker", field: F.ModLfoShape, x: 1, y: 1, w: FOOTPRINT.dropdownW, label: "Shape" },
  { kind: "display", display: DisplayKind.ModLfoScope, x: 5, y: 0, w: 4, h: 2 },
  { kind: "knob", field: F.ModLfoRate, x: 5, y: KNOB_Y, label: "Rate" },
  // On the right edge the name loses the border's clearance: "Rate Ctrl" no longer fits.
  { kind: "knob", field: F.ModLfoRateControl, x: 7, y: KNOB_Y, label: "Rt Ctrl" },
  { kind: "fader", field: F.ModLfoPitchDepth, x: 1, ...FADER, label: "Pit" },
  { kind: "fader", field: F.ModLfoFilterDepth, x: 3, ...FADER, label: "Flt" },
  { kind: "fader", field: F.ModLfoAmpDepth, x: 5, ...FADER, label: "Amp" },
  { kind: "fader", field: F.ModLfoPanDepth, x: 7, ...FADER, label: "Pan" },
]);

/** LFO. Its rate knob carries its tempo-sync toggle, as the MOD LFO's does. */
export const SNS_LFO_LAYOUT = box("LFO", 9, [
  { kind: "picker", field: F.LfoShape, x: 1, y: 1, w: FOOTPRINT.dropdownW, label: "Shape" },
  { kind: "display", display: DisplayKind.LfoScope, x: 5, y: 0, w: 4, h: 2 },
  // The switch sits level with the knobs' dials.
  { kind: "switch", field: F.LfoKeyTrigger, x: 3, y: KNOB_Y + 1, label: "KeyTrig" },
  { kind: "knob", field: F.LfoRate, x: 5, y: KNOB_Y, label: "Rate" },
  { kind: "knob", field: F.LfoFadeTime, x: 7, y: KNOB_Y, label: "Fade" },
  { kind: "fader", field: F.LfoPitchDepth, x: 1, ...FADER, label: "Pit" },
  { kind: "fader", field: F.LfoFilterDepth, x: 3, ...FADER, label: "Flt" },
  { kind: "fader", field: F.LfoAmpDepth, x: 5, ...FADER, label: "Amp" },
  { kind: "fader", field: F.LfoPanDepth, x: 7, ...FADER, label: "Pan" },
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
