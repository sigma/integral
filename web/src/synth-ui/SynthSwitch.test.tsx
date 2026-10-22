import { afterEach, describe, expect, it } from "vitest";
import { SynthSwitch } from "./SynthSwitch";
import { CELL, FOOTPRINT, NAME, OPTION_TEXT } from "./grid";
import { expectFillsFootprint, rootOf, stackedHeight } from "../test/grid";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const onOff = [
  { value: 0, label: "OFF" },
  { value: 1, label: "ON" },
];
const waves = ["SAW", "SQR", "PW", "TRI"].map((label, value) => ({ value, label }));

describe("SynthSwitch on the Grid", () => {
  it.each([
    ["with", "EQ"],
    ["without", undefined],
  ])("fills 2×1 %s a name", (_, label) => {
    const { container } = render(
      <SynthSwitch value={1} options={onOff} onChange={() => {}} label={label} grid />,
    );
    const root = rootOf(container);
    expectFillsFootprint(root, FOOTPRINT.switch);
    expect(stackedHeight([...root.children])).toBeLessThanOrEqual(CELL);
  });

  it("keeps a 7px name", () => {
    const { container } = render(
      <SynthSwitch value={1} options={onOff} onChange={() => {}} label="EQ" grid />,
    );
    expect(container.querySelector("span")!.style.fontSize).toBe(`${NAME.font}px`);
  });

  it("gives a vertical option list one Cell row per option", () => {
    const { container } = render(
      <SynthSwitch value={2} options={waves} onChange={() => {}} grid />,
    );
    const root = rootOf(container);
    expectFillsFootprint(root, { w: FOOTPRINT.optionListW, h: waves.length });
    const rows = [...root.querySelectorAll<HTMLElement>('[role="radio"]')];
    expect(rows.map((r) => r.style.height)).toEqual(waves.map(() => `${CELL}px`));
  });

  it("draws option names in the type and spacing the layout validator measures with", () => {
    const { container: list } = render(<SynthSwitch value={0} options={waves} onChange={() => {}} grid />);
    const row = list.querySelector<HTMLElement>('[role="radio"]')!;
    const l = OPTION_TEXT.list;
    expect([row.style.fontSize, row.style.padding, row.style.gap]).toEqual([`${l.fontSize}px`, `0px ${l.padding}px`, `${l.gap}px`]);
    expect(row.querySelector<HTMLElement>("span")!.style.width).toBe(`${l.led}px`);
    cleanup();
    const { container: toggle } = render(<SynthSwitch value={0} options={onOff} onChange={() => {}} grid />);
    const button = toggle.querySelector<HTMLElement>("button")!;
    const t = OPTION_TEXT.switch;
    expect([button.style.fontSize, button.style.padding, button.style.gap]).toEqual([`${t.fontSize}px`, `0px ${t.padding}px`, `${t.gap}px`]);
    expect(button.style.width).toBe(`calc(100% - ${2 * t.margin}px)`);
    expect(button.querySelector<HTMLElement>("span")!.style.width).toBe(`${t.led}px`);
  });

  it("puts a list's name on its own row above the options", () => {
    const { container } = render(
      <SynthSwitch value={2} options={waves} onChange={() => {}} label="WAVE" grid />,
    );
    const root = rootOf(container);
    expectFillsFootprint(root, { w: FOOTPRINT.optionListW, h: waves.length + 1 });
    expect(stackedHeight([...root.children])).toBe((waves.length + 1) * CELL);
  });

  it("stretches a list to the N rows a layout gives it", () => {
    const { container } = render(
      <SynthSwitch value={0} options={waves} onChange={() => {}} gridRows={6} />,
    );
    expectFillsFootprint(rootOf(container), { w: FOOTPRINT.optionListW, h: 6 });
  });

  it("never clips options to a short N", () => {
    const { container } = render(
      <SynthSwitch value={0} options={waves} onChange={() => {}} label="WAVE" gridRows={4} />,
    );
    expectFillsFootprint(rootOf(container), { w: FOOTPRINT.optionListW, h: waves.length + 1 });
  });

  it("fits a box cut short by edge padding, taking the px from its name's row first", () => {
    const pad = 7;
    const box = { width: 2 * CELL - pad, height: (waves.length + 1) * CELL - pad };
    const { container } = render(
      <SynthSwitch value={0} options={waves} onChange={() => {}} label="WAVE" gridRows={5} box={box} />,
    );
    const root = rootOf(container);
    expect([root.style.width, root.style.height]).toEqual([`${box.width}px`, `${box.height}px`]);
    expect(stackedHeight([...root.children])).toBe(box.height);
    const rows = [...root.querySelectorAll<HTMLElement>('[role="radio"]')];
    expect(rows.map((r) => r.style.height)).toEqual(waves.map(() => `${CELL}px`));
  });

  it("sizes a toggle with an empty name as unnamed", () => {
    const button = (label: string | undefined) =>
      rootOf(
        render(
          <SynthSwitch value={1} options={onOff} onChange={() => {}} label={label} grid />,
        ).container,
      ).querySelector("button")!.style.height;
    expect(button("")).toBe(button(undefined));
    expect(button("")).not.toBe(button("EQ"));
  });
});

describe("SynthSwitch off the Grid", () => {
  it("keeps its natural size", () => {
    const { container } = render(
      <SynthSwitch value={0} options={waves} onChange={() => {}} label="WAVE" />,
    );
    const root = rootOf(container);
    expect(root.style.width).toBe("");
    expect(root.querySelector<HTMLElement>('[role="radio"]')!.style.height).toBe("");
  });
});
