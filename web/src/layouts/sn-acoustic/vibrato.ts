/** The SN Acoustic VIB Section: the three vibrato offsets, stacked. */

import { SnaCommonField } from "../../../pkg/integral_wasm.js";
import type { SectionLayout } from "../types";
import { SNA_COMMON_FIELDS } from "./fields";

/** SN-A VIB: Rate, Depth and Delay in one column. */
export const SNA_VIBRATO_LAYOUT: SectionLayout<SnaCommonField> = {
  title: "VIB",
  titleEdge: "top",
  w: 2,
  h: 9,
  fields: SNA_COMMON_FIELDS,
  items: [
    { kind: "knob", field: SnaCommonField.VibratoRate, x: 0, y: 0, label: "Rate" },
    { kind: "knob", field: SnaCommonField.VibratoDepth, x: 0, y: 3, label: "Depth" },
    { kind: "knob", field: SnaCommonField.VibratoDelay, x: 0, y: 6, label: "Delay" },
  ],
};
