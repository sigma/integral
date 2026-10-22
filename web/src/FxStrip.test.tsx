import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { chorus_params } from "../pkg/integral_wasm.js";
import { FxParams } from "./FxStrip";
import { fxTypes } from "./layouts";
import type { FxStripKind } from "./layouts/mixer/strip";
import { cleanup, render } from "./test/dom";
import { sectionSize } from "./test/grid";
import type { FxState } from "./types";

afterEach(cleanup);

function strip(block: FxStripKind, type: number, params: number[] = Array<number>(20).fill(0), enabled = true) {
  const onNibParam = vi.fn();
  const fx: FxState = { enabled, type, level: 100, output: 0, outputAssign: 0, params };
  const { container } = render(<FxParams block={block} fx={fx} onNibParam={onNibParam} />);
  return { container, onNibParam };
}

describe("FxParams", () => {
  it.each(["chorus", "reverb"] as const)("keeps one %s footprint whatever the type", (block) => {
    const sizes = fxTypes(block).map((t) => sectionSize(strip(block, t).container));
    for (const s of sizes) expect(s).toEqual(sizes[0]);
  });

  it("draws every parameter of the type", () => {
    const { container } = strip("chorus", 2);
    for (const def of chorus_params(2)) {
      expect(container.querySelector(`[data-field="${def.name}"]`), def.name).not.toBeNull();
    }
  });

  it("dims every parameter while the FX is switched off", () => {
    const off = [...strip("reverb", 3, undefined, false).container.querySelectorAll<HTMLElement>("[data-field]")];
    expect(off.length).toBeGreaterThan(0);
    for (const c of off) expect(c.dataset.inactive).toBe("dim");
    cleanup();
    expect(strip("reverb", 3).container.querySelector("[data-inactive]")).toBeNull();
  });

  it("keeps tempo sync inside the Control", () => {
    const def = chorus_params(2).find((d) => d.name === "Dly L");
    if (!def || def.syncIndex === undefined) throw new Error("no synced Dly L");
    const { container, onNibParam } = strip("chorus", 2);
    const control = container.querySelector<HTMLElement>('[data-field="Dly L"]');
    // The readout reads the free time, its unit the toggle.
    const unit = control?.querySelector("button");
    if (!unit) throw new Error("no sync unit in the Dly L Control");
    expect(unit.parentElement!.textContent).toBe(`${def.formatNumber(0)} ${def.syncLabels[0]}`);
    expect(unit.getAttribute("aria-pressed")).toBe("false");
    act(() => unit.click());
    expect(onNibParam).toHaveBeenCalledWith(def.syncIndex, 1);
  });

  it("edits the slot the param's def names", () => {
    const def = chorus_params(1).find((d) => d.name === "Depth");
    if (!def) throw new Error("no Depth");
    const { container, onNibParam } = strip("chorus", 1);
    const slider = container.querySelector<HTMLElement>('[data-field="Depth"] [role="slider"]');
    if (!slider) throw new Error("no Depth knob");
    act(() => {
      slider.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    expect(onNibParam).toHaveBeenCalledWith(def.index, 1);
  });
});
