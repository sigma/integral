/**
 * The SN Drum Kit editor: the kit's levels, beside the key grid, and the
 * selected key's Note.
 * The Note Sections take their Scope (`key`) from the key grid, so one
 * layout serves every key and changing the key only rebinds it.
 */

import {
  ParameterFamily,
  SndCommonField,
  SndNoteField,
  snd_common_spec,
  snd_note_spec,
} from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import { MFX_PANEL_FRAME } from "../fx/mfx";
import { placeRow, rowCells, sectionCells, type SectionRow } from "../geometry";
import type { ScreenLayout, SectionLayout } from "../types";
import { LEVEL_PAN_MIN_H, levelPanStrip } from "./levelPanStrip";

/** Height of the TONE's Variation list, in Cells: its options and its name. */
const VARIATION_H = 9;
/**
 * Height of the key Sections, in Cells: the Variation list and a free row
 * under it, grown so the LEVEL / PAN strip's fader is long enough to use.
 */
const KEY_H = Math.max(VARIATION_H + 1, LEVEL_PAN_MIN_H);
/** Length of the KIT Section's faders, in Cells. */
const KIT_FADER_H = 9;

/** The kit-wide levels. The editor sets them beside the key grid, off the key Sections' screen. */
export const SND_KIT_LAYOUT: SectionLayout<SndCommonField> = {
  title: "KIT",
  titleEdge: "top",
  w: 4,
  // The faders and a free row under them.
  h: KIT_FADER_H + 1,
  fields: enumFields("SndCommonField", SndCommonField, snd_common_spec),
  items: [
    {
      kind: "fader",
      field: SndCommonField.KitLevel,
      x: 0,
      y: 0,
      h: KIT_FADER_H,
      label: "Kit Lv",
    },
    {
      kind: "fader",
      field: SndCommonField.AmbienceLevel,
      x: 2,
      y: 0,
      h: KIT_FADER_H,
      label: "Amb Lv",
    },
  ],
};

const noteFields = enumFields("SndNoteField", SndNoteField, snd_note_spec);

/** The key's Inst and its shaping: the Inst picker over two rows of knobs, its Variation and Output beside them. */
export const SND_TONE_LAYOUT: SectionLayout<SndNoteField> = {
  title: "TONE",
  titleEdge: "top",
  w: 16,
  h: KEY_H,
  fields: noteFields,
  // The picker and lists keep a free Cell from the left, right and bottom borders.
  items: [
    {
      kind: "picker",
      field: SndNoteField.InstNumber,
      x: 1,
      y: 0,
      label: "Inst",
    },
    { kind: "knob", field: SndNoteField.Attack, x: 1, y: 2 },
    { kind: "knob", field: SndNoteField.Decay, x: 3, y: 2 },
    {
      kind: "knob",
      field: SndNoteField.Brilliance,
      x: 5,
      y: 2,
      label: "Brill",
    },
    {
      kind: "knob",
      field: SndNoteField.DynamicRange,
      x: 1,
      y: 5,
      label: "DynRng",
    },
    {
      kind: "knob",
      field: SndNoteField.StereoWidth,
      x: 3,
      y: 5,
      label: "Width",
    },
    {
      kind: "list",
      field: SndNoteField.Variation,
      x: 11,
      y: 0,
      h: VARIATION_H,
    },
    { kind: "list", field: SndNoteField.OutputAssign, x: 13, y: 0, h: 8 },
  ],
};

/** The key's pan, effect sends and level, as the drum kits' mixer strip. */
export const SND_OUTPUT_LAYOUT: SectionLayout<SndNoteField> = levelPanStrip({
  pan: SndNoteField.Pan,
  fx1: SndNoteField.ChorusSend,
  fx2: SndNoteField.ReverbSend,
  level: SndNoteField.Level,
  fields: noteFields,
  h: KEY_H,
});

/** Which family accents each Section's border, by placement id. */
export const SND_FAMILIES: Readonly<Record<string, ParameterFamily>> = {
  tone: ParameterFamily.Pitch,
  output: ParameterFamily.Amp,
};

/** The key's Sections. */
const KEY_ROW: SectionRow = [
  { id: "tone", section: SND_TONE_LAYOUT },
  { id: "output", section: SND_OUTPUT_LAYOUT },
];
const KEYS = rowCells(KEY_ROW);
/** Right edge of the key Sections, in Cells. */
const KEY_W = KEYS.w;
/** The Lane between the key's output and the MFX, in Cells. */
const LANE_W = 2;
const MFX = sectionCells(MFX_PANEL_FRAME);
/** The screen's height: the key Sections or the MFX, whichever is taller. */
const SCREEN_H = Math.max(KEYS.h, MFX.h);

/**
 * The SN Drum editor's Grid: the key's tone and output, and
 * the MFX every key feeds across a Lane for its arrow.
 */
export const SND_SCREEN: ScreenLayout = {
  name: "SN Drum",
  w: KEY_W + LANE_W + MFX.w,
  h: SCREEN_H,
  sections: [
    ...placeRow(KEY_ROW, 0),
    { id: "mfx", section: MFX_PANEL_FRAME, x: KEY_W + LANE_W, y: 0 },
  ],
  lanes: [{ x: KEY_W, y: 0, w: LANE_W, h: SCREEN_H }],
  // The kit's keys all feed the MFX.
  silkscreen: [
    {
      kind: "arrow",
      from: { section: "output", side: "right" },
      to: { section: "mfx", side: "left" },
    },
  ],
};
