import { useLayoutEffect, useState, type RefObject } from "react";
import css from "./SignalArrows.module.css";

/** A signal connection from one box to another, by their `data-node` names. */
export interface Edge {
  from: string;
  to: string;
  /** Drawn where the edge meets its target: + (mix), B (booster), R (ring mod). */
  glyph?: string;
}

/** A box's edges, relative to the overlay. */
export interface Box {
  left: number;
  right: number;
  mid: number;
}

/** Room left of a target for a glyph the arrows meet at. */
const GLYPH_GAP = 16;
const GLYPH_R = 7;

/**
 * The curve of an edge from `from`'s right side to `to`'s left side, or null
 * when the two line up (a straight hop needs no arrow). With a glyph, the
 * curve ends at the glyph, left of the target.
 */
export function arrowPath(from: Box, to: Box, glyph?: boolean): string | null {
  const endX = glyph ? to.left - GLYPH_GAP - GLYPH_R : to.left;
  if (!glyph && Math.abs(from.mid - to.mid) < 2) return null;
  const bend = (from.right + endX) / 2;
  return `M ${from.right} ${from.mid} C ${bend} ${from.mid} ${bend} ${to.mid} ${endX} ${to.mid}`;
}

interface Drawing {
  curves: string[];
  glyphs: { x: number; y: number; text: string; lead: string }[];
}

/**
 * Curved arrows between the boxes of a signal path, over `container`. Boxes
 * are found by their `data-node` attribute; arrows follow layout changes.
 */
export function SignalArrows({ container, edges }: { container: RefObject<HTMLElement | null>; edges: Edge[] }) {
  const [drawing, setDrawing] = useState<Drawing>({ curves: [], glyphs: [] });
  const [size, setSize] = useState({ w: 0, h: 0 });
  const key = JSON.stringify(edges);

  useLayoutEffect(() => {
    const root = container.current;
    if (!root) return;
    const measure = () => {
      const origin = root.getBoundingClientRect();
      const box = (name: string): Box | null => {
        const el = root.querySelector(`[data-node="${name}"]`);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        const left = r.left - origin.left + root.scrollLeft;
        const top = r.top - origin.top + root.scrollTop;
        return { left, right: left + r.width, mid: top + r.height / 2 };
      };
      const next: Drawing = { curves: [], glyphs: [] };
      const glyphed = new Set<string>();
      for (const e of JSON.parse(key) as Edge[]) {
        const from = box(e.from);
        const to = box(e.to);
        if (!from || !to) continue;
        const d = arrowPath(from, to, !!e.glyph);
        if (d) next.curves.push(d);
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
  }, [container, key]);

  return (
    <svg className={css.overlay} width={size.w} height={size.h} aria-hidden="true">
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
