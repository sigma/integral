/**
 * The SN Acoustic AMP Section: the A/R envelope over its Attack and Release
 * offset faders, and the Tone Level fader beside them.
 */

import { DisplayKind, SnaCommonField } from "../../../pkg/integral_wasm.js";
import type { SectionLayout } from "../types";
import { SNA_COMMON_FIELDS } from "./fields";

/** SN-A AMP; Atk and Rel dim for an instrument that ignores the tone offsets. */
export const SNA_AMP_LAYOUT: SectionLayout<SnaCommonField> = {
  title: "AMP",
  titleEdge: "top",
  w: 7,
  h: 6,
  fields: SNA_COMMON_FIELDS,
  items: [
    // The envelope keeps a free Cell from the left border.
    { kind: "display", display: DisplayKind.AmpEnvelope, x: 1, y: 0, w: 4, h: 2 },
    { kind: "fader", field: SnaCommonField.AttackTimeOffset, x: 1, y: 2, h: 4, label: "Atk" },
    { kind: "fader", field: SnaCommonField.ReleaseTimeOffset, x: 3, y: 2, h: 4, label: "Rel" },
    { kind: "fader", field: SnaCommonField.ToneLevel, x: 5, y: 0, h: 6, label: "Level" },
  ],
};
