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
      <SynthKnob {...knob} family={ParameterFamily.Filter} />,
    );
    const fills = [...container.querySelectorAll("circle")].map(
      (c) => c.style.fill,
    );
    expect(fills).toContain("var(--family-filter)");
    expect(container.querySelector("line")!.style.stroke).toBe("");
  });

  it("takes its colour only from a family, never a literal", () => {
    // Type-level checks: tsc fails if either line stops being an error.
    // @ts-expect-error -- a knob has no colour prop
    void (<SynthKnob {...knob} family={ParameterFamily.Filter} color="#fff" />);
    // @ts-expect-error -- a knob needs a family
    void (<SynthKnob {...knob} />);
  });
});
