/** The SN Acoustic FILTER Section: the filter scope over the Cutoff and Resonance offsets. */

import { DisplayKind, SnaCommonField } from "../../../pkg/integral_wasm.js";
import type { SectionLayout } from "../types";
import { SNA_COMMON_FIELDS } from "./fields";

/** SN-A FILTER; both knobs dim for an instrument that ignores the tone offsets. */
export const SNA_FILTER_LAYOUT: SectionLayout<SnaCommonField> = {
  title: "FILTER",
  titleEdge: "top",
  w: 6,
  h: 5,
  fields: SNA_COMMON_FIELDS,
  items: [
    // The scope keeps a free Cell from the left and right borders.
    { kind: "display", display: DisplayKind.FilterScope, x: 1, y: 0, w: 4, h: 2 },
    { kind: "knob", field: SnaCommonField.CutoffOffset, x: 1, y: 2, label: "Cutoff" },
    { kind: "knob", field: SnaCommonField.ResonanceOffset, x: 3, y: 2, label: "Reso" },
  ],
};
