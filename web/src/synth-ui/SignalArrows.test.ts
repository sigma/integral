import { describe, expect, it } from "vitest";
import { arrowPath } from "./SignalArrows";

describe("arrowPath", () => {
  const box = (left: number, mid: number) => ({ left, right: left + 100, top: mid - 20, mid, bottom: mid + 20 });

  it("curves between boxes at different heights", () => {
    expect(arrowPath(box(0, 50), box(200, 100))).toBe("M 100 50 C 150 50 150 100 200 100");
  });

  it("runs straight between boxes in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50))).toBe("M 100 50 C 150 50 150 50 200 50");
  });

  it("ends at the glyph left of the target, even in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50), true)).toBe("M 100 50 C 138.5 50 138.5 50 177 50");
  });

  it("curves just over a box it must go around, coming from above", () => {
    const block = box(150, 300);
    expect(arrowPath(box(0, 50), box(400, 400), false, block)).toBe(
      "M 100 50 C 175 50 175 270 250 270 C 325 270 325 400 400 400",
    );
  });

  it("curves just under a box it must go around", () => {
    // Ring's right edge at x 250, bottom 95: the edge passes through (250, 105)
    // and only rises after it.
    const ring = box(150, 75);
    expect(arrowPath(box(0, 300), box(400, 50), false, ring)).toBe(
      "M 100 300 C 175 300 175 105 250 105 C 325 105 325 50 400 50",
    );
  });
});
