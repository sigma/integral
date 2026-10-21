import { afterEach, describe, expect, it } from "vitest";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { SynthKnob } from "./SynthKnob";
import { CELL, FOOTPRINT, NAME, READOUT } from "./grid";
import { expectFillsFootprint, rootOf, stackedHeight } from "../test/grid";
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

const ledFills = (container: HTMLElement) =>
  [...container.querySelectorAll("circle")]
    .map((c) => c.style.fill)
    .filter((f) => f !== "");

describe("SynthKnob", () => {
  it("lights its LEDs in its family's colour and keeps the pointer neutral", () => {
    const { container } = render(
      <SynthKnob {...knob} family={ParameterFamily.Filter} />,
    );
    expect(ledFills(container)).toContain("var(--family-filter)");
    expect(container.querySelector("line")!.style.stroke).toBe("");
  });

  it("lights its LEDs in the neutral Common family when it has no family", () => {
    const { container } = render(<SynthKnob {...knob} />);
    expect(ledFills(container)).toContain("var(--family-common)");
  });

  it("takes its colour only from a family, never a literal", () => {
    // Type-level check: tsc fails if this line stops being an error.
    // @ts-expect-error -- a knob has no colour prop
    void (<SynthKnob {...knob} family={ParameterFamily.Filter} color="#fff" />);
  });

  it.each([
    ["sm", FOOTPRINT.knobSm],
    ["lg", FOOTPRINT.knobLg],
  ] as const)("fills its %s footprint at scale 1 on the Grid", (size, cells) => {
    const { container } = render(<SynthKnob {...knob} size={size} grid />);
    const root = rootOf(container);
    expectFillsFootprint(root, cells);
    const svg = root.querySelector("svg")!;
    expect(Number(svg.getAttribute("width"))).toBeLessThanOrEqual(
      cells.w * CELL,
    );
    expect(stackedHeight([...root.children])).toBeLessThanOrEqual(
      cells.h * CELL,
    );
  });

  it("makes room for an addon line inside its Grid footprint", () => {
    const { container } = render(<SynthKnob {...knob} grid addon={<button>♪</button>} />);
    const root = rootOf(container);
    expectFillsFootprint(root, FOOTPRINT.knobSm);
    expect(root.querySelector("button")?.textContent).toBe("♪");
    expect(stackedHeight([...root.children])).toBeLessThanOrEqual(FOOTPRINT.knobSm.h * CELL);
  });

  it("keeps a 7px name and a 9px readout on the Grid", () => {
    const { container } = render(<SynthKnob {...knob} grid />);
    const [name, readout] = [...container.querySelectorAll("span")];
    expect(name!.textContent).toBe("Cutoff");
    expect(name!.style.fontSize).toBe(`${NAME.font}px`);
    expect(readout!.textContent).toBe("127");
    expect(readout!.style.fontSize).toBe(`${READOUT.font}px`);
    expect(NAME.font).toBe(7);
    expect(READOUT.font).toBe(9);
  });

  it("keeps its natural size off the Grid", () => {
    const { container } = render(<SynthKnob {...knob} />);
    const root = rootOf(container);
    expect(root.style.width).toBe("");
    expect(root.querySelector("svg")!.getAttribute("width")).toBe("52");
  });
});
