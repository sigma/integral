/**
 * The mixer screen: one 4-Cell column per strip, left to right the parts,
 * the external input, the Drum Comp+EQ units (while the Comp+EQ is on), the
 * Chorus and Reverb strips, and the master. A column stacks its EQ, its
 * strip and its name; the external input has no EQ and leaves those Cells
 * empty, so its strip starts on the same row as the others.
 *
 * The Chorus and Reverb strips and the category plates are CSS Sections hosted in
 * whole-Cell placements. A Chorus or Reverb strip draws its parameters as a
 * Grid Section of its type's variant set (`layouts/fx/strip.ts`), one column
 * wide.
 */

import { MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { deviceSpec } from "../../deviceSpec";
import { memo } from "../fields";
import { sectionCells } from "../geometry";
import type { ScreenLayout, SectionFrame, SectionPlacement } from "../types";
import { EQ_LAYOUT } from "./eq";
import { STRIP_SET } from "./strip";

/** The plate below a strip's fader: its tone's category or its role, under its icon. */
export const NAME_FRAME: SectionFrame = { title: "", titleEdge: "top", w: 4, h: 2 };

/** The width of every column, in Cells. */
const COLUMN = 4;

const EQ_H = sectionCells(EQ_LAYOUT).h;
const STRIP_H = sectionCells(STRIP_SET).h;
const NAME_H = sectionCells(NAME_FRAME).h;
/** A column's height: EQ, strip and name. */
const HEIGHT = EQ_H + STRIP_H + NAME_H;

/** A Chorus or Reverb strip: a CSS Section filling a whole column, its unused title row included, around its Grid parameter Section. */
export const FX_FRAME: SectionFrame = { title: "", titleEdge: "top", w: COLUMN, h: HEIGHT - 1 };

/** What a placement on the mixer screen draws. */
export type MixerSlot =
  /** A strip's EQ Section, in its Scope. */
  | { role: "eq"; kind: MixerScopeKind; index: number }
  /** A strip, in its Scope. */
  | { role: "strip"; kind: MixerScopeKind; index: number }
  /** The plate under a strip: its category or role. */
  | { role: "name"; kind: MixerScopeKind; index: number }
  /** The Chorus or Reverb strip. */
  | { role: "fx"; fx: "chorus" | "reverb" };

/** A screen layout and what each of its placements draws, by placement id. */
export interface MixerScreen {
  /** The layout. */
  layout: ScreenLayout;
  /** What each placement draws. */
  slots: ReadonlyMap<string, MixerSlot>;
}

/** Builds the mixer screen, with or without the Drum Comp+EQ unit strips. */
function buildMixerScreen(compEq: boolean): MixerScreen {
  const spec = deviceSpec();
  const sections: SectionPlacement[] = [];
  const slots = new Map<string, MixerSlot>();
  let x = 0;
  const place = (id: string, section: SectionFrame, y: number, slot: MixerSlot) => {
    sections.push({ id, section, x, y });
    slots.set(id, slot);
  };
  const column = (kind: MixerScopeKind, index: number, id: string, eq: boolean) => {
    if (eq) place(`eq.${id}`, EQ_LAYOUT, 0, { role: "eq", kind, index });
    place(`strip.${id}`, STRIP_SET, EQ_H, { role: "strip", kind, index });
    place(`name.${id}`, NAME_FRAME, EQ_H + STRIP_H, { role: "name", kind, index });
    x += COLUMN;
  };
  for (let i = 0; i < spec.part_count; i++) column(MixerScopeKind.Part, i, `part.${i}`, true);
  column(MixerScopeKind.Ext, 0, "ext", false);
  if (compEq) {
    for (let u = 0; u < spec.comp_eq_unit_count; u++) column(MixerScopeKind.CompEqUnit, u, `comp.${u}`, true);
  }
  for (const fx of ["chorus", "reverb"] as const) {
    place(fx, FX_FRAME, 0, { role: "fx", fx });
    x += COLUMN;
  }
  column(MixerScopeKind.Master, 0, "master", true);
  return {
    layout: { name: compEq ? "mixer with Comp+EQ" : "mixer", w: x, h: HEIGHT, sections, lanes: [], silkscreen: [] },
    slots,
  };
}

const plain = memo(() => buildMixerScreen(false));
const withCompEq = memo(() => buildMixerScreen(true));

/**
 * The mixer screen: with `compEq`, a strip per Drum Comp+EQ unit after the
 * external input. Built on first use, since it reads the device spec from
 * core, which the app starts after its modules load.
 */
export function mixerScreen(compEq: boolean): MixerScreen {
  return compEq ? withCompEq() : plain();
}
