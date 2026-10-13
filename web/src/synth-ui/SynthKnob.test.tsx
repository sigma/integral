import { afterEach, describe, expect, it } from "vitest";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { SynthKnob } from "./SynthKnob";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const knob = {
  label: "Cutoff",
  value: 127,
  min: 0,
  max: 127,
  defaultValue: 64,
  onChange: () => {},
};

describe("SynthKnob", () => {
  it("lights its LEDs in its family's colour and keeps the pointer neutral", () => {
    const { container } = render(
      <SynthKnob {...knob} color="#fc8" family={ParameterFamily.Filter} />,
    );
    const fills = [...container.querySelectorAll("circle")].map(
      (c) => c.style.fill,
    );
    expect(fills).toContain("var(--family-filter)");
    expect(fills).not.toContain("#fc8");
    expect(container.querySelector("line")!.style.stroke).toBe("");
  });

  it("falls back to `color` without a family", () => {
    const { container } = render(<SynthKnob {...knob} color="red" />);
    expect(
      [...container.querySelectorAll("circle")].map((c) => c.style.fill),
    ).toContain("red");
    expect(container.querySelector("line")!.style.stroke).toBe("red");
  });
});
