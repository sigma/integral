/**
 * The SN Synth editor screen: Common on top, one partial row per partial
 * under it, and the MFX beside the rows. Every row's output is summed into
 * the MFX: Screen Silkscreen arrows run from each row's right edge through
 * the Lane between the rows and the MFX, and meet at one `+`.
 *
 * The MFX is `MfxPanel`, a CSS Section around the MFX variant set, hosted in
 * the whole Cells of `MFX_PANEL_FRAME`.
 */

import { sns_partial_count } from "../../../pkg/integral_wasm.js";
import { memo } from "../fields";
import { MFX_PANEL_FRAME } from "../fx/mfx";
import { sectionCells } from "../geometry";
import type { ScreenArrowSilk, ScreenLayout, SectionPlacement } from "../types";
import { SNS_COMMON_LAYOUT } from "./common";
import { SNS_PARTIAL_LAYOUT } from "./partial";

/** Placement ids. */
export const SNS_SECTION = {
  common: "common",
  mfx: "mfx",
  /** The row of `partial` (0-based). */
  partial: (partial: number) => `partial-${partial}`,
} as const;

/** Every partial, 0-based, from core; read on first use, not at import. */
const partials = memo(() => Array.from({ length: sns_partial_count() }, (_, i) => i));
const partialIds = memo(() => new Map(partials().map((p) => [SNS_SECTION.partial(p), p])));

/** The partial a placement id names (0-based), or `undefined` for another Section. */
export function partialOf(id: string): number | undefined {
  return partialIds().get(id);
}

const common = sectionCells(SNS_COMMON_LAYOUT);
const row = sectionCells(SNS_PARTIAL_LAYOUT);
const mfx = sectionCells(MFX_PANEL_FRAME);
/** The Lane column, right of the rows. */
const LANE_X = row.w;

function buildScreen(): ScreenLayout {
  const rows = partials();
  const sections: SectionPlacement[] = [
    { id: SNS_SECTION.common, section: SNS_COMMON_LAYOUT, x: 0, y: 0 },
    ...rows.map((p) => ({ id: SNS_SECTION.partial(p), section: SNS_PARTIAL_LAYOUT, x: 0, y: common.h + p * row.h })),
    { id: SNS_SECTION.mfx, section: MFX_PANEL_FRAME, x: LANE_X + 1, y: common.h },
  ];
  const arrows: ScreenArrowSilk[] = rows.map((p) => ({
    kind: "arrow",
    from: { section: SNS_SECTION.partial(p), side: "right" },
    to: { section: SNS_SECTION.mfx, side: "left" },
    glyph: "+",
  }));
  const h = Math.max(rows.length * row.h, mfx.h);
  return {
    name: "SN Synth",
    w: LANE_X + 1 + mfx.w,
    h: common.h + h,
    sections,
    lanes: [{ x: LANE_X, y: common.h, w: 1, h }],
    silkscreen: arrows,
  };
}

/** The SN Synth editor screen, built on first use: it reads the partial count from core. */
export const snsScreen: () => ScreenLayout = memo(buildScreen);
