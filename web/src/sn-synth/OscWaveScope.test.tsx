import { afterEach, describe, expect, it } from "vitest";
import { SnsPartialField, sns_partial_spec } from "../../pkg/integral_wasm.js";
import { OscWaveScope } from "./OscWaveScope";
import { switchOptions } from "../fieldSpec";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathsOf(wave: string, pulseWidth = 0, pwmDepth = 0, superSawDetune = 0, pwShift = 127): string[] {
  const { container } = render(
    <OscWaveScope wave={wave} pulseWidth={pulseWidth} pwmDepth={pwmDepth} pwShift={pwShift}
      superSawDetune={superSawDetune} />,
  );
  return [...container.querySelectorAll("path")].map((p) => p.getAttribute("d") ?? "");
}

const pathOf = (wave: string) => pathsOf(wave)[0] ?? null;

describe("OscWaveScope", () => {
  it("draws every OSC wave except PCM", () => {
    for (const { label: wave } of switchOptions(sns_partial_spec(SnsPartialField.OscWave))) {
      if (wave === "PCM") expect(pathOf(wave)).toBeNull();
      else expect(pathOf(wave), wave).toBeTruthy();
    }
  });

  it("ends a saw whose last cycle completes at its peak, not its trough", () => {
    expect(pathOf("SAW")).toMatch(/L 1 0$/);
  });

  it("draws the saw's resets as vertical lines, not gaps", () => {
    expect(pathOf("SAW")).toContain("L 0.5 0 L 0.5 1");
    expect(pathOf("SAW")!.slice(1)).not.toContain("M");
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

  it("draws SP-SAW as one clean saw at zero detune, reshaped as S-Saw rises", () => {
    const at = (d: string, x: string) => Number(d.split(` ${x} `)[1]?.split(" ")[0]);
    const clean = pathsOf("SP-SAW", 0, 0, 0)[0]!;
    expect(at(clean, "0.25")).toBeCloseTo(0.5); // halfway up the first ramp
    expect(pathsOf("SP-SAW", 0, 0, 127)[0]).not.toEqual(clean);
  });

  it("pulls PW-SQR back to a square as PW Shift drops", () => {
    expect(pathsOf("PW-SQR", 127, 0, 0, 0)).toEqual([pathOf("SQR")]);
    expect(pathsOf("PW-SQR", 127, 0, 0, 64)[0]).not.toEqual(pathsOf("PW-SQR", 127, 0, 0, 127)[0]);
  });
});
