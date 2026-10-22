import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { PcmsPmtField, SnsCommonField } from "../../pkg/integral_wasm.js";
import { CELL } from "../synth-ui/grid";
import { cleanup, render } from "../test/dom";
import type { FieldKey } from "./fields";
import { sectionCells } from "./geometry";
import { GridSection, type Binding } from "./GridSection";
import { PARTIAL_LAYOUT } from "./pcm-synth/sections";
import { SNS_PARTIAL_SWITCH_LAYOUT } from "./sn-synth/partial";
import { switchStrip } from "./switchStrip";
import type { SectionLayout } from "./types";
import { validateSection } from "./validate";

afterEach(cleanup);

/** Both synth editors' partial strips, with the field each toggle writes. */
const STRIPS: [string, SectionLayout<FieldKey>, FieldKey][] = [
  ["SN Synth", SNS_PARTIAL_SWITCH_LAYOUT as SectionLayout<FieldKey>, SnsCommonField.Partial1Switch],
  ["PCM Synth", PARTIAL_LAYOUT as SectionLayout<FieldKey>, PcmsPmtField.PartialSwitch],
];

/** `layout` for Partial 2 with its switch at `value`, recording what its toggle writes. */
function draw(layout: SectionLayout<FieldKey>, value: number) {
  const writes: [FieldKey, number][] = [];
  const bind = (field: FieldKey): Binding => ({
    name: "Partial 2 Switch",
    value,
    min: 0,
    max: 1,
    defaultValue: 0,
    onChange: (v) => writes.push([field, v]),
  });
  const root = render(<GridSection layout={layout} scope={{ partial: 1 }} bind={bind} />).container
    .firstElementChild as HTMLElement;
  const title = root.children[1] as HTMLElement;
  const buttons = root.querySelectorAll("button");
  return { root, title, buttons, button: buttons[0]!, writes };
}

describe.each(STRIPS)("the %s partial strip", (_, layout, field) => {
  it("is the shared switch strip, valid", () => {
    expect(layout).toEqual(switchStrip({ title: "Partial {partial}", field, fields: layout.fields, h: layout.h }));
    expect(validateSection(layout)).toEqual([]);
  });

  it("is two Cells wide: the title column and the toggle's", () => {
    expect(sectionCells(layout).w).toBe(2);
    expect(draw(layout, 1).root.style.width).toBe(`${2 * CELL}px`);
  });

  it("draws its one Control as the power toggle, 1 Cell wide, beside the title", () => {
    const { buttons, button } = draw(layout, 1);
    expect(buttons).toHaveLength(1);
    // `PowerButton`: the power symbol, no text.
    expect(button.querySelector("svg path")).not.toBeNull();
    expect(button.textContent).toBe("");
    const host = button.closest<HTMLElement>("[data-field]")!;
    expect(host.dataset.field).toBe(String(field));
    const [left, width] = [parseFloat(host.style.left), parseFloat(host.style.width)];
    expect(left).toBeGreaterThanOrEqual(CELL);
    expect(left + width).toBeLessThanOrEqual(2 * CELL);
  });

  it("keeps the switch's name and pressed state, and writes the partial switch", () => {
    const on = draw(layout, 1);
    expect(on.button.title).toBe("Partial 2 Switch");
    expect(on.button.getAttribute("aria-pressed")).toBe("true");
    act(() => on.button.click());
    expect(on.writes).toEqual([[field, 0]]);
    cleanup();
    const off = draw(layout, 0);
    expect(off.button.getAttribute("aria-pressed")).toBe("false");
    act(() => off.button.click());
    expect(off.writes).toEqual([[field, 1]]);
  });

  it("reads its label bottom to top, as the mixer's strip names do", () => {
    const { title } = draw(layout, 1);
    expect(title.textContent).toBe("Partial 2");
    expect(title.style.writingMode).toBe("vertical-rl");
    expect(title.style.transform).toBe("rotate(180deg)");
  });
});
