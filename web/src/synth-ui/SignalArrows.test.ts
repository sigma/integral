import { describe, expect, it } from "vitest";
import { arrowPath } from "./SignalArrows";

describe("arrowPath", () => {
  const box = (left: number, mid: number) => ({ left, right: left + 100, mid });

  it("curves between boxes at different heights", () => {
    expect(arrowPath(box(0, 50), box(200, 100))).toBe("M 100 50 C 150 50 150 100 200 100");
  });

  it("draws nothing between boxes in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50))).toBeNull();
  });

  it("ends at the glyph left of the target, even in line", () => {
    expect(arrowPath(box(0, 50), box(200, 50), true)).toBe("M 100 50 C 138.5 50 138.5 50 177 50");
  });
});
