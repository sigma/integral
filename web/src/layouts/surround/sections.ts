/**
 * The Motional Surround Sections: the common block (switch, room, depth),
 * its ambience, the pad Display, and one Section per part and the Ext Part.
 */

import { DisplayKind, MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { deviceSpec } from "../../deviceSpec";
import { FOOTPRINT } from "../../synth-ui/grid";
import { memo, mixerFields } from "../fields";
import { sectionCells, titleCells } from "../geometry";
import type { SectionLayout } from "../types";

const common = mixerFields([MixerScopeKind.Common]);

/** The surround switch, room and depth, in the Common scope. */
export const SURROUND_COMMON_LAYOUT: SectionLayout<MixerField> = {
  title: "Surround",
  titleEdge: "top",
  w: 9,
  h: 6,
  fields: common,
  // The switch and lists keep a free Cell from the left and bottom borders.
  items: [
    // The title already names the switch.
    { kind: "switch", field: MixerField.SurroundSwitch, x: 1, y: 0, label: false },
    { kind: "list", field: MixerField.SurroundRoomType, x: 3, y: 0, h: 5, label: "Room" },
    { kind: "list", field: MixerField.SurroundRoomSize, x: 5, y: 0, h: 4, label: "Size" },
    { kind: "knob", field: MixerField.SurroundDepth, x: 7, y: 0 },
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
  // 24 Cells square with its title, so a square grid of 2, 3, 4 or 6 part
  // Sections a side matches it in whole Cells.
  w: 24,
  h: 23,
  fields: position,
  // A free Cell from the left, right and bottom borders.
  items: [{ kind: "display", display: DisplayKind.SurroundPad, x: 1, y: 0, w: 22, h: 22 }],
};

/** Part Sections a side of the square grid beside the pad: enough for `partCount`. */
export function surroundGridSide(partCount: number): number {
  return Math.ceil(Math.sqrt(partCount));
}

/**
 * A part Section titled `title`: one of a `perSide`×`perSide` grid the pad
 * Section's size, its width and ambience send knobs side by side, centred
 * under the title.
 */
function partLayout(title: string, perSide: number): SectionLayout<MixerField> {
  const pad = sectionCells(SURROUND_PAD_LAYOUT);
  if (pad.w % perSide !== 0 || pad.h % perSide !== 0) {
    throw new Error(`the pad Section's ${pad.w}×${pad.h} Cells do not split into ${perSide} part Sections a side`);
  }
  const frame = { title, titleEdge: "top" } as const;
  const titleArea = titleCells({ ...frame, w: 0, h: 0 });
  const w = pad.w / perSide - titleArea.side;
  const h = pad.h / perSide - titleArea.top;
  const x = Math.floor((w - 2 * FOOTPRINT.knobSm.w) / 2);
  return {
    ...frame,
    w,
    h,
    fields: position,
    items: [
      { kind: "knob", field: MixerField.SurroundWidth, x, y: 0 },
      { kind: "knob", field: MixerField.SurroundAmbienceSend, x: x + FOOTPRINT.knobSm.w, y: 0, label: "Amb" },
    ],
  };
}

// Built on first use: they read the part count from core, which the app
// starts after its modules load.

/** One part's width and ambience send; a Part Scope binds it. */
export const surroundPartLayout: () => SectionLayout<MixerField> = memo(() =>
  partLayout("Part {part}", surroundGridSide(deviceSpec().part_count)),
);

/** The Ext Part's: the part Section under its own title, in the Ext scope. */
export const surroundExtLayout: () => SectionLayout<MixerField> = memo(() => ({
  ...surroundPartLayout(),
  title: "Ext",
}));
