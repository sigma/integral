import { afterEach, describe, expect, it } from "vitest";
import { SynthFader, FADER_LABEL_HEIGHT, LEVEL_FRAME } from "./SynthFader";
import { CELL, FOOTPRINT, NAME, READOUT } from "./grid";
import { expectFillsFootprint, rootOf, stackedHeight } from "../test/grid";
import { cleanup, render } from "../test/dom";
import css from "./SynthFader.module.css";

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

  it("draws a level fader as the pre-Grid mixer's: panel, scale with its 0 dB mark, wide cap, no fill", () => {
    const plain = rootOf(render(<SynthFader {...fader} value={60} gridRows={8} />).container);
    expect(plain.textContent).not.toContain("0dB");
    // A plain fader fills from its default; a level fader never does.
    expect(plain.querySelector(`.${css.fill}`)).not.toBeNull();
    const { container } = render(<SynthFader {...fader} value={60} gridRows={8} hideLabel scale />);
    const root = rootOf(container);
    expectFillsFootprint(root, { w: FOOTPRINT.levelFaderW, h: 8 });
    expect(root.classList).toContain(css.level);
    const slider = root.querySelector<HTMLElement>('[role="slider"]')!;
    expect(slider.className).toBe(css.levelTrack);
    // The scale: the MIDI ruler, 100 marked 0 dB and drawn emphasised.
    const marks = [...slider.querySelectorAll<HTMLElement>(`.${css.mark}`)];
    expect(marks.map((m) => m.textContent)).toEqual(["127", "0dB", "80", "60", "40", "20", "0"]);
    expect(marks.filter((m) => m.classList.contains(css.markZero!)).map((m) => m.textContent)).toEqual(["0dB"]);
    // The groove, and the wide cap last, its centre level with the mark of its value.
    expect(slider.querySelector(`.${css.groove}`)).not.toBeNull();
    const cap = slider.lastElementChild as HTMLElement;
    expect(cap.classList).toContain(css.cap);
    expect(cap.style.top).toBe(marks.find((m) => m.textContent === "60")!.style.top);
    expect(root.querySelector(`.${css.fill}`)).toBeNull();
    expect(root.querySelector(`.${css.thumb}`)).toBeNull();
    // The value under it, in its panel; the panel's frame, track and readout stack to the footprint.
    expect(root.lastElementChild!.textContent).toBe("60");
    expect(stackedHeight([...root.children]) + 2 * LEVEL_FRAME).toBe(8 * CELL);
  });

  it("marks a level fader over another range by its formatter, 0 dB at its default", () => {
    const { container } = render(
      <SynthFader {...fader} min={0} max={24} defaultValue={0} value={6} gridRows={8} hideLabel scale formatValue={(v) => `+${v}dB`} />,
    );
    const marks = [...container.querySelectorAll<HTMLElement>(`.${css.mark}`)];
    expect(marks.map((m) => m.textContent)).toEqual(["+24dB", "+18dB", "+12dB", "+6dB", "+0dB"]);
    expect(marks[marks.length - 1]!.classList).toContain(css.markZero);
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
