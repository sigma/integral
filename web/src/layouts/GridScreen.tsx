/**
 * Places a screen's Sections on the Grid at zoom 1. Sections go into the DOM
 * in reading order (top edge, then left edge), so focus runs Section by
 * Section in screen order; each Section orders its own elements row-major.
 */

import type { CSSProperties, ReactNode } from "react";
import { CELL, cellsPx } from "../synth-ui/grid";
import { readingOrder, sectionCells } from "./geometry";
import type { ScreenLayout, SectionPlacement } from "./types";

/** Props of {@link GridScreen}. */
export interface GridScreenProps {
  /** What to place. */
  layout: ScreenLayout;
  /** Draws one placed Section (usually a `GridSection` with its Scope's binding). */
  children: (placement: SectionPlacement) => ReactNode;
  /** Extra style for the screen's root. */
  style?: CSSProperties;
}

/** A screen of Sections on one Grid. */
export function GridScreen({ layout, children, style }: GridScreenProps) {
  const placements = [...layout.sections].sort(readingOrder);
  return (
    <div
      data-grid-screen={layout.name}
      style={{ position: "relative", ...cellsPx({ w: layout.w, h: layout.h }), ...style }}
    >
      {placements.map((p) => (
        <div
          key={p.id}
          data-section={p.id}
          style={{ position: "absolute", left: p.x * CELL, top: p.y * CELL, ...cellsPx(sectionCells(p.section)) }}
        >
          {children(p)}
        </div>
      ))}
    </div>
  );
}
