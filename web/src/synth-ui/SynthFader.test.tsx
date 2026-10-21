import { afterEach, describe, expect, it } from "vitest";
import { SynthFader, FADER_LABEL_HEIGHT } from "./SynthFader";
import { CELL, FOOTPRINT, NAME, READOUT } from "./grid";
import { expectFillsFootprint, rootOf, stackedHeight } from "../test/grid";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const fader = {
  label: "Level",
  value: 100,
  min: 0,
  max: 127,
  defaultValue: 100,
  onChange: () => {},
};

describe("SynthFader", () => {
  it.each([3, 4, 6])("fills 2×%i on the Grid", (rows) => {
    const { container } = render(<SynthFader {...fader} gridRows={rows} />);
    const root = rootOf(container);
    const cells = { w: FOOTPRINT.faderW, h: rows };
    expectFillsFootprint(root, cells);
    expect(stackedHeight([...root.children])).toBe(rows * CELL);
  });

  it("makes room for an addon line inside its Grid footprint", () => {
    const { container } = render(<SynthFader {...fader} gridRows={4} addon={<button>♪</button>} />);
    const root = rootOf(container);
    expectFillsFootprint(root, { w: FOOTPRINT.faderW, h: 4 });
    expect(root.querySelector("button")?.textContent).toBe("♪");
    expect(stackedHeight([...root.children])).toBe(4 * CELL);
  });

  it("draws the level scale beside its track, inside its Grid footprint, when asked", () => {
    const plain = rootOf(render(<SynthFader {...fader} gridRows={8} />).container);
    expect(plain.textContent).not.toContain("0dB");
    const { container } = render(<SynthFader {...fader} gridRows={8} scale formatValue={(v) => `#${v}`} />);
    const root = rootOf(container);
    expectFillsFootprint(root, { w: FOOTPRINT.faderW, h: 8 });
    const labels = [...root.querySelectorAll("span")].map((s) => s.textContent);
    expect(labels).toEqual(expect.arrayContaining(["#127", "0dB", "#80", "#0"]));
    // The 0 dB mark sits level with the thumb at that value.
    const zero = [...root.querySelectorAll<HTMLElement>("span")].find((s) => s.textContent === "0dB")!.parentElement!;
    const thumb = root.querySelector<HTMLElement>('[role="slider"] > div:last-child')!;
    expect(zero.style.top).toBe(thumb.style.top);
  });

  it("gives the track the name's row when the name is hidden", () => {
    const shown = render(<SynthFader {...fader} gridRows={4} />);
    const hidden = render(<SynthFader {...fader} gridRows={4} hideLabel />);
    const track = (c: HTMLElement) =>
      parseFloat(c.querySelector<HTMLElement>('[role="slider"]')!.style.height);
    expect(track(hidden.container) - track(shown.container)).toBe(NAME.line);
    expectFillsFootprint(rootOf(hidden.container), { w: FOOTPRINT.faderW, h: 4 });
  });

  it.each([0, 0.5, 1])("keeps its thumb inside the footprint at %f of its range", (at) => {
    const { container } = render(
      <SynthFader {...fader} value={Math.round(at * fader.max)} gridRows={3} hideLabel />,
    );
    const area = container.querySelector<HTMLElement>('[role="slider"]')!;
    const thumb = area.lastElementChild as HTMLElement;
    const top = parseFloat(thumb.style.top) + parseFloat(thumb.style.marginTop);
    expect(top).toBeGreaterThanOrEqual(0);
    expect(top + parseFloat(thumb.style.height)).toBeLessThanOrEqual(
      parseFloat(area.style.height),
    );
  });

  it("keeps a 7px name and a 9px readout on the Grid", () => {
    const { container } = render(<SynthFader {...fader} gridRows={4} />);
    const [name, readout] = [...container.querySelectorAll("span")];
    expect(name!.style.fontSize).toBe(`${NAME.font}px`);
    expect(readout!.style.fontSize).toBe(`${READOUT.font}px`);
  });

  it("keeps its natural size off the Grid", () => {
    const { container } = render(<SynthFader {...fader} />);
    const root = rootOf(container);
    expect(root.style.height).toBe("");
    expect((root.firstElementChild as HTMLElement).style.height).toBe(
      `${FADER_LABEL_HEIGHT}px`,
    );
  });
});
