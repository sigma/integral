/**
 * Screen regions (`RegionVariantSet`): placing one on a screen, and the Lanes
 * a region leaves free.
 */

import { sectionCells } from "./geometry";
import type { Lane, ScreenRegion, SectionPlacement } from "./types";

/**
 * Lanes over every Cell of a `w`×`h` footprint that no Section covers, so
 * arrows may run anywhere the Sections leave room: one rectangle per run of
 * free Cells along a row, grown down while the rows below free the same run.
 */
export function freeLanes(w: number, h: number, sections: readonly SectionPlacement[]): Lane[] {
  const covered = (x: number, y: number) =>
    sections.some((p) => {
      const { w: pw, h: ph } = sectionCells(p.section);
      return x >= p.x && y >= p.y && x < p.x + pw && y < p.y + ph;
    });
  const lanes: Lane[] = [];
  /** The lane each run (`"x,w"`) of the previous row grows, if any. */
  let open = new Map<string, Lane>();
  for (let y = 0; y < h; y++) {
    const next = new Map<string, Lane>();
    for (let x = 0; x < w; ) {
      if (covered(x, y)) {
        x++;
        continue;
      }
      let end = x;
      while (end < w && !covered(end, y)) end++;
      const key = `${x},${end - x}`;
      let lane = open.get(key);
      if (lane) lane.h++;
      else {
        lane = { x, y, w: end - x, h: 1 };
        lanes.push(lane);
      }
      next.set(key, lane);
      x = end;
    }
    open = next;
  }
  return lanes;
}

/** `region` with its origin at `(x, y)` and every Section id prefixed with `prefix`. */
export function placeRegion(region: ScreenRegion, x: number, y: number, prefix: string): ScreenRegion {
  return {
    sections: region.sections.map((p) => ({ ...p, id: prefix + p.id, x: p.x + x, y: p.y + y })),
    lanes: region.lanes.map((l) => ({ ...l, x: l.x + x, y: l.y + y })),
    silkscreen: region.silkscreen.map((a) => ({
      ...a,
      from: { ...a.from, section: prefix + a.from.section },
      to: { ...a.to, section: prefix + a.to.section },
    })),
  };
}
