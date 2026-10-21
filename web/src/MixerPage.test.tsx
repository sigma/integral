import { afterEach, describe, expect, it, vi } from "vitest";
import { MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { MixerPage } from "./MixerPage";
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

  it("writes each strip's EQ and compressor fields in its own scope", () => {
    const setMixerField = vi.fn();
    const base = defaultMixerState();
    const mixer = {
      state: { ...base, loading: false, drumCompEq: { ...base.drumCompEq, enabled: true } },
      setMixerField,
    } as unknown as UseMixerResult;
    const el = render(<MixerPage mixer={mixer} expansions={expansions} />).container;
    const buttons = (label: RegExp) =>
      [...el.querySelectorAll("button")].filter((b) => label.test(b.textContent ?? ""));
    // Strip order: the parts, Ext, the Comp+EQ units, then the master.
    const eqSwitches = buttons(/^EQ (ON|OFF)$/);
    const parts = base.parts.length;
    const unit = (i: number) => base.drumCompEq.units[i]!;

    eqSwitches[2]!.click();
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Part, 2, MixerField.EqSwitch, 0);
    eqSwitches[parts + 1 + 1]!.click();
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 1, MixerField.EqSwitch, unit(1).eqSwitch ? 0 : 1,
    );
    eqSwitches[eqSwitches.length - 1]!.click();
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Master, 0, MixerField.EqSwitch, 0);
    buttons(/^COMP (ON|OFF)$/)[3]!.click();
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 3, MixerField.CompSwitch, unit(3).compSwitch ? 0 : 1,
    );
  });
});
