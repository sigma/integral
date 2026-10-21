/** Assertions for Controls drawn on the Grid. */

import { expect } from "vitest";
import { cellsPx, type Cells } from "../synth-ui/grid";

/** `root` is exactly `cells` in size, and nothing inside it is scaled to fit. */
export function expectFillsFootprint(root: HTMLElement, cells: Cells): void {
  const { width, height } = cellsPx(cells);
  expect(root.style.width).toBe(`${width}px`);
  expect(root.style.height).toBe(`${height}px`);
  for (const el of [root, ...root.querySelectorAll<HTMLElement>("*")]) {
    expect(el.style.transform).toBe("");
    expect(el.style.scale).toBe("");
    expect(el.style.zoom).toBe("");
  }
}

/** Sum of the px heights `els` declare inline (or as an SVG `height`). */
export function stackedHeight(els: Element[]): number {
  return els.reduce((sum, el) => {
    const h =
      el instanceof SVGElement
        ? Number(el.getAttribute("height"))
        : parseFloat((el as HTMLElement).style.height);
    expect(Number.isFinite(h)).toBe(true);
    return sum + h;
  }, 0);
}

/** The Control `render` mounted: the container's first element. */
export function rootOf(container: HTMLElement): HTMLElement {
  return container.firstElementChild as HTMLElement;
}
