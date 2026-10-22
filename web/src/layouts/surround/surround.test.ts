// The Surround screen: the part Sections form a square grid the size of the
// Position Section, right beside it, and the Ext Part's Section beside the grid.
import { describe, expect, it } from "vitest";
import { MixerField } from "../../../pkg/integral_wasm.js";
import { deviceSpec } from "../../deviceSpec";
import { allElements, itemRect, sectionCells } from "../geometry";
import type { SectionPlacement } from "../types";
import { validateScreen, validateSection } from "../validate";
import { SURROUND_IDS, surroundPartIds, surroundScreen } from "./screen";
import { surroundExtLayout, surroundPartLayout } from "./sections";

/** The placement `id` of the Surround screen. */
function placed(id: string): SectionPlacement {
  const p = surroundScreen().sections.find((s) => s.id === id);
  if (!p) throw new Error(`no placement ${id}`);
  return p;
}

/** A placement's rectangle on the screen, in Cells. */
function rect(p: SectionPlacement) {
  return { x: p.x, y: p.y, ...sectionCells(p.section) };
}

describe("Surround screen", () => {
  it("is valid, and its part Sections too", () => {
    expect(validateScreen(surroundScreen())).toEqual([]);
    expect(validateSection(surroundPartLayout())).toEqual([]);
    expect(validateSection(surroundExtLayout())).toEqual([]);
  });

  it("places every part core counts", () => {
    expect(surroundPartIds()).toHaveLength(deviceSpec().part_count);
  });

  it("lays the 16 parts out as 4 rows of 4, in part order", () => {
    const parts = surroundPartIds().map((id) => rect(placed(id)));
    const rows = [...new Set(parts.map((r) => r.y))];
    const cols = [...new Set(parts.map((r) => r.x))];
    expect(rows).toHaveLength(4);
    expect(cols).toHaveLength(4);
    parts.forEach((r, i) => {
      expect(r.y, `part ${i + 1}`).toBe(rows[Math.floor(i / 4)]);
      expect(r.x, `part ${i + 1}`).toBe(cols[i % 4]);
    });
  });

  it("makes the grid the Position Section's size, top-aligned right beside it", () => {
    const pad = rect(placed(SURROUND_IDS.pad));
    const parts = surroundPartIds().map((id) => rect(placed(id)));
    const left = Math.min(...parts.map((r) => r.x));
    const top = Math.min(...parts.map((r) => r.y));
    const right = Math.max(...parts.map((r) => r.x + r.w));
    const bottom = Math.max(...parts.map((r) => r.y + r.h));
    expect({ x: left, y: top, w: right - left, h: bottom - top }).toEqual({
      x: pad.x + pad.w,
      y: pad.y,
      w: pad.w,
      h: pad.h,
    });
  });

  it("puts Ext top-aligned right beside the grid", () => {
    const parts = surroundPartIds().map((id) => rect(placed(id)));
    const ext = rect(placed(SURROUND_IDS.ext));
    expect({ x: ext.x, y: ext.y }).toEqual({
      x: Math.max(...parts.map((r) => r.x + r.w)),
      y: Math.min(...parts.map((r) => r.y)),
    });
  });

  it("keeps the Width and Amb knobs at the top of each part Section, under its title", () => {
    for (const layout of [surroundPartLayout(), surroundExtLayout()]) {
      const knobs = allElements(layout.items).filter((el) => el.kind === "knob");
      expect(knobs.map((k) => k.kind === "knob" && k.field)).toEqual([
        MixerField.SurroundWidth,
        MixerField.SurroundAmbienceSend,
      ]);
      for (const k of knobs) expect(itemRect(k).y).toBe(0);
    }
  });
});
