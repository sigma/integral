/**
 * The SN Synth editor screen: COMMON on top, one row per partial under it,
 * and the MFX beside the rows. A row is the partial's switch strip, then its
 * OSC, FILTER, AMP, MOD LFO and LFO boxes, abutting. Every row's output is
 * summed into the MFX: Screen Silkscreen arrows run from each row's LFO box
 * through the Lane between the rows and the MFX, and meet at one `+`.
 *
 * The MFX is `MfxPanel`, a CSS Section around the MFX variant set, hosted in
 * the whole Cells of `MFX_PANEL_FRAME`.
 */

import { sns_partial_count } from "../../../pkg/integral_wasm.js";
import { memo } from "../fields";
import { MFX_PANEL_FRAME } from "../fx/mfx";
import { placeRow, rowCells, rowSectionLookup, sectionCells, type SectionRow } from "../geometry";
import type { ScreenArrowSilk, ScreenLayout, SectionPlacement } from "../types";
import { SNS_COMMON_LAYOUT } from "./common";
import { SNS_BOXES, SNS_PARTIAL_SWITCH_LAYOUT, boxOf, type SnsBox } from "./partial";

/** A Section of a partial's row: its switch strip, or one of its boxes. */
export type SnsRowSection = "switch" | SnsBox;

/** Placement ids. */
export const SNS_SECTION = {
  common: "common",
  mfx: "mfx",
  /** Section `section` of the row of `partial` (0-based). */
  partial: (partial: number, section: SnsRowSection) => `partial-${partial}.${section}`,
} as const;

/** Every partial, 0-based, from core; read on first use, not at import. */
const partials = memo(() => Array.from({ length: sns_partial_count() }, (_, i) => i));

/** Every row Section, left to right. */
export const SNS_ROW: readonly SnsRowSection[] = ["switch", ...SNS_BOXES.map((b) => b.id)];

const rowIds = rowSectionLookup(() => partials().length, SNS_ROW, SNS_SECTION.partial);

/** The partial (0-based) and row Section a placement id names, or `undefined` for another Section. */
export function rowSectionOf(id: string): { partial: number; section: SnsRowSection } | undefined {
  const at = rowIds(id);
  return at && { partial: at.row, section: at.section };
}

/** Each row Section's outline. */
const frameOf = (section: SnsRowSection) =>
  section === "switch" ? SNS_PARTIAL_SWITCH_LAYOUT : boxOf(section).layout;

const common = sectionCells(SNS_COMMON_LAYOUT);
const strip = sectionCells(SNS_PARTIAL_SWITCH_LAYOUT);
const mfx = sectionCells(MFX_PANEL_FRAME);
/** Row `partial`'s Sections, the strip then its boxes. */
const rowOf = (partial: number): SectionRow =>
  SNS_ROW.map((s) => ({ id: SNS_SECTION.partial(partial, s), section: frameOf(s) }));

/** A row's width: the strip and its boxes, abutting. */
const ROW_W = rowCells(rowOf(0)).w;
/** A row's height: every row Section is as tall as the strip. */
const ROW_H = strip.h;
/** The Lane column, right of the rows. */
const LANE_X = ROW_W;

/** Row `partial`'s Sections, abutting from the left edge. */
const row = (partial: number): SectionPlacement[] => placeRow(rowOf(partial), common.h + partial * ROW_H);

function buildScreen(): ScreenLayout {
  const rows = partials();
  const last = SNS_ROW[SNS_ROW.length - 1]!;
  const sections: SectionPlacement[] = [
    { id: SNS_SECTION.common, section: SNS_COMMON_LAYOUT, x: 0, y: 0 },
    ...rows.flatMap(row),
    { id: SNS_SECTION.mfx, section: MFX_PANEL_FRAME, x: LANE_X + 1, y: common.h },
  ];
  const arrows: ScreenArrowSilk[] = rows.map((p) => ({
    kind: "arrow",
    from: { section: SNS_SECTION.partial(p, last), side: "right" },
    to: { section: SNS_SECTION.mfx, side: "left" },
    glyph: "+",
  }));
  const h = Math.max(rows.length * ROW_H, mfx.h);
  return {
    name: "SN Synth",
    w: Math.max(LANE_X + 1 + mfx.w, common.w),
    h: common.h + h,
    sections,
    lanes: [{ x: LANE_X, y: common.h, w: 1, h }],
    silkscreen: arrows,
  };
}

/** The SN Synth editor screen, built on first use: it reads the partial count from core. */
export const snsScreen: () => ScreenLayout = memo(buildScreen);
