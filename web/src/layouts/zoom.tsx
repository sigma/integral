/**
 * Uniform zoom of a Grid page (#329, #356). The page is laid out at zoom 1
 * and scaled as a whole: zoom fits the page's Grid width into the window
 * width, rounded down to a 5% step so it never overshoots, but never below
 * {@link MIN_ZOOM}: a Grid wider than the window scrolls sideways instead of
 * shrinking. Nothing
 * reflows and there are no breakpoints.
 *
 * One {@link GridZoom} wraps each page that has a Grid, and it is the page
 * root. A Grid screen and any CSS Sections on the same page sit inside that
 * one root, so the whole page scales by the same factor; a GridScreen never
 * zooms itself, so nothing is zoomed twice. CSS `zoom` (not `transform`) is
 * used so the page's scaled size takes part in layout and scrolling. Code
 * that measures with `getBoundingClientRect` inside the page divides by
 * `zoomOf` (`synth-ui/grid.ts`) to get back to layout px.
 *
 * The browser and the plugin share this code path: the plugin renders the
 * same page in its webview. ADR 0004 sizes the plugin window from
 * {@link naturalSize} × zoom, and fitting that window gives the zoom back.
 */

import { useCallback, useLayoutEffect, useState, type ReactNode } from "react";
import { cellsPx, type Cells } from "../synth-ui/grid";

/** Zoom steps per 100%; the rounding grain is its inverse. */
const STEPS = 20;

/** The zoom grain: 5%. */
export const ZOOM_STEP = 1 / STEPS;

/**
 * The smallest zoom a page is fitted to. At zoom 1 Control names are 7px and
 * readouts 9px, the smallest the Grid was drawn for (#329, #354); below it a
 * wide screen such as the PCM Synth editor becomes unreadable on a laptop.
 */
export const MIN_ZOOM = 1;

/** `zoom` rounded to the nearest {@link ZOOM_STEP}, never below one step. */
export function roundZoom(zoom: number): number {
  return Math.max(1, Math.round(zoom * STEPS)) / STEPS;
}

/**
 * The zoom that fits `naturalPx` (the Grid's width at zoom 1) into
 * `availablePx`: rounded down to a 5% step, so the Grid is never wider than
 * the space it is given, and floored at {@link MIN_ZOOM}. It is 1 while
 * either width is unknown (0).
 */
export function fitZoom(availablePx: number, naturalPx: number): number {
  if (availablePx <= 0 || naturalPx <= 0) return 1;
  // The epsilon keeps an exact fit (e.g. 1.25) from flooring a step down on float error.
  const steps = Math.floor((availablePx / naturalPx) * STEPS + 1e-9);
  return Math.max(MIN_ZOOM, steps / STEPS);
}

/** A Grid's size at zoom 1, in Cells and in px. */
export interface NaturalSize {
  /** Size in Cells. */
  cells: Cells;
  /** Size in px at zoom 1 (Cells × `CELL`). */
  px: { width: number; height: number };
}

/**
 * The natural size of a Grid of `grid` Cells, such as a `ScreenLayout`. The
 * plugin sizes its window from it (#365).
 */
export function naturalSize(grid: Cells): NaturalSize {
  const cells = { w: grid.w, h: grid.h };
  return { cells, px: cellsPx(cells) };
}

/**
 * Measures the width the element at `ref` is given and returns the zoom that
 * fits `naturalPx` into it. It re-measures when the element resizes.
 */
export function useFitZoom<E extends HTMLElement>(naturalPx: number): { ref: (el: E | null) => void; zoom: number } {
  const [el, setEl] = useState<E | null>(null);
  const [available, setAvailable] = useState(0);
  const ref = useCallback((node: E | null) => setEl(node), []);

  useLayoutEffect(() => {
    if (!el) return;
    const measure = () => setAvailable(el.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [el]);

  return { ref, zoom: fitZoom(available, naturalPx) };
}

/** Props of {@link GridZoom}. */
export interface GridZoomProps {
  /** The page's Grid size, in Cells; its width is what zoom fits. */
  cells: Cells;
  /** The page, laid out at zoom 1: a Grid screen and any CSS Sections. */
  children: ReactNode;
}

/**
 * A page root that scales its whole content uniformly to fit the width it is
 * given. The outer element takes the available width unscaled and is what is
 * measured; the inner element carries the zoom and is the Grid's width at
 * zoom 1, so CSS Sections lay out in that width whatever the window is.
 * The outer element scrolls sideways when the Grid is wider than it even at
 * {@link MIN_ZOOM}, so the page around it keeps the window's width and
 * nothing is cut off.
 */
export function GridZoom({ cells, children }: GridZoomProps) {
  const { width } = naturalSize(cells).px;
  const { ref, zoom } = useFitZoom<HTMLDivElement>(width);
  return (
    <div ref={ref} style={{ width: "100%", minWidth: 0, overflowX: "auto" }}>
      <div data-grid-zoom={zoom} style={{ zoom, width }}>
        {children}
      </div>
    </div>
  );
}
