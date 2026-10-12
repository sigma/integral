import { afterEach, describe, expect, it } from "vitest";
import { OscWaveScope } from "./OscWaveScope";
import { OSC_WAVE_NAMES } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathsOf(wave: string, pulseWidth = 0, pwmDepth = 0): string[] {
  const { container } = render(<OscWaveScope wave={wave} pulseWidth={pulseWidth} pwmDepth={pwmDepth} />);
  return [...container.querySelectorAll("path")].map((p) => p.getAttribute("d") ?? "");
}

const pathOf = (wave: string) => pathsOf(wave)[0] ?? null;

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

  it("draws PW-SQR at PW 0 as a square, thinner as PW rises", () => {
    expect(pathsOf("PW-SQR", 0)).toEqual([pathOf("SQR")]);
    expect(pathsOf("PW-SQR", 127)[0]).not.toEqual(pathOf("SQR"));
  });

  it("adds a PWM swing trace only when the depth is non-zero", () => {
    expect(pathsOf("PW-SQR", 64, 0)).toHaveLength(1);
    expect(pathsOf("PW-SQR", 64, 32)).toHaveLength(2);
    expect(pathsOf("SAW", 64, 32)).toHaveLength(1);
  });
});
