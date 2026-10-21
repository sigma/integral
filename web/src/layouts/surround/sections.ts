/**
 * The Motional Surround Sections: the common block (switch, room, depth),
 * its ambience, the pad Display, and one Section per part and the Ext Part.
 */

import { DisplayKind, MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { mixerFields } from "../fields";
import type { SectionLayout } from "../types";

const common = mixerFields([MixerScopeKind.Common]);

/** The surround switch, room and depth, in the Common scope. */
export const SURROUND_COMMON_LAYOUT: SectionLayout<MixerField> = {
  title: "Surround",
  titleEdge: "top",
  w: 8,
  h: 5,
  fields: common,
  items: [
    // The title already names the switch.
    { kind: "switch", field: MixerField.SurroundSwitch, x: 0, y: 0, label: false },
    { kind: "list", field: MixerField.SurroundRoomType, x: 2, y: 0, h: 5, label: "Room" },
    { kind: "list", field: MixerField.SurroundRoomSize, x: 4, y: 0, h: 4, label: "Size" },
    { kind: "knob", field: MixerField.SurroundDepth, x: 6, y: 0 },
  ],
};

/** The room's ambience, in the Common scope. */
export const SURROUND_AMBIENCE_LAYOUT: SectionLayout<MixerField> = {
  title: "Ambience",
  titleEdge: "top",
  w: 8,
  h: 3,
  fields: common,
  items: [
    { kind: "knob", field: MixerField.SurroundAmbienceLevel, x: 0, y: 0, label: "Level" },
    { kind: "knob", field: MixerField.SurroundAmbienceTime, x: 2, y: 0, label: "Time" },
    { kind: "knob", field: MixerField.SurroundAmbienceDensity, x: 4, y: 0, label: "Density" },
    { kind: "knob", field: MixerField.SurroundAmbienceHfDamp, x: 6, y: 0, label: "HF Damp" },
  ],
};

/** The parts' and the Ext Part's fields: one Section serves both. */
const position = mixerFields([MixerScopeKind.Part, MixerScopeKind.Ext]);

/** The pad: every part's position, dragged on one Display. */
export const SURROUND_PAD_LAYOUT: SectionLayout<MixerField> = {
  title: "Position",
  titleEdge: "top",
  w: 20,
  h: 20,
  fields: position,
  items: [{ kind: "display", display: DisplayKind.SurroundPad, x: 0, y: 0, w: 20, h: 20 }],
};

/** One part's width and ambience send; a Part Scope binds it. */
export const SURROUND_PART_LAYOUT: SectionLayout<MixerField> = {
  title: "Part {part}",
  titleEdge: "top",
  w: 4,
  h: 3,
  fields: position,
  items: [
    { kind: "knob", field: MixerField.SurroundWidth, x: 0, y: 0 },
    { kind: "knob", field: MixerField.SurroundAmbienceSend, x: 2, y: 0, label: "Amb" },
  ],
};

/** The Ext Part's: the part Section under its own title, in the Ext scope. */
export const SURROUND_EXT_LAYOUT: SectionLayout<MixerField> = { ...SURROUND_PART_LAYOUT, title: "Ext" };
