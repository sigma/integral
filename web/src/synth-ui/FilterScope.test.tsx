import { afterEach, describe, expect, it } from "vitest";
import { PcmsPartialField, pcms_partial_spec } from "../../pkg/integral_wasm.js";
import { FilterScope } from "./FilterScope";
import { switchOptions } from "../fieldSpec";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathOf(type: string, cutoff = 64, resonance = 0): string {
  const d = render(<FilterScope type={type} cutoff={cutoff} resonance={resonance} />)
    .container.querySelector("path")?.getAttribute("d") ?? "";
  cleanup();
  return d;
}

/** y of the path's sample nearest x; the path is `M x y L x y …`. */
function yAt(d: string, x: number): number {
  const tokens = d.split(" ");
  let best = { dx: Infinity, y: NaN };
  for (let i = 0; i + 2 < tokens.length; i += 3) {
    const dx = Math.abs(Number(tokens[i + 1]) - x);
    if (dx < best.dx) best = { dx, y: Number(tokens[i + 2]) };
  }
  return best.y;
}

describe("FilterScope", () => {
  it("draws every PCM-S filter type", () => {
    for (const { label } of switchOptions(pcms_partial_spec(PcmsPartialField.TvfFilterType))) {
      expect(pathOf(label), label).toBeTruthy();
    }
  });

  it("follows cutoff and resonance", () => {
    // a low-pass passes the low end and cuts the top (y grows downwards)
    expect(yAt(pathOf("LPF"), 0)).toBeLessThan(yAt(pathOf("LPF"), 1));
    expect(pathOf("LPF", 100)).not.toEqual(pathOf("LPF", 20));
    expect(pathOf("LPF", 64, 127)).not.toEqual(pathOf("LPF", 64, 0));
    // LPF2 and LPF3 ignore Resonance
    expect(pathOf("LPF2", 64, 127)).toEqual(pathOf("LPF2", 64, 0));
  });
});
