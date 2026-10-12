import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MfxHeaderField } from "../pkg/integral_wasm.js";
import { MfxPanel, type MfxState } from "./MfxPanel";
import { cleanup, render } from "./test/dom";

afterEach(cleanup);

function panel(source: number) {
  const onHeaderParam = vi.fn();
  const mfx: MfxState = {
    mfxType: 0,
    chorusSend: 0,
    reverbSend: 0,
    controls: [{ source, sens: 64, assign: 0 }],
    params: [],
  };
  const { container } = render(
    <MfxPanel
      mfx={mfx}
      onTypeChange={vi.fn()}
      onHeaderParam={onHeaderParam}
      onNibParam={vi.fn()}
    />,
  );
  const select = container.querySelector<HTMLSelectElement>('select[title="Source"]');
  if (!select) throw new Error("no Source picker");
  return { select, onHeaderParam };
}

describe("MfxPanel Control Source", () => {
  it("displays a stored 97 as AFT", () => {
    const { select } = panel(97);
    expect(select.selectedOptions[0]?.textContent).toBe("AFT");
  });

  it("writes 96 when BEND is picked", () => {
    const { select, onHeaderParam } = panel(0);
    const bend = [...select.options].find((o) => o.textContent === "BEND");
    if (!bend) throw new Error("no BEND option");
    act(() => {
      select.value = bend.value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(onHeaderParam).toHaveBeenCalledWith(MfxHeaderField.ControlSource, 0, 96);
  });

  it("does not offer value 32", () => {
    const { select } = panel(0);
    const values = [...select.options].map((o) => Number(o.value));
    expect(values).not.toContain(32);
    expect(values).toContain(33);
  });
});

describe("MfxPanel params", () => {
  function typePanel(mfxType: number, params: number[]) {
    const onNibParam = vi.fn();
    const mfx: MfxState = { mfxType, chorusSend: 0, reverbSend: 0, controls: [], params };
    const { container } = render(
      <MfxPanel mfx={mfx} onTypeChange={vi.fn()} onHeaderParam={vi.fn()} onNibParam={onNibParam} />,
    );
    return { container, onNibParam };
  }

  it("formats stored values through the spec", () => {
    // Chorus Low Gain is slot 8; stored 15 is 0 dB.
    const params = Array<number>(32).fill(0);
    params[8] = 15;
    const { container } = typePanel(22, params);
    const knob = container.querySelector('[title="Low Gain"]');
    expect(knob?.textContent).toContain("+0dB");
  });

  it("edits the note slot of a delay time in note mode", () => {
    // Delay Left: switch at slot 0, msec at 1, note at 2.
    const params = Array<number>(32).fill(0);
    params[0] = 1;
    params[2] = 12;
    const { container, onNibParam } = typePanel(34, params);
    expect(container.querySelector('[title="Delay Left"]')?.textContent).toContain("1/4");
    const ms = [...container.querySelectorAll("button")].find((b) => b.textContent === "ms");
    act(() => ms?.click());
    expect(onNibParam).toHaveBeenCalledWith(0, 0);
  });
});
