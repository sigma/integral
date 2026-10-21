/**
 * The mixer EQ Section, after the old ParametricEq: one knob per 2-Cell row,
 * zig-zagging down two columns, frequency (and Q) left, gain right, each band
 * starting in the column the previous one ended. Zig-zag rules divide the
 * bands; band names sit in the free slot beside a knob.
 */

import { MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { mixerFields } from "../fields";
import type { SectionLayout } from "../types";

/** The mixer EQ; it serves part, master and Comp+EQ strips. */
export const EQ_LAYOUT: SectionLayout<MixerField> = {
  title: "EQ",
  titleEdge: "top",
  w: 4,
  h: 16,
  fields: mixerFields([MixerScopeKind.Part, MixerScopeKind.Master, MixerScopeKind.CompEqUnit]),
  items: [
    // The title already names the switch.
    { kind: "switch", field: MixerField.EqSwitch, x: 1, y: 0, label: false },
    { kind: "knob", field: MixerField.EqHighFreq, x: 0, y: 1, label: "Freq" },
    { kind: "knob", field: MixerField.EqHighGain, x: 2, y: 3, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidFreq, x: 0, y: 5, label: "Freq" },
    { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 7, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidQ, x: 0, y: 9, label: "Q" },
    { kind: "knob", field: MixerField.EqLowGain, x: 2, y: 11, label: "Gain" },
    { kind: "knob", field: MixerField.EqLowFreq, x: 0, y: 13, label: "Freq" },
  ],
  // Rules run through the column centres and the gaps between knobs.
  silkscreen: [
    { kind: "rule", points: [[0, 4.5], [1, 4.5], [3, 6.5], [4, 6.5]] },
    { kind: "rule", points: [[0, 12.5], [1, 12.5], [3, 10.5], [4, 10.5]] },
    { kind: "label", text: "HI", x: 3, y: 2.5 },
    { kind: "label", text: "MID", x: 1, y: 8.5 },
    { kind: "label", text: "LO", x: 3, y: 14.5 },
  ],
};
