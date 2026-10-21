import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { chorus_params } from "../pkg/integral_wasm.js";
import { FxStrip } from "./FxStrip";
import { fxTypes, type FxBlock } from "./layouts";
import { cleanup, render } from "./test/dom";
import { sectionSize } from "./test/grid";
import type { FxState } from "./types";

afterEach(cleanup);

function strip(block: Exclude<FxBlock, "mfx">, type: number, params: number[] = Array<number>(20).fill(0)) {
  const onNibParam = vi.fn();
  const fx: FxState = { enabled: true, type, level: 100, output: 0, outputAssign: 0, params };
  const typeNames = fxTypes(block).map(String);
  const { container } = render(
    <FxStrip
      label={block === "chorus" ? "FX1" : "FX2"}
      block={block}
      fx={fx}
      typeNames={typeNames}
      routes={[]}
      onToggleSwitch={vi.fn()}
      onParam={vi.fn()}
      onNibParam={onNibParam}
    />,
  );
  return { container, onNibParam };
}

describe("FxStrip", () => {
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

  it("keeps tempo sync inside the Control", () => {
    const def = chorus_params(2).find((d) => d.name === "Dly L");
    if (!def || def.syncIndex === undefined) throw new Error("no synced Dly L");
    const { container, onNibParam } = strip("chorus", 2);
    const control = container.querySelector<HTMLElement>('[data-field="Dly L"]');
    const note = [...(control?.querySelectorAll("button") ?? [])].find((b) => b.textContent === def.syncLabels[1]);
    if (!note) throw new Error("no note toggle in the Dly L Control");
    act(() => note.click());
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
