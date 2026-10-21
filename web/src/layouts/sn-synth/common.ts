/**
 * The SN Synth COMMON Section: one top-titled row of knobs and switches.
 * The tone-wide knobs, MONO and LEGATO come first; UNISON, PORTAMENTO and
 * BEND follow, divided by rules and named in the free top row over their
 * Controls. The Partial Switches are not here: each heads its partial's row
 * (`SNS_PARTIAL_SWITCH_LAYOUT`).
 */

import { SnsCommonField as F, sns_common_spec } from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import type { SectionLayout } from "../types";

/** The SN Synth Common fields. */
export const SNS_COMMON_FIELDS = enumFields<F>("SnsCommonField", F, sns_common_spec);

/** Content height: the group names' row, then a knob. */
const H = 4;

/** Groups after the tone-wide Controls, each named over its columns. */
const GROUPS = [
  { text: "UNISON", from: 12, to: 16 },
  { text: "PORTAMENTO", from: 16, to: 22 },
  { text: "BEND", from: 22, to: 26 },
] as const;

/** SN Synth Common. */
export const SNS_COMMON_LAYOUT: SectionLayout<F> = {
  title: "COMMON",
  titleEdge: "top",
  w: 26,
  h: H,
  fields: SNS_COMMON_FIELDS,
  // Switches and the list sit level with the knobs' dials, clear of the bottom edge.
  items: [
    { kind: "knob", field: F.ToneLevel, x: 0, y: 1, label: "Level" },
    { kind: "knob", field: F.OctaveShift, x: 2, y: 1, label: "Octave" },
    { kind: "knob", field: F.WaveShape, x: 4, y: 1, label: "Shape" },
    { kind: "knob", field: F.AnalogFeel, x: 6, y: 1, label: "Analog" },
    { kind: "switch", field: F.MonoSwitch, x: 8, y: 2, label: "Mono" },
    { kind: "switch", field: F.LegatoSwitch, x: 10, y: 2, label: "Legato" },
    { kind: "switch", field: F.UnisonSwitch, x: 12, y: 2, label: "SW" },
    { kind: "knob", field: F.UnisonSize, x: 14, y: 1, label: "Size" },
    { kind: "switch", field: F.PortamentoSwitch, x: 16, y: 2, label: "SW" },
    { kind: "knob", field: F.PortamentoTime, x: 18, y: 1, label: "Time" },
    // NORMAL and LEGATO are too long for a switch: both shown, unnamed, as PORTAMENTO names them.
    { kind: "list", field: F.PortamentoMode, x: 20, y: 1, h: 2, label: false },
    { kind: "knob", field: F.PitchBendRangeDown, x: 22, y: 1, label: "Down" },
    { kind: "knob", field: F.PitchBendRangeUp, x: 24, y: 1, label: "Up" },
  ],
  silkscreen: [
    // MONO and LEGATO set apart from the knobs, then a rule before each group.
    ...[8, ...GROUPS.map(({ from }) => from)].map((x) => ({ kind: "rule" as const, points: [[x, 0], [x, H]] as const })),
    ...GROUPS.map(({ text, from, to }) => ({ kind: "label" as const, text, x: (from + to) / 2, y: 0.5, style: "title" as const })),
  ],
};
