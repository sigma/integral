import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MfxHeaderField, mfx_params, mfxHeaderFamily } from "../pkg/integral_wasm.js";
import { familyVar } from "./synth-ui";
import { MfxPanel, type MfxState } from "./MfxPanel";
import { cleanup, render } from "./test/dom";
import { sectionSize } from "./test/grid";

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
    expect(knob?.textContent).toContain("+0 dB");
  });

  it("colours a knob by its def's family", () => {
    const def = mfx_params(22).find((d) => d.name === "Rate");
    if (!def) throw new Error("no Rate");
    const { container } = typePanel(22, Array<number>(32).fill(127));
    const fills = [...container.querySelectorAll('[title="Rate"] circle')].map((c) => (c as SVGElement).style.fill);
    expect(fills).toContain(familyVar(def.family));
  });

  it("edits the note slot of a delay time in note mode", () => {
    // Delay Left: switch at slot 0, msec at 1, note at 2.
    const params = Array<number>(32).fill(0);
    params[0] = 1;
    params[2] = 12;
    const { container, onNibParam } = typePanel(34, params);
    // The readout reads the note, its unit the toggle back to msec.
    const control = container.querySelector('[title="Delay Left"]');
    expect(control?.textContent).toContain("1/4 ♪");
    const unit = control?.querySelector("button");
    expect(unit?.getAttribute("aria-pressed")).toBe("true");
    act(() => unit?.click());
    expect(onNibParam).toHaveBeenCalledWith(0, 0);
  });
});

describe("MfxPanel layout", () => {
  function draw(mfxType: number, params: number[] = []) {
    const onNibParam = vi.fn();
    const onHeaderParam = vi.fn();
    const mfx: MfxState = { mfxType, chorusSend: 0, reverbSend: 0, controls: [], params };
    const { container } = render(
      <MfxPanel mfx={mfx} onTypeChange={vi.fn()} onHeaderParam={onHeaderParam} onNibParam={onNibParam} />,
    );
    return { container, onNibParam, onHeaderParam };
  }

  it("keeps one footprint whatever the type, hand or generated", () => {
    // Thru, Equalizer (hand), Slicer (generated, most params), Delay (hand).
    const sizes = [0, 1, 18, 34].map((t) => sectionSize(draw(t).container));
    for (const s of sizes) expect(s).toEqual(sizes[0]);
  });

  it("places every parameter of a generated type", () => {
    const { container } = draw(18);
    for (const def of mfx_params(18)) {
      expect(container.querySelector(`[data-field="${def.name}"]`), def.name).not.toBeNull();
    }
  });

  it("writes the output Level from its fader in the parameter Section", () => {
    const level = mfx_params(22).find((d) => d.name === "Level");
    if (!level) throw new Error("no Level");
    const params = Array<number>(32).fill(0);
    params[level.index] = 100;
    const { container, onNibParam } = draw(22, params);
    const slider = container.querySelector<HTMLElement>('[data-field="Level"] [role="slider"]');
    if (!slider) throw new Error("no Level fader");
    act(() => {
      slider.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    expect(onNibParam).toHaveBeenCalledWith(level.index, 101);
  });

  it("keeps the Chorus and Reverb sends", () => {
    const { container, onHeaderParam } = draw(22);
    const send = container.querySelector<HTMLElement>('[title="Reverb Send Level"] [role="slider"]');
    if (!send) throw new Error("no Reverb send");
    act(() => {
      send.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    expect(onHeaderParam).toHaveBeenCalledWith(MfxHeaderField.ReverbSend, 0, 1);
  });
});

describe("MfxPanel Control Assign", () => {
  it("offers the type's destinations and writes the picked index", () => {
    const onHeaderParam = vi.fn();
    const mfx: MfxState = {
      mfxType: 34,
      chorusSend: 0,
      reverbSend: 0,
      controls: [{ source: 0, sens: 64, assign: 0 }],
      params: [],
    };
    const { container } = render(
      <MfxPanel mfx={mfx} onTypeChange={vi.fn()} onHeaderParam={onHeaderParam} onNibParam={vi.fn()} />,
    );
    const select = container.querySelector<HTMLSelectElement>('select[title="Destination"]');
    if (!select) throw new Error("no Destination picker");
    expect([...select.options].map((o) => o.textContent)).toEqual(["OFF", "Feedback", "Balance"]);
    act(() => {
      select.value = "2";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(onHeaderParam).toHaveBeenCalledWith(MfxHeaderField.ControlAssign, 0, 2);
  });
});

describe("MfxPanel Control Sens", () => {
  it("takes its family from the core", () => {
    const mfx: MfxState = {
      mfxType: 0,
      chorusSend: 0,
      reverbSend: 0,
      controls: [{ source: 0, sens: 127, assign: 0 }],
      params: [],
    };
    const { container } = render(
      <MfxPanel mfx={mfx} onTypeChange={vi.fn()} onHeaderParam={vi.fn()} onNibParam={vi.fn()} />,
    );
    const fills = [...container.querySelectorAll('[title="Macro 1 Depth"] circle')].map(
      (c) => (c as SVGElement).style.fill,
    );
    expect(fills).toContain(familyVar(mfxHeaderFamily(MfxHeaderField.ControlSens)));
  });
});
