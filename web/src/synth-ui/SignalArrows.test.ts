import { describe, expect, it } from "vitest";
import { arrowPath } from "./SignalArrows";

describe("arrowPath", () => {
  const box = (left: number, mid: number) => ({ left, right: left + 100, mid });

  it("curves between boxes at different heights", () => {
    expect(arrowPath(box(0, 50), box(200, 100))).toBe("M 100 50 C 150 50 150 100 200 100");
  });

  it("runs straight between boxes in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50))).toBe("M 100 50 C 150 50 150 50 200 50");
  });

  it("ends at the glyph left of the target, even in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50), true)).toBe("M 100 50 C 138.5 50 138.5 50 177 50");
  });

  it("runs level past a box it must go around, then bends", () => {
    const ring = box(150, 75);
    expect(arrowPath(box(0, 100), box(400, 50), false, ring)).toBe("M 100 100 H 250 C 325 100 325 50 400 50");
  });
});
