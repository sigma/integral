// Uniform zoom of a Grid page (#356): fit the window width, in 5% steps.
import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CELL, cellsPx, zoomOf } from "../synth-ui/grid";
import { cleanup, render } from "../test/dom";
import { GridZoom, MIN_ZOOM, ZOOM_STEP, fitZoom, naturalSize, roundZoom } from "./zoom";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

/** jsdom has no ResizeObserver: this one records its callbacks so a test can fire them. */
function stubResizeObserver(): { resize: () => void } {
  const callbacks: (() => void)[] = [];
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: () => void) {
        callbacks.push(callback);
      }
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    },
  );
  return { resize: () => callbacks.forEach((c) => c()) };
}

describe("roundZoom", () => {
  it("rounds to the nearest 5%", () => {
    expect(ZOOM_STEP).toBe(0.05);
    expect(roundZoom(1)).toBe(1);
    expect(roundZoom(1.024)).toBe(1);
    expect(roundZoom(1.026)).toBe(1.05);
    expect(roundZoom(0.874)).toBe(0.85);
    expect(roundZoom(0.876)).toBe(0.9);
    expect(roundZoom(1.7)).toBe(1.7);
  });
  it("gives exact steps, not float drift", () => {
    expect(roundZoom(0.7 + 0.000001)).toBe(0.7);
    expect(roundZoom(1.15)).toBe(1.15);
  });
  it("never goes below one step", () => {
    expect(roundZoom(0.01)).toBe(ZOOM_STEP);
    expect(roundZoom(0)).toBe(ZOOM_STEP);
  });
});

describe("fitZoom", () => {
  it("fits the available width, rounded down to a 5% step so it never overshoots", () => {
    expect(fitZoom(1000, 1000)).toBe(1);
    expect(fitZoom(1500, 1000)).toBe(1.5);
    expect(fitZoom(1250, 1000)).toBe(1.25);
    expect(fitZoom(1249, 1000)).toBe(1.2);
    for (const available of [1001, 1234, 1399, 1440, 1999]) {
      expect(fitZoom(available, 1000) * 1000).toBeLessThanOrEqual(available);
    }
  });
  it("never shrinks below MIN_ZOOM: a wider Grid scrolls instead", () => {
    expect(MIN_ZOOM).toBe(1);
    expect(fitZoom(640, 960)).toBe(MIN_ZOOM);
    // The PCM Synth editor (148 Cells, 3552px) in a 1440px laptop window.
    expect(fitZoom(1440, 3552)).toBe(MIN_ZOOM);
  });
  it("stays at 1 until there is a width to fit", () => {
    expect(fitZoom(0, 960)).toBe(1);
    expect(fitZoom(960, 0)).toBe(1);
  });
});

describe("naturalSize", () => {
  it("is the Grid in Cells and in px at zoom 1", () => {
    expect(naturalSize({ w: 40, h: 12 })).toEqual({
      cells: { w: 40, h: 12 },
      px: { width: 40 * CELL, height: 12 * CELL },
    });
  });
});

describe("GridZoom", () => {
  const PAGE = { w: 40, h: 10 };
  const PAGE_PX = cellsPx(PAGE).width;

  /** Mounts a 40-Cell-wide page in a window `width` px wide. */
  function mount(width: { px: number }) {
    vi.spyOn(Element.prototype, "clientWidth", "get").mockImplementation(() => width.px);
    const { container } = render(
      <GridZoom cells={PAGE}>
        <div data-testid="grid" style={{ width: PAGE_PX }} />
        <div data-testid="css-section" />
      </GridZoom>,
    );
    const page = container.querySelector<HTMLElement>("[data-grid-zoom]");
    if (!page) throw new Error("no zoomed root");
    return page;
  }

  it("zooms the whole page, CSS Sections included, to fit the width", () => {
    stubResizeObserver();
    const page = mount({ px: PAGE_PX * 1.25 });
    expect(page.style.zoom).toBe("1.25");
    expect(page.querySelector('[data-testid="grid"]')).not.toBeNull();
    expect(page.querySelector('[data-testid="css-section"]')).not.toBeNull();
  });

  it("rescales in 5% steps when the window resizes, down to MIN_ZOOM", () => {
    const observer = stubResizeObserver();
    const width = { px: PAGE_PX };
    const page = mount(width);
    expect(page.style.zoom).toBe("1");
    width.px = PAGE_PX * 1.26;
    act(observer.resize);
    expect(page.style.zoom).toBe("1.25");
    width.px = PAGE_PX * 0.86;
    act(observer.resize);
    expect(page.style.zoom).toBe("1");
  });

  it("scrolls a Grid wider than the window at MIN_ZOOM instead of widening the page", () => {
    stubResizeObserver();
    const page = mount({ px: PAGE_PX / 2 });
    expect(page.style.zoom).toBe(String(MIN_ZOOM));
    const outer = page.parentElement!;
    expect(outer.style.overflowX).toBe("auto");
    expect(outer.style.minWidth).toBe("0px");
  });

  it("leaves the page's own layout at zoom 1", () => {
    stubResizeObserver();
    const page = mount({ px: PAGE_PX * 1.25 });
    expect(page.style.width).toBe(`${PAGE_PX}px`);
    const grid = page.querySelector<HTMLElement>('[data-testid="grid"]');
    expect(grid?.style.zoom).toBe("");
  });
});

describe("zoomOf", () => {
  /** An element `layout` px wide in layout, drawn `drawn` px wide on screen. */
  function drawn(layout: number, drawnPx: number): HTMLElement {
    const el = document.createElement("div");
    Object.defineProperty(el, "offsetWidth", { value: layout });
    el.getBoundingClientRect = () => new DOMRect(0, 0, drawnPx, 10);
    return el;
  }

  it("is the on-screen width over the layout width", () => {
    expect(zoomOf(drawn(200, 250))).toBe(1.25);
    expect(zoomOf(drawn(200, 200))).toBe(1);
  });
  it("is 1 for an element not laid out", () => {
    expect(zoomOf(drawn(0, 0))).toBe(1);
  });
});
