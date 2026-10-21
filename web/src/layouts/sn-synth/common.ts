/**
 * The SN Synth Common Section: one row of switches and knobs, divided by
 * rules into groups, with group names in the free bottom row. The Partial
 * Switches sit here, not in the partial rows: a row dims as a whole while
 * its partial is off, and its switch must stay live.
 */

import { SnsCommonField as F, sns_common_spec } from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import type { SectionLayout } from "../types";

/** Content height: a knob, and the group names' row under it. */
const H = 4;

/** SN Synth Common. */
export const SNS_COMMON_LAYOUT: SectionLayout<F> = {
  title: "Common",
  titleEdge: "side",
  w: 28,
  h: H,
  fields: enumFields<F>("SnsCommonField", F, sns_common_spec),
  items: [
    { kind: "switch", field: F.UnisonSwitch, x: 0, y: 0, label: "SW" },
    { kind: "knob", field: F.UnisonSize, x: 2, y: 0, label: "Size" },
    { kind: "switch", field: F.MonoSwitch, x: 4, y: 0, label: "MONO" },
    { kind: "switch", field: F.PortamentoSwitch, x: 6, y: 0, label: "SW" },
    { kind: "knob", field: F.PortamentoTime, x: 8, y: 0, label: "Time" },
    { kind: "switch", field: F.PortamentoMode, x: 10, y: 0, label: "Mode" },
    { kind: "knob", field: F.PitchBendRangeDown, x: 12, y: 0, label: "Down" },
    { kind: "knob", field: F.PitchBendRangeUp, x: 14, y: 0, label: "Up" },
    { kind: "knob", field: F.WaveShape, x: 16, y: 0, label: "Wave Shape" },
    { kind: "knob", field: F.ToneLevel, x: 18, y: 0, label: "Tone Level" },
    { kind: "knob", field: F.AnalogFeel, x: 20, y: 0, label: "Analog Feel" },
    { kind: "switch", field: F.LegatoSwitch, x: 22, y: 0, label: "LEGATO" },
    { kind: "knob", field: F.OctaveShift, x: 24, y: 0, label: "Oct Shift" },
    { kind: "switch", field: F.Partial1Switch, x: 26, y: 0, label: "Partial 1" },
    { kind: "switch", field: F.Partial2Switch, x: 26, y: 1, label: "Partial 2" },
    { kind: "switch", field: F.Partial3Switch, x: 26, y: 2, label: "Partial 3" },
  ],
  silkscreen: [
    ...[4, 6, 12, 16, 22, 26].map((x) => ({ kind: "rule" as const, points: [[x, 0], [x, H]] as const })),
    { kind: "label", text: "UNISON", x: 2, y: 3.5 },
    { kind: "label", text: "PORTAMENTO", x: 9, y: 3.5 },
    { kind: "label", text: "BEND", x: 14, y: 3.5 },
  ],
};
