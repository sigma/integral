/**
 * Draws routed signal arrows (`routing.ts`) into an enclosing `<svg>`, in px at
 * zoom 1. Used by both Silkscreen layers: `GridSection` for arrows inside a
 * Section, `GridScreen` for arrows between Sections.
 */

import { useId } from "react";
import { CELL } from "../synth-ui/grid";
import { glyphKey, type DrawnArrow } from "./routing";
import type { ArrowGlyph, HalfCellPoint } from "./types";
import css from "./SilkArrows.module.css";


/** Props of {@link SilkArrows}. */
export interface SilkArrowsProps {
  /** What to draw. */
  arrows: readonly DrawnArrow[];
  /** Where Cell (0, 0) is, px. */
  ox?: number;
  /** Where Cell (0, 0) is, px. */
  oy?: number;
  /**
   * How far past each route's ends to draw, px: a Section's border sits this
   * far inside its Cells, so a screen arrow reaches it.
   */
  inset?: number;
}

/** A glyph's ring radius, px; it fits inside one Cell. */
const GLYPH_R = 7;

type Px = readonly [number, number];

/** The unit direction from `a` to `b` (routes only run straight across or down). */
function heading(a: Px, b: Px): Px {
  return [Math.sign(b[0] - a[0]), Math.sign(b[1] - a[1])];
}

/** `p` moved `d` px along `dir`. */
const along = (p: Px, dir: Px, d: number): Px => [p[0] + dir[0] * d, p[1] + dir[1] * d];

/** A path through `points`, dropping repeats. */
function pathOf(points: readonly Px[]): string {
  const kept = points.filter((p, i) => i === 0 || p[0] !== points[i - 1]![0] || p[1] !== points[i - 1]![1]);
  return kept.map(([x, y], i) => `${i === 0 ? "M" : "L"} ${x} ${y}`).join(" ");
}

/**
 * Signal arrows, each with its arrowhead; arrows into one target through one
 * glyph meet at it.
 */
export function SilkArrows({ arrows, ox = 0, oy = 0, inset = 0 }: SilkArrowsProps) {
  // One marker per drawing, so no two SVGs on a page share an id.
  const marker = `silk-arrow-${useId().replace(/[^a-zA-Z0-9]/g, "")}`;
  const px = ([x, y]: HalfCellPoint): Px => [ox + x * CELL, oy + y * CELL];
  const bodies: string[] = [];
  const glyphs = new Map<string, { at: Px; text: ArrowGlyph; lead: string }>();

  for (const arrow of arrows) {
    const { route, glyph } = arrow;
    const pts = route.points.map(px);
    const n = pts.length;
    // Each end reaches past its Cell edge by `inset`, along its own segment.
    const start = along(pts[0]!, heading(pts[1]!, pts[0]!), inset);
    const enter = heading(pts[n - 2]!, pts[n - 1]!);
    const end = along(pts[n - 1]!, enter, inset);
    if (glyph === undefined) {
      bodies.push(pathOf([start, ...pts.slice(1, -1), end]));
      continue;
    }
    // The route stops at the glyph's ring, and a lead runs from it into the target.
    const at = px(route.glyphAt);
    // The glyph's Cell lies on the last segment, or is its first corner.
    const before = [start, ...pts.slice(1, -1)];
    const corner = before[before.length - 1]!;
    if (before.length > 1 && corner[0] === at[0] && corner[1] === at[1]) before.pop();
    bodies.push(pathOf([...before, along(at, heading(at, before[before.length - 1]!), GLYPH_R)]));
    const key = glyphKey(arrow);
    if (!glyphs.has(key)) glyphs.set(key, { at, text: glyph, lead: pathOf([along(at, enter, GLYPH_R), end]) });
  }

  const head = `url(#${marker})`;
  return (
    <g className={css.arrows}>
      <defs>
        <marker id={marker} viewBox="0 0 6 6" refX="5" refY="3" markerWidth="6" markerHeight="6" orient="auto">
          <path className={css.head} d="M 0 0 L 6 3 L 0 6 z" />
        </marker>
      </defs>
      {bodies.map((d, i) => (
        <path key={i} data-arrow className={css.line} d={d} markerEnd={head} />
      ))}
      {[...glyphs].map(([key, { at: [x, y], text, lead }]) => (
        <g key={key}>
          <path data-arrow className={css.line} d={lead} markerEnd={head} />
          <circle className={css.ring} cx={x} cy={y} r={GLYPH_R} />
          <text data-glyph className={css.glyph} x={x} y={y}>
            {text}
          </text>
        </g>
      ))}
    </g>
  );
}
