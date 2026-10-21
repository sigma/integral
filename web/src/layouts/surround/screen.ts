/**
 * The Surround page: the common Sections on top, the pad below them, and a
 * block of part Sections (16 parts, then the Ext Part) beside the pad.
 */

import { deviceSpec } from "../../deviceSpec";
import { memo } from "../fields";
import { sectionCells } from "../geometry";
import type { ScreenLayout, SectionPlacement } from "../types";
import {
  SURROUND_AMBIENCE_LAYOUT,
  SURROUND_COMMON_LAYOUT,
  SURROUND_EXT_LAYOUT,
  SURROUND_PAD_LAYOUT,
  SURROUND_PART_LAYOUT,
} from "./sections";

/** Part Sections per row of the block. */
const PARTS_PER_ROW = 6;

/** The ids of the part Sections' placements, by 0-based part. */
function partIds(partCount: number): string[] {
  return Array.from({ length: partCount }, (_, i) => `part-${i}`);
}

/** The ids of the screen's single Sections' placements. */
export const SURROUND_IDS = { common: "common", ambience: "ambience", pad: "pad", ext: "ext" } as const;

/**
 * The Surround screen for `partCount` parts: the part Sections fill rows of
 * `PARTS_PER_ROW` beside the pad, the Ext Part's last.
 */
function buildSurroundScreen(partCount: number): ScreenLayout {
  const top = sectionCells(SURROUND_COMMON_LAYOUT);
  const pad = sectionCells(SURROUND_PAD_LAYOUT);
  const cell = sectionCells(SURROUND_PART_LAYOUT);
  const slot = (i: number) => ({
    x: pad.w + (i % PARTS_PER_ROW) * cell.w,
    y: top.h + Math.floor(i / PARTS_PER_ROW) * cell.h,
  });
  const parts: SectionPlacement[] = partIds(partCount).map((id, i) => ({
    id,
    section: SURROUND_PART_LAYOUT,
    ...slot(i),
  }));
  const rows = Math.ceil((partCount + 1) / PARTS_PER_ROW);
  return {
    name: "Surround",
    w: pad.w + PARTS_PER_ROW * cell.w,
    h: top.h + Math.max(pad.h, rows * cell.h),
    sections: [
      { id: SURROUND_IDS.common, section: SURROUND_COMMON_LAYOUT, x: 0, y: 0 },
      { id: SURROUND_IDS.ambience, section: SURROUND_AMBIENCE_LAYOUT, x: top.w, y: 0 },
      { id: SURROUND_IDS.pad, section: SURROUND_PAD_LAYOUT, x: 0, y: top.h },
      ...parts,
      { id: SURROUND_IDS.ext, section: SURROUND_EXT_LAYOUT, ...slot(partCount) },
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
