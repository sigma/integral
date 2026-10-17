import { afterEach, describe, expect, it } from "vitest";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { OutputStrip } from "./OutputStrip";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const spec = {
  min: 10,
  max: 90,
  defaultValue: 40,
  formatValue: (v: number) => `${v}%`,
};
const fx1Spec = { ...spec, family: ParameterFamily.Filter };
const fx2Spec = { ...spec, family: ParameterFamily.Modulation };
const levelSpec = { ...spec };

const strip = {
  fx1: 30,
  fx2: 50,
  level: 70,
  onFx1Change: () => {},
  onFx2Change: () => {},
  onLevelChange: () => {},
};

const sliders = (container: HTMLElement) =>
  [...container.querySelectorAll('[role="slider"]')].map((el) => ({
    now: el.getAttribute("aria-valuenow"),
    min: el.getAttribute("aria-valuemin"),
    max: el.getAttribute("aria-valuemax"),
  }));

describe("OutputStrip", () => {
  it("takes each control's range, default and format from its passed spec", () => {
    const { container } = render(<OutputStrip {...strip} fx1Spec={fx1Spec} fx2Spec={fx2Spec} levelSpec={levelSpec} />);

    // FX1, FX2 knobs, then the level fader — every bound comes from the spec.
    expect(sliders(container)).toEqual([
      { now: "30", min: "10", max: "90" },
      { now: "50", min: "10", max: "90" },
      { now: "70", min: "10", max: "90" },
    ]);
    // Values render through the spec's format, not String(v).
    expect(container.textContent).toContain("30%");
    expect(container.textContent).toContain("70%");
  });

  it("colours its knobs by each spec's family", () => {
    const { container } = render(<OutputStrip {...strip} fx1Spec={fx1Spec} fx2Spec={fx2Spec} levelSpec={levelSpec} />);

    const fills = [...container.querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain("var(--family-filter)");
    expect(fills).toContain("var(--family-modulation)");
  });

  it("drives the optional pan knob from its spec", () => {
    const panSpec = { ...spec, family: ParameterFamily.Time, center: 50 };
    const { container } = render(
      <OutputStrip
        {...strip} fx1Spec={fx1Spec} fx2Spec={fx2Spec} levelSpec={levelSpec}
        pan={{ value: 60, onChange: () => {}, spec: panSpec }}
      />,
    );

    // Pan renders first, bound to its own spec's range and format.
    expect(sliders(container)[0]).toEqual({ now: "60", min: "10", max: "90" });
    const fills = [...container.querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain("var(--family-time)");
  });

  it("has no literal ranges: every control needs a spec", () => {
    // Type-level check: tsc fails if this stops being an error.
    // @ts-expect-error -- ranges come from specs, not literals
    void (<OutputStrip {...strip} fx1Spec={fx1Spec} fx2Spec={fx2Spec} levelSpec={levelSpec} min={0} max={127} />);
    // @ts-expect-error -- a knob spec needs a family
    void (<OutputStrip {...strip} fx1Spec={spec} fx2Spec={fx2Spec} levelSpec={levelSpec} />);
  });
});
