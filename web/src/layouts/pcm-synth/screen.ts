/**
 * The PCM Synth editor's screen: COMMON and PARTIAL MIX on top, the two
 * partial pairs under them, one above the other, and MFX beside them,
 * centred on the pairs. Every partial OUTPUT is summed into MFX through a
 * Lane column between them.
 *
 * Each pair is a {@link PCMS_PAIR} region at a fixed origin, so changing one
 * pair's Structure moves nothing outside that pair's footprint.
 */

import { MFX_PANEL_FRAME } from "../fx/mfx";
import { placeRow, rowCells, sectionCells, type SectionRow } from "../geometry";
import { placeRegion } from "../region";
import type { ScreenArrowSilk, ScreenLayout, SectionPlacement } from "../types";
import { PCMS_PAIR, type PairSection } from "./pair";
import { PCMS_COMMON_LAYOUT, PCMS_PMT_LAYOUT } from "./sections";

/** The ids of the screen's single Sections. */
export const PCMS_IDS = { common: "common", pmt: "pmt", mfx: "mfx" } as const;

/** Each pair's id prefix, by pair (0: partials 1 & 2, 1: partials 3 & 4). */
const PAIR_PREFIX = ["p12.", "p34."] as const;

/** The screen id of a pair's Section. */
export function pairSectionId(pair: number, section: PairSection): string {
  return `${PAIR_PREFIX[pair]}${section}`;
}

/** The pair (0 or 1) and the Section in it a screen id names, if it names one. */
export function parsePairSectionId(id: string): { pair: number; section: PairSection } | undefined {
  const pair = PAIR_PREFIX.findIndex((prefix) => id.startsWith(prefix));
  return pair < 0 ? undefined : { pair, section: id.slice(PAIR_PREFIX[pair]!.length) as PairSection };
}

/** The top row: COMMON, then PARTIAL MIX. */
const TOP_ROW: SectionRow = [
  { id: PCMS_IDS.common, section: PCMS_COMMON_LAYOUT },
  { id: PCMS_IDS.pmt, section: PCMS_PMT_LAYOUT },
];

/** Rows of the top row, the screen's first. */
const COMMON_H = rowCells(TOP_ROW).h;

/** Top row of each pair. */
export const PAIR_Y = [COMMON_H, COMMON_H + PCMS_PAIR.h] as const;

/** Rows the pairs take together. */
const PAIRS_H = 2 * PCMS_PAIR.h;

/** MFX's outline in Cells: `MfxPanel`, hosted around the MFX variant set. */
const MFX_CELLS = sectionCells(MFX_PANEL_FRAME);

/** Column of the Lane between the pairs and MFX. */
const MFX_LANE_X = PCMS_PAIR.w;

/** The screen's size in Cells, whatever the Structures. */
export const PCMS_SCREEN_CELLS = {
  w: Math.max(MFX_LANE_X + 1 + MFX_CELLS.w, rowCells(TOP_ROW).w),
  h: COMMON_H + PAIRS_H,
} as const;

/** The screen for pair Structures `structures` (0-based, pair 1 & 2's first). */
export function pcmsScreen(structures: readonly [number, number]): ScreenLayout {
  const sections: SectionPlacement[] = [
    ...placeRow(TOP_ROW, 0),
    { id: PCMS_IDS.mfx, section: MFX_PANEL_FRAME, x: MFX_LANE_X + 1, y: COMMON_H + Math.floor((PAIRS_H - MFX_CELLS.h) / 2) },
  ];
  const lanes = [{ x: MFX_LANE_X, y: COMMON_H, w: 1, h: PAIRS_H }];
  const silkscreen: ScreenArrowSilk[] = [];
  structures.forEach((structure, pair) => {
    const variant = PCMS_PAIR.variants.get(structure);
    if (!variant) return;
    const placed = placeRegion(variant, 0, PAIR_Y[pair]!, PAIR_PREFIX[pair]!);
    sections.push(...placed.sections);
    lanes.push(...placed.lanes);
    silkscreen.push(...placed.silkscreen);
    // Every partial output is summed into the MFX.
    for (const p of placed.sections) {
      if (!p.id.endsWith(".out")) continue;
      silkscreen.push({
        kind: "arrow",
        from: { section: p.id, side: "right" },
        to: { section: PCMS_IDS.mfx, side: "left" },
        glyph: "+",
      });
    }
  });
  return {
    name: `PCM Synth (Structures ${structures[0] + 1}, ${structures[1] + 1})`,
    ...PCMS_SCREEN_CELLS,
    sections,
    lanes,
    silkscreen,
  };
}

/** One screen per Structure Type, both pairs on it, for the validator. */
export function pcmsScreens(): ScreenLayout[] {
  return PCMS_PAIR.keys.map((s) => pcmsScreen([s, s]));
}
