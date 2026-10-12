import { afterEach, describe, expect, it } from "vitest";
import { LfoScope } from "./LfoScope";
import { LFO_SHAPE_NAMES } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathOf(shape: string, rate = 0): string | null {
  return render(<LfoScope shape={shape} rate={rate} />).container.querySelector("path")?.getAttribute("d") ?? null;
}

describe("LfoScope", () => {
  it("draws every LFO shape", () => {
    for (const shape of LFO_SHAPE_NAMES) expect(pathOf(shape), shape).toBeTruthy();
  });

  it("fits more cycles as the rate rises", () => {
    expect(pathOf("SIN", 127)).not.toEqual(pathOf("SIN", 0));
  });
});
