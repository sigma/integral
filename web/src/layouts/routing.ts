/**
 * Signal arrow routing (#331, #355). A route is computed from the layout, never
 * measured from the DOM, so it survives variant sets and Scope changes.
 *
 * An arrow leaves the centre of the middle Cell along its `from` side (the
 * upper or left middle, on an even side), runs through Cell centres of free
 * Cells, and enters its `to` side the same way. Between Sections the free
 * Cells are the screen's Lanes; inside a Section they are the Cells no item
 * covers. Of all routes, it takes the one with the fewest bends, then the
 * shortest, then the first found in a fixed search order: the same layout
 * always draws the same route.
 */

import { allElements, itemRect, sectionCells } from "./geometry";
import type { FieldKey } from "./fields";
import type {
  ArrowGlyph,
  CellRect,
  ElementLayout,
  HalfCellPoint,
  ItemEndpoint,
  ScreenArrowSilk,
  ScreenLayout,
  SectionArrowSilk,
  SectionLayout,
  Side,
} from "./types";

/** The rectangle an arrow leaves or enters, and which edge of it. */
export interface Anchor {
  /** In Cells. */
  rect: CellRect;
  /** Which edge. */
  side: Side;
}

/** A computed arrow, in Cells from its container's origin. */
export interface Route {
  /**
   * Its corners, from the `from` edge to the `to` edge, on the half-Cell
   * lattice; each segment is horizontal or vertical.
   */
  points: HalfCellPoint[];
  /** Centre of the free Cell next to the `to` edge: where a glyph sits. */
  glyphAt: HalfCellPoint;
}

/** A Cell, `[x, y]` in Cells. */
type Cell = readonly [number, number];

/** Whether the Cell at `(x, y)` is free for an arrow to run through. */
export type Free = (x: number, y: number) => boolean;

/** The unit step out of each side. */
const OUT: Record<Side, Cell> = { top: [0, -1], right: [1, 0], bottom: [0, 1], left: [-1, 0] };

/** The search order of steps, fixed so ties always break the same way. */
const STEPS: readonly Side[] = ["right", "bottom", "left", "top"];

/** Where an arrow meets `anchor`'s side, and the Cell just outside it there. */
export function sidePoint({ rect: r, side }: Anchor): { point: HalfCellPoint; cell: Cell } {
  const across = Math.floor(((side === "top" || side === "bottom" ? r.w : r.h) - 1) / 2);
  switch (side) {
    case "top":
      return { point: [r.x + across + 0.5, r.y], cell: [r.x + across, r.y - 1] };
    case "bottom":
      return { point: [r.x + across + 0.5, r.y + r.h], cell: [r.x + across, r.y + r.h] };
    case "left":
      return { point: [r.x, r.y + across + 0.5], cell: [r.x - 1, r.y + across] };
    case "right":
      return { point: [r.x + r.w, r.y + across + 0.5], cell: [r.x + r.w, r.y + across] };
  }
}

/** A search state: a Cell, the way the route moved into it, and its cost. */
interface State {
  cell: Cell;
  heading: Side;
  bends: number;
  length: number;
  /** Insertion order: the last tie-break. */
  seq: number;
  parent: State | null;
}

/** `a` is cheaper than `b`: fewer bends, then shorter, then found first. */
const cheaper = (a: State, b: State): boolean =>
  a.bends !== b.bends ? a.bends < b.bends : a.length !== b.length ? a.length < b.length : a.seq < b.seq;

/** A binary min-heap of states by {@link cheaper}. */
class Queue {
  private readonly heap: State[] = [];

  get size(): number {
    return this.heap.length;
  }

  push(s: State): void {
    const h = this.heap;
    h.push(s);
    let i = h.length - 1;
    while (i > 0) {
      const up = (i - 1) >> 1;
      if (!cheaper(h[i]!, h[up]!)) break;
      [h[i], h[up]] = [h[up]!, h[i]!];
      i = up;
    }
  }

  pop(): State {
    const h = this.heap;
    const top = h[0]!;
    const last = h.pop()!;
    if (h.length > 0) {
      h[0] = last;
      let i = 0;
      for (;;) {
        const l = 2 * i + 1;
        const r = l + 1;
        let min = i;
        if (l < h.length && cheaper(h[l]!, h[min]!)) min = l;
        if (r < h.length && cheaper(h[r]!, h[min]!)) min = r;
        if (min === i) break;
        [h[i], h[min]] = [h[min]!, h[i]!];
        i = min;
      }
    }
    return top;
  }
}

/** The side opposite `side`: the way a route moves to enter it. */
const OPPOSITE: Record<Side, Side> = { top: "bottom", right: "left", bottom: "top", left: "right" };

/** Drops the points where a route runs straight on. */
function corners(points: HalfCellPoint[]): HalfCellPoint[] {
  return points.filter((p, i) => {
    const a = points[i - 1];
    const b = points[i + 1];
    if (!a || !b) return true;
    return !((a[0] === p[0] && p[0] === b[0]) || (a[1] === p[1] && p[1] === b[1]));
  });
}

/**
 * The route from `from` to `to` through free Cells, or `null` when there is
 * none: the Cell outside either side isn't free, or no run of free Cells
 * joins them. Moving into the `to` side counts as a step, so entering it
 * crosswise counts as a bend.
 */
export function route(free: Free, from: Anchor, to: Anchor): Route | null {
  const start = sidePoint(from);
  const end = sidePoint(to);
  const isFree = ([x, y]: Cell) => free(x, y);
  if (!isFree(start.cell) || !isFree(end.cell)) return null;
  const entering = OPPOSITE[to.side];
  const key = ([x, y]: Cell, heading: Side) => `${x},${y},${heading}`;
  const done = new Set<string>();
  const queue = new Queue();
  let seq = 0;
  queue.push({ cell: start.cell, heading: from.side, bends: 0, length: 0, seq: seq++, parent: null });
  let found: State | null = null;
  while (queue.size > 0) {
    const s = queue.pop();
    const k = key(s.cell, s.heading);
    if (done.has(k)) continue;
    done.add(k);
    // Every later state has at least as many bends, and entering only adds.
    if (found !== null && s.bends > found.bends) break;
    const [x, y] = s.cell;
    if (x === end.cell[0] && y === end.cell[1]) {
      // Entering the target is one more step, so a crosswise entry is a bend.
      const bends = s.bends + (s.heading === entering ? 0 : 1);
      const best = { ...s, bends };
      if (found === null || cheaper(best, found)) found = best;
    }
    for (const heading of STEPS) {
      if (heading === OPPOSITE[s.heading]) continue;
      const [dx, dy] = OUT[heading];
      const next: Cell = [x + dx, y + dy];
      if (!isFree(next) || done.has(key(next, heading))) continue;
      const bends = s.bends + (heading === s.heading ? 0 : 1);
      queue.push({ cell: next, heading, bends, length: s.length + 1, seq: seq++, parent: s });
    }
  }
  if (found === null) return null;
  const centres: HalfCellPoint[] = [];
  for (let s: State | null = found; s; s = s.parent) centres.push([s.cell[0] + 0.5, s.cell[1] + 0.5]);
  centres.reverse();
  const glyphAt = centres[centres.length - 1]!;
  return { points: corners([start.point, ...centres, end.point]), glyphAt };
}

/** Whether `(x, y)` is a Cell of `r`. */
const within = (r: CellRect, x: number, y: number): boolean => x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;

/**
 * The route of an arrow between two Sections of `screen`, through its Lanes,
 * in screen Cells; `null` when either Section is unknown or no Lanes join them.
 */
export function routeScreenArrow(screen: ScreenLayout, arrow: ScreenArrowSilk): Route | null {
  const anchor = ({ section, side }: ScreenArrowSilk["from"]): Anchor | null => {
    const p = screen.sections.find((s) => s.id === section);
    return p ? { rect: { x: p.x, y: p.y, ...sectionCells(p.section) }, side } : null;
  };
  const from = anchor(arrow.from);
  const to = anchor(arrow.to);
  if (!from || !to) return null;
  return route((x, y) => screen.lanes.some((lane) => within(lane, x, y)), from, to);
}

/** The Cells free for arrows inside `layout`: in its content, under no item. */
export function sectionFree<F extends FieldKey>(layout: SectionLayout<F>): Free {
  const content: CellRect = { x: 0, y: 0, w: layout.w, h: layout.h };
  const covered = layout.items.map(itemRect);
  return (x, y) => within(content, x, y) && !covered.some((r) => within(r, x, y));
}

/** Whether `el` is what `end` names. */
export function isEndpoint<F extends FieldKey>(el: ElementLayout<F>, end: ItemEndpoint<F>): boolean {
  return "field" in end
    ? el.kind !== "display" && el.field === end.field && el.index === end.index
    : el.kind === "display" && el.display === end.display;
}

/**
 * The route of an arrow inside one Section, through Cells no item covers, in
 * the Section's content Cells. Endpoints are looked up among `elements` (by
 * default every element, every alternative included); `null` when either is
 * missing there or no free Cells join them.
 */
export function routeSectionArrow<F extends FieldKey>(
  layout: SectionLayout<F>,
  arrow: SectionArrowSilk<F>,
  elements: readonly ElementLayout<F>[] = allElements(layout.items),
): Route | null {
  const anchor = (end: ItemEndpoint<F>): Anchor | null => {
    const el = elements.find((e) => isEndpoint(e, end));
    return el ? { rect: itemRect(el), side: end.side } : null;
  };
  const from = anchor(arrow.from);
  const to = anchor(arrow.to);
  if (!from || !to) return null;
  return route(sectionFree(layout), from, to);
}

/** A routed arrow and the glyph it meets its target through, if any. */
export interface DrawnArrow {
  /** Its route, in Cells from the drawing's origin. */
  route: Route;
  /** Drawn centred in the Cell next to the target. */
  glyph?: ArrowGlyph;
}

/** `arrows` with their routes, dropping those `routeOf` can't route. */
export function drawnArrows<A extends { glyph?: ArrowGlyph }>(
  arrows: readonly A[],
  routeOf: (arrow: A) => Route | null,
): DrawnArrow[] {
  return arrows.flatMap((a) => {
    const route = routeOf(a);
    return route ? [{ route, glyph: a.glyph }] : [];
  });
}

/** Where a glyph sits, as `"x,y"`. */
const glyphCell = ({ route }: DrawnArrow): string => route.glyphAt.join(",");

/**
 * What one glyph draws: its Cell, its target's edge point and its text. Arrows
 * with one key share one glyph.
 */
export function glyphKey(arrow: DrawnArrow): string {
  return `${glyphCell(arrow)} ${arrow.route.points[arrow.route.points.length - 1]!.join(",")} ${arrow.glyph}`;
}

/**
 * The Cells (`"x,y"`) where glyphed arrows would draw two different glyphs:
 * a different text, or arrows into different targets meeting in one Cell.
 */
export function glyphClashes(arrows: readonly DrawnArrow[]): string[] {
  const byCell = new Map<string, string>();
  const clashes = new Set<string>();
  for (const a of arrows) {
    if (a.glyph === undefined) continue;
    const cell = glyphCell(a);
    const seen = byCell.get(cell);
    if (seen === undefined) byCell.set(cell, glyphKey(a));
    else if (seen !== glyphKey(a)) clashes.add(cell);
  }
  return [...clashes];
}
