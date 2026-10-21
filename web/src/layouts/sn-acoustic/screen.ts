/**
 * The SN Acoustic editor screen: COMMON on top; under it INST, VIB, FILTER
 * and AMP in a row, and the MFX beside them across a Lane for the AMP's
 * arrow.
 *
 * The MFX is `MfxPanel`, a CSS Section around the MFX variant set, hosted in
 * the whole Cells of `MFX_PANEL_FRAME`.
 */

import { MFX_PANEL_FRAME } from "../fx/mfx";
import { sectionCells } from "../geometry";
import type { ScreenLayout } from "../types";
import { SNA_AMP_LAYOUT } from "./amp";
import { SNA_COMMON_LAYOUT } from "./common";
import { SNA_FILTER_LAYOUT } from "./filter";
import { SNA_INST_SET } from "./inst";
import { SNA_VIBRATO_LAYOUT } from "./vibrato";

/** Placement ids, which arrows and the editor name. */
export const SNA_SECTION = {
  common: "common",
  inst: "inst",
  vibrato: "vibrato",
  filter: "filter",
  amp: "amp",
  mfx: "mfx",
} as const;

/** Top of the tone row: under COMMON. */
const ROW = sectionCells(SNA_COMMON_LAYOUT).h;
const INST_X = 0;
const VIB_X = INST_X + sectionCells(SNA_INST_SET).w;
const FILTER_X = VIB_X + sectionCells(SNA_VIBRATO_LAYOUT).w;
const AMP_X = FILTER_X + sectionCells(SNA_FILTER_LAYOUT).w;
const LANE_X = AMP_X + sectionCells(SNA_AMP_LAYOUT).w;
/** The Lane between AMP and MFX, in Cells. */
const LANE_W = 2;
const MFX_X = LANE_X + LANE_W;
const MFX = sectionCells(MFX_PANEL_FRAME);
/** The screen's height: the taller of the MFX and COMMON over the tone row. */
const H = Math.max(
  MFX.h,
  ROW +
    Math.max(
      ...[SNA_INST_SET, SNA_VIBRATO_LAYOUT, SNA_FILTER_LAYOUT, SNA_AMP_LAYOUT].map((s) => sectionCells(s).h),
    ),
);

/** The SN-A editor screen. */
export const SNA_SCREEN: ScreenLayout = {
  name: "SN-A editor",
  w: MFX_X + MFX.w,
  h: H,
  sections: [
    { id: SNA_SECTION.common, section: SNA_COMMON_LAYOUT, x: 0, y: 0 },
    { id: SNA_SECTION.inst, section: SNA_INST_SET, x: INST_X, y: ROW },
    { id: SNA_SECTION.vibrato, section: SNA_VIBRATO_LAYOUT, x: VIB_X, y: ROW },
    { id: SNA_SECTION.filter, section: SNA_FILTER_LAYOUT, x: FILTER_X, y: ROW },
    { id: SNA_SECTION.amp, section: SNA_AMP_LAYOUT, x: AMP_X, y: ROW },
    { id: SNA_SECTION.mfx, section: MFX_PANEL_FRAME, x: MFX_X, y: 0 },
  ],
  lanes: [{ x: LANE_X, y: 0, w: LANE_W, h: H }],
  silkscreen: [
    { kind: "arrow", from: { section: SNA_SECTION.amp, side: "right" }, to: { section: SNA_SECTION.mfx, side: "left" } },
  ],
};
