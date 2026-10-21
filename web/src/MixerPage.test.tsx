import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { MixerPage } from "./MixerPage";
import { EQ_LAYOUT } from "./layouts/mixer/eq";
import { CELL } from "./synth-ui/grid";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import { defaultMixerState } from "./types";
import { cleanup, render } from "./test/dom";

// jsdom lacks the ResizeObserver the FX strips' EQ plot measures with.
globalThis.ResizeObserver ??= class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
};

afterEach(cleanup);

const expansions = {
  unloadedExpansionOf: () => null,
} as unknown as UseExpansionsResult;

/** The mixer page with the given failed block labels. */
function page(failedBlocks: string[]): { el: HTMLElement; reloaded: () => boolean } {
  let reloaded = false;
  const mixer = {
    state: { ...defaultMixerState(), loading: false, failedBlocks },
    reload: () => {
      reloaded = true;
    },
  } as unknown as UseMixerResult;
  return {
    el: render(<MixerPage mixer={mixer} expansions={expansions} />).container,
    reloaded: () => reloaded,
  };
}

describe("MixerPage", () => {
  it("warns about blocks whose load failed, with a retry", () => {
    const { el, reloaded } = page(["Chorus", "Part 4"]);
    expect(el.textContent).toContain("2 blocks failed to load");
    el.querySelector("button")!.click();
    expect(reloaded()).toBe(true);
  });

  it("warns nothing while every block loaded", () => {
    expect(page([]).el.textContent).not.toContain("failed to load");
  });

  /** A loaded mixer with the Comp+EQ units shown, writing through `setMixerField`. */
  function loaded() {
    const setMixerField = vi.fn();
    const base = defaultMixerState();
    const mixer = {
      state: { ...base, loading: false, drumCompEq: { ...base.drumCompEq, enabled: true } },
      setMixerField,
    } as unknown as UseMixerResult;
    const el = render(<MixerPage mixer={mixer} expansions={expansions} />).container;
    // Strip order: the parts, Ext, the Comp+EQ units, then the master.
    const eqs = [...el.querySelectorAll<HTMLElement>('[data-grid-section="EQ"]')];
    return { el, base, eqs, setMixerField };
  }

  /** The Control bound to `field` in one EQ Section. */
  const control = (eq: HTMLElement, field: MixerField): HTMLElement => {
    const found = eq.querySelector<HTMLElement>(`[data-field="${field}"]`);
    if (!found) throw new Error(`no ${MixerField[field]} in the EQ`);
    return found;
  };

  /** Turns a knob one step up from the keyboard. */
  const stepUp = (knob: HTMLElement) =>
    act(() => {
      knob.querySelector('[role="slider"]')!.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }),
      );
    });

  it("draws the Grid EQ, every band, in each part, Comp+EQ and master strip", () => {
    const { base, eqs } = loaded();
    // Ext keeps its (hidden) EQ, so its rows stay level with the other strips.
    expect(eqs).toHaveLength(base.parts.length + 1 + base.drumCompEq.units.length + 1);
    const ext = eqs[base.parts.length]!;
    expect(ext.style.visibility).toBe("hidden");
    for (const eq of eqs) {
      expect(eq.style.width).toBe(`${EQ_LAYOUT.w * CELL}px`);
      for (const field of [
        MixerField.EqSwitch, MixerField.EqHighFreq, MixerField.EqHighGain, MixerField.EqMidFreq,
        MixerField.EqMidGain, MixerField.EqMidQ, MixerField.EqLowFreq, MixerField.EqLowGain,
      ]) {
        control(eq, field);
      }
    }
  });

  it("shows each strip's own EQ values", () => {
    const { base, eqs } = loaded();
    const last = base.drumCompEq.units.length - 1;
    const unitEq = eqs[base.parts.length + 1 + last]!;
    const slider = control(unitEq, MixerField.EqMidQ).querySelector('[role="slider"]')!;
    expect(slider.getAttribute("aria-valuenow")).toBe(String(base.drumCompEq.units[last]!.eqMidQ));
    const master = eqs[eqs.length - 1]!;
    expect(control(master, MixerField.EqLowGain).querySelector('[role="slider"]')!.getAttribute("aria-valuenow"))
      .toBe(String(base.masterEq.lowGain));
  });

  it("writes a turned EQ knob in its strip's Scope", () => {
    const { base, eqs, setMixerField } = loaded();
    const parts = base.parts.length;
    const unit = (i: number) => base.drumCompEq.units[i]!;

    stepUp(control(eqs[3]!, MixerField.EqMidGain));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Part, 3, MixerField.EqMidGain, base.parts[3]!.eq.midGain + 1,
    );
    stepUp(control(eqs[parts + 1 + 2]!, MixerField.EqHighGain));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 2, MixerField.EqHighGain, unit(2).eqHighGain + 1,
    );
    stepUp(control(eqs[eqs.length - 1]!, MixerField.EqLowGain));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Master, 0, MixerField.EqLowGain, base.masterEq.lowGain + 1,
    );
  });

  it("writes each strip's EQ and compressor switches in its own scope", () => {
    const { el, base, eqs, setMixerField } = loaded();
    const parts = base.parts.length;
    const unit = (i: number) => base.drumCompEq.units[i]!;
    const eqSwitch = (eq: HTMLElement) => control(eq, MixerField.EqSwitch).querySelector("button")!;

    eqSwitch(eqs[2]!).click();
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Part, 2, MixerField.EqSwitch, 0);
    eqSwitch(eqs[parts + 1 + 1]!).click();
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 1, MixerField.EqSwitch, unit(1).eqSwitch ? 0 : 1,
    );
    eqSwitch(eqs[eqs.length - 1]!).click();
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Master, 0, MixerField.EqSwitch, 0);
    [...el.querySelectorAll("button")].filter((b) => /^COMP (ON|OFF)$/.test(b.textContent ?? ""))[3]!.click();
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 3, MixerField.CompSwitch, unit(3).compSwitch ? 0 : 1,
    );
  });
});
