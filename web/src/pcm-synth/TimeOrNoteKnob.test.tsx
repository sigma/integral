import { afterEach, describe, expect, it } from "vitest";
import { PcmsPartialField } from "../../pkg/integral_wasm.js";
import { TimeOrNoteKnob } from "./TimeOrNoteKnob";
import { cleanup, click, render } from "../test/dom";

afterEach(cleanup);

function knob(field: PcmsPartialField, value: number) {
  const writes: number[] = [];
  const { container } = render(
    <TimeOrNoteKnob label="Rate" field={field} value={value} onChange={(v) => writes.push(v)} />,
  );
  const slider = container.querySelector("[role=slider]")!;
  const button = (label: string) =>
    [...container.querySelectorAll("button")].find((b) => b.textContent === label)!;
  return { container, writes, slider, button };
}

describe("TimeOrNoteKnob", () => {
  it("covers the free values in Hz mode", () => {
    const { slider, button } = knob(PcmsPartialField.Lfo1Rate, 10);
    expect(button("Hz").getAttribute("aria-pressed")).toBe("true");
    expect(slider.getAttribute("aria-valuemin")).toBe("0");
    expect(slider.getAttribute("aria-valuemax")).toBe("127");
  });

  it("covers the note values in note mode and shows note names", () => {
    const { container, slider, button } = knob(PcmsPartialField.Lfo1Rate, 140);
    expect(button("♪").getAttribute("aria-pressed")).toBe("true");
    expect(slider.getAttribute("aria-valuemin")).toBe("128");
    expect(slider.getAttribute("aria-valuemax")).toBe("149");
    expect(container.textContent).toContain("1/4");
  });

  it("switches to 1/4, and back to the default", () => {
    const free = knob(PcmsPartialField.Lfo1Rate, 10);
    click(free.button("♪"));
    click(free.button("Hz"));
    expect(free.writes).toEqual([140]);
    cleanup();
    const note = knob(PcmsPartialField.Lfo1Rate, 140);
    click(note.button("Hz"));
    expect(note.writes).toEqual([0]);
  });

  it("labels the partial Delay in ms", () => {
    expect(knob(PcmsPartialField.DelayTime, 0).button("ms")).toBeDefined();
  });
});
