import { useLayoutEffect, useRef, useState } from "react";
import css from "./SignalArrows.module.css";

/** A signal connection from one box to another, by their `data-node` names. */
export interface Edge {
  from: string;
  to: string;
  /** Drawn where the edge meets its target: + (mix), B (booster), R (ring mod). */
  glyph?: string;
  /** A box the edge must not cross: it passes under it (over it, coming from above). */
  around?: string;
}

/** A box's edges, relative to the overlay. */
export interface Box {
  left: number;
  right: number;
  top: number;
  mid: number;
  bottom: number;
}

/** Clearance under a box an edge goes around. */
const AROUND_GAP = 10;

/** Room left of a target for a glyph the arrows meet at. */
const GLYPH_GAP = 16;
const GLYPH_R = 7;

/**
 * The curve of an edge from `from`'s right side to `to`'s left side (straight
 * when they line up). With a glyph, the curve ends at the glyph, left of the
 * target. With `around`, it passes just under that box (just over it, when
 * coming from above it): an S-curve to the box's bottom right corner, so it
 * clears the whole box, then another into the target.
 */
export function arrowPath(from: Box, to: Box, glyph?: boolean, around?: Box): string {
  const endX = glyph ? to.left - GLYPH_GAP - GLYPH_R : to.left;
  const s = (x0: number, y0: number, x1: number, y1: number) => {
    const bend = (x0 + x1) / 2;
    return ` C ${bend} ${y0} ${bend} ${y1} ${x1} ${y1}`;
  };
  if (around) {
    const wx = around.right;
    const wy = from.mid > around.mid
      ? Math.min(from.mid, around.bottom + AROUND_GAP)
      : Math.max(from.mid, around.top - AROUND_GAP);
    return `M ${from.right} ${from.mid}${s(from.right, from.mid, wx, wy)}${s(wx, wy, endX, to.mid)}`;
  }
  return `M ${from.right} ${from.mid}${s(from.right, from.mid, endX, to.mid)}`;
}

interface Drawing {
  curves: string[];
  glyphs: { x: number; y: number; text: string; lead: string }[];
}

/**
 * Curved arrows between the boxes of a signal path, drawn over its parent
 * element (which must be positioned). Boxes are found inside the parent by
 * their `data-node` attribute; arrows follow layout changes.
 */
export function SignalArrows({ edges }: { edges: Edge[] }) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [drawing, setDrawing] = useState<Drawing>({ curves: [], glyphs: [] });
  const [size, setSize] = useState({ w: 0, h: 0 });
  const key = JSON.stringify(edges);

  useLayoutEffect(() => {
    // The parent, not a ref passed down: a parent's ref isn't attached yet when
    // its children's layout effects run.
    const root = svgRef.current?.parentElement;
    if (!root) return;
    const measure = () => {
      const origin = root.getBoundingClientRect();
      const box = (name: string): Box | null => {
        const el = root.querySelector(`[data-node="${name}"]`);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        const left = r.left - origin.left + root.scrollLeft;
        const top = r.top - origin.top + root.scrollTop;
        return { left, right: left + r.width, top, mid: top + r.height / 2, bottom: top + r.height };
      };
      const next: Drawing = { curves: [], glyphs: [] };
      const glyphed = new Set<string>();
      for (const e of JSON.parse(key) as Edge[]) {
        const from = box(e.from);
        const to = box(e.to);
        if (!from || !to) continue;
        next.curves.push(arrowPath(from, to, !!e.glyph, (e.around && box(e.around)) || undefined));
        if (e.glyph && !glyphed.has(e.to)) {
          glyphed.add(e.to);
          const x = to.left - GLYPH_GAP;
          next.glyphs.push({ x, y: to.mid, text: e.glyph, lead: `M ${x + GLYPH_R} ${to.mid} H ${to.left}` });
        }
      }
      setDrawing((prev) => (JSON.stringify(prev) === JSON.stringify(next) ? prev : next));
      setSize((prev) =>
        prev.w === root.scrollWidth && prev.h === root.scrollHeight ? prev : { w: root.scrollWidth, h: root.scrollHeight });
    };
    measure();
    root.addEventListener("scroll", measure, true);
    // jsdom (tests) has no ResizeObserver; the first measure is all it needs.
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(root);
    root.querySelectorAll("[data-node]").forEach((el) => observer?.observe(el));
    return () => {
      observer?.disconnect();
      root.removeEventListener("scroll", measure, true);
    };
  }, [key]);

  return (
    <svg ref={svgRef} className={css.overlay} width={size.w} height={size.h} aria-hidden="true">
      <defs>
        <marker id="signal-arrow" viewBox="0 0 6 6" refX="5" refY="3" markerWidth="6" markerHeight="6" orient="auto">
          <path d="M 0 0 L 6 3 L 0 6 z" fill="#888" />
        </marker>
      </defs>
      {drawing.curves.map((d, i) => (
        <path key={i} d={d} fill="none" stroke="#888" strokeWidth="1.5" markerEnd="url(#signal-arrow)" />
      ))}
      {drawing.glyphs.map((g, i) => (
        <g key={`g${i}`}>
          <path d={g.lead} stroke="#888" strokeWidth="1.5" markerEnd="url(#signal-arrow)" />
          <circle cx={g.x} cy={g.y} r={GLYPH_R} fill="#1a1a30" stroke="#888" />
          <text x={g.x} y={g.y + 3.5} textAnchor="middle" fontSize="10" fontWeight="700" fill="#ccc">{g.text}</text>
        </g>
      ))}
    </svg>
  );
}
