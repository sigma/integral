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
