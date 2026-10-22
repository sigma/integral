/**
 * The Surround page: the common Sections on top, the pad below them, a
 * square grid of part Sections the pad's size beside it, and the Ext Part's
 * Section beside the grid.
 */

import { deviceSpec } from "../../deviceSpec";
import { memo } from "../fields";
import { sectionCells } from "../geometry";
import type { ScreenLayout, SectionPlacement } from "../types";
import {
  SURROUND_AMBIENCE_LAYOUT,
  SURROUND_COMMON_LAYOUT,
  SURROUND_PAD_LAYOUT,
  surroundExtLayout,
  surroundGridSide,
  surroundPartLayout,
} from "./sections";

/** The ids of the part Sections' placements, by 0-based part. */
function partIds(partCount: number): string[] {
  return Array.from({ length: partCount }, (_, i) => `part-${i}`);
}

/** The ids of the screen's single Sections' placements. */
export const SURROUND_IDS = { common: "common", ambience: "ambience", pad: "pad", ext: "ext" } as const;

/**
 * The Surround screen for `partCount` parts: their Sections fill a square
 * grid, row by row, top-aligned right of the pad; the Ext Part's sits right
 * of the grid.
 */
function buildSurroundScreen(partCount: number): ScreenLayout {
  const top = sectionCells(SURROUND_COMMON_LAYOUT);
  const pad = sectionCells(SURROUND_PAD_LAYOUT);
  const side = surroundGridSide(partCount);
  const part = surroundPartLayout();
  const cell = sectionCells(part);
  const parts: SectionPlacement[] = partIds(partCount).map((id, i) => ({
    id,
    section: part,
    x: pad.w + (i % side) * cell.w,
    y: top.h + Math.floor(i / side) * cell.h,
  }));
  const ext = surroundExtLayout();
  const extX = pad.w + side * cell.w;
  return {
    name: "Surround",
    w: Math.max(top.w + sectionCells(SURROUND_AMBIENCE_LAYOUT).w, extX + sectionCells(ext).w),
    h: top.h + Math.max(pad.h, side * cell.h),
    sections: [
      { id: SURROUND_IDS.common, section: SURROUND_COMMON_LAYOUT, x: 0, y: 0 },
      { id: SURROUND_IDS.ambience, section: SURROUND_AMBIENCE_LAYOUT, x: top.w, y: 0 },
      { id: SURROUND_IDS.pad, section: SURROUND_PAD_LAYOUT, x: 0, y: top.h },
      ...parts,
      { id: SURROUND_IDS.ext, section: ext, x: extX, y: top.h },
    ],
    lanes: [],
    silkscreen: [],
  };
}

// Built on first use: they read the device spec from core, which the app
// starts after its modules load.

/** The Surround screen of the device. */
export const surroundScreen: () => ScreenLayout = memo(() => buildSurroundScreen(deviceSpec().part_count));

/** The ids of {@link surroundScreen}'s part Sections, by 0-based part. */
export const surroundPartIds: () => readonly string[] = memo(() => partIds(deviceSpec().part_count));
