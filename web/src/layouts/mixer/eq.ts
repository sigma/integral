/**
 * The mixer EQ Section, after the old ParametricEq: one knob per 2-Cell row,
 * zig-zagging down two columns, frequency (and Q) left, gain right, each band
 * starting in the column the previous one ended. Zig-zag rules divide the
 * bands; band names sit in the free slot beside a knob.
 *
 * A free row under the switch makes the Section as tall as the staggered
 * Chorus and Reverb Sections beside it (`layouts/fx/strip.ts`), and puts its
 * knobs on their rows.
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
    { kind: "knob", field: MixerField.EqHighFreq, x: 0, y: 2, label: "Freq" },
    { kind: "knob", field: MixerField.EqHighGain, x: 2, y: 4, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidFreq, x: 0, y: 6, label: "Freq" },
    { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 8, label: "Gain" },
    { kind: "knob", field: MixerField.EqMidQ, x: 0, y: 10, label: "Q" },
    { kind: "knob", field: MixerField.EqLowGain, x: 2, y: 12, label: "Gain" },
    { kind: "knob", field: MixerField.EqLowFreq, x: 0, y: 14, label: "Freq" },
  ],
  // Rules run through the column centres and the gaps between knobs.
  silkscreen: [
    { kind: "rule", points: [[0, 5.5], [1, 5.5], [3, 7.5], [4, 7.5]] },
    { kind: "rule", points: [[0, 13.5], [1, 13.5], [3, 11.5], [4, 11.5]] },
    { kind: "label", text: "HI", x: 3, y: 3.5 },
    { kind: "label", text: "MID", x: 1, y: 9.5 },
    { kind: "label", text: "LO", x: 3, y: 15.5 },
  ],
};
