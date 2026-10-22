/**
 * The mixer EQ Section, after the old ParametricEq: one knob per 2-Cell row,
 * zig-zagging down two columns, frequency (and Q) left, gain right, each band
 * starting in the column the previous one ended. Zig-zag rules divide the
 * bands; band names sit in the free slot beside a knob.
 *
 * The switch is in the title, so the knobs start on the first row. The
 * staggered Chorus and Reverb Sections beside it (`layouts/fx/strip.ts`)
 * start theirs on the same row and column, so each FX knob is level with an
 * EQ knob; the Section is as tall as they are, with blank rows under its last
 * knob.
 */

import { MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { mixerFields } from "../fields";
import type { SectionLayout } from "../types";

/** The mixer EQ; it serves part, master and Comp+EQ strips. */
export const EQ_LAYOUT: SectionLayout<MixerField> = {
  title: "EQ",
  titleEdge: "top",
  w: 4,
  h: 17,
  fields: mixerFields([MixerScopeKind.Part, MixerScopeKind.Master, MixerScopeKind.CompEqUnit]),
  // The switch is a power toggle in the title: "EQ ⏻".
  titleSwitch: MixerField.EqSwitch,
  items: [
    { kind: "knob", field: MixerField.EqHighFreq, x: 0, y: 0, label: "Freq" },
    { kind: "knob", field: MixerField.EqHighGain, x: 2, y: 2, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidFreq, x: 0, y: 4, label: "Freq" },
    { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 6, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidQ, x: 0, y: 8, label: "Q" },
    { kind: "knob", field: MixerField.EqLowGain, x: 2, y: 10, label: "Gain" },
    { kind: "knob", field: MixerField.EqLowFreq, x: 0, y: 12, label: "Freq" },
  ],
  // Rules run through the column centres and the gaps between knobs.
  silkscreen: [
    { kind: "rule", points: [[0, 3.5], [1, 3.5], [3, 5.5], [4, 5.5]] },
    { kind: "rule", points: [[0, 11.5], [1, 11.5], [3, 9.5], [4, 9.5]] },
    { kind: "label", text: "HI", x: 3, y: 1.5 },
    { kind: "label", text: "MID", x: 1, y: 7.5 },
    { kind: "label", text: "LO", x: 3, y: 13.5 },
  ],
};
