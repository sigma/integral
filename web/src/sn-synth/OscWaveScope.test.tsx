import { afterEach, describe, expect, it } from "vitest";
import { OscWaveScope } from "./OscWaveScope";
import { OSC_WAVE_NAMES } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathOf(wave: string): string | null {
  return render(<OscWaveScope wave={wave} />).container.querySelector("path")?.getAttribute("d") ?? null;
}

describe("OscWaveScope", () => {
  it("draws every OSC wave except PCM", () => {
    for (const wave of OSC_WAVE_NAMES) {
      if (wave === "PCM") expect(pathOf(wave)).toBeNull();
      else expect(pathOf(wave), wave).toBeTruthy();
    }
  });

  it("ends a saw whose last cycle completes at its peak, not its trough", () => {
    expect(pathOf("SAW")).toMatch(/L 1 0$/);
  });
});
