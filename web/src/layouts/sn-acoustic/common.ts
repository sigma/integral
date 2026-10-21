/** The SN Acoustic COMMON Section: Mono/Poly, Octave Shift and the Portamento Time offset. */

import { SnaCommonField } from "../../../pkg/integral_wasm.js";
import type { SectionLayout } from "../types";
import { SNA_COMMON_FIELDS } from "./fields";

/** SN-A COMMON; Port dims for an instrument that ignores the offset. */
export const SNA_COMMON_LAYOUT: SectionLayout<SnaCommonField> = {
  title: "COMMON",
  titleEdge: "top",
  w: 6,
  h: 3,
  fields: SNA_COMMON_FIELDS,
  items: [
    { kind: "switch", field: SnaCommonField.MonoPoly, x: 0, y: 1, label: "Mono/Poly" },
    { kind: "knob", field: SnaCommonField.OctaveShift, x: 2, y: 0, label: "Oct" },
    { kind: "knob", field: SnaCommonField.PortamentoTimeOffset, x: 4, y: 0, label: "Port" },
  ],
};
