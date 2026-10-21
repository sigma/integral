import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { MixerPage } from "./MixerPage";
import { EQ_LAYOUT } from "./layouts/mixer/eq";
import { CELL } from "./synth-ui/grid";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import { defaultMixerState, type MixerState } from "./types";
import { cleanup, click, render } from "./test/dom";
import { paramFormatter, paramMeta } from "./paramDefaults";

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

  /** A loaded mixer with the Comp+EQ units shown (or not), writing through `setMixerField`. */
  function loaded(patch: (state: MixerState) => MixerState = (st) => st, compEq = true) {
    const setMixerField = vi.fn();
    const selectPart = vi.fn();
    const base = defaultMixerState();
    const state = patch({ ...base, loading: false, drumCompEq: { ...base.drumCompEq, enabled: compEq } });
    const mixer = { state, setMixerField, selectPart } as unknown as UseMixerResult;
    const el = render(<MixerPage mixer={mixer} expansions={expansions} />).container;
    // Strip order: the parts, the Comp+EQ units, then the master (Ext has no EQ).
    const eqs = [...el.querySelectorAll<HTMLElement>('[data-grid-section="EQ"]')];
    return { el, base, state, eqs, setMixerField, selectPart };
  }

  /** The Control bound to `field` in one Section. */
  const control = (section: Element, field: MixerField): HTMLElement => {
    const found = section.querySelector<HTMLElement>(`[data-field="${field}"]`);
    if (!found) throw new Error(`no ${MixerField[field]} in the Section`);
    return found;
  };

  /** The placement `id` on the mixer screen. */
  const placed = (el: HTMLElement, id: string): HTMLElement => {
    const found = el.querySelector<HTMLElement>(`[data-section="${id}"]`);
    if (!found) throw new Error(`no placement ${id}`);
    return found;
  };

  /** Clicks the switch bound to `field` in placement `id`. */
  const press = (el: HTMLElement, id: string, field: MixerField) =>
    act(() => control(placed(el, id), field).querySelector("button")!.click());

  /** Turns a knob one step up from the keyboard. */
  const stepUp = (knob: HTMLElement) =>
    act(() => {
      knob.querySelector('[role="slider"]')!.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }),
      );
    });

  const value = (c: HTMLElement) => c.querySelector('[role="slider"]')!.getAttribute("aria-valuenow");

  it("draws the Grid EQ, every band, in each part, Comp+EQ and master strip", () => {
    const { base, eqs, el } = loaded();
    expect(eqs).toHaveLength(base.parts.length + base.drumCompEq.units.length + 1);
    expect(el.querySelector('[data-section="eq.ext"]')).toBeNull();
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
    const unitEq = eqs[base.parts.length + last]!;
    expect(value(control(unitEq, MixerField.EqMidQ))).toBe(String(base.drumCompEq.units[last]!.eqMidQ));
    const master = eqs[eqs.length - 1]!;
    expect(value(control(master, MixerField.EqLowGain))).toBe(String(base.masterEq.lowGain));
  });

  it("writes a turned EQ knob in its strip's Scope", () => {
    const { base, eqs, setMixerField } = loaded();
    const parts = base.parts.length;
    const unit = (i: number) => base.drumCompEq.units[i]!;

    stepUp(control(eqs[3]!, MixerField.EqMidGain));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Part, 3, MixerField.EqMidGain, base.parts[3]!.eq.midGain + 1,
    );
    stepUp(control(eqs[parts + 2]!, MixerField.EqHighGain));
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
    eqSwitch(eqs[parts + 1]!).click();
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 1, MixerField.EqSwitch, unit(1).eqSwitch ? 0 : 1,
    );
    eqSwitch(eqs[eqs.length - 1]!).click();
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Master, 0, MixerField.EqSwitch, 0);
    press(el, "strip.comp.3", MixerField.CompSwitch);
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 3, MixerField.CompSwitch, unit(3).compSwitch ? 0 : 1,
    );
  });

  it("titles each strip: its part, EX, its Comp+EQ unit, Master", () => {
    const { el } = loaded();
    const title = (id: string) => placed(el, id).querySelector("[data-grid-section]")!.getAttribute("data-grid-section");
    expect(title("strip.part.0")).toBe("1");
    expect(title("strip.part.15")).toBe("16");
    expect(title("strip.ext")).toBe("EX");
    expect(title("strip.comp.2")).toBe("C+E3");
    expect(title("strip.master")).toBe("Master");
  });

  it("writes a part's strip controls in its Scope", () => {
    const { el, base, setMixerField } = loaded();
    const strip = placed(el, "strip.part.4");
    stepUp(control(strip, MixerField.PartPan));
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Part, 4, MixerField.PartPan, base.parts[4]!.pan + 1);
    stepUp(control(strip, MixerField.PartChorusSend));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Part, 4, MixerField.PartChorusSend, base.parts[4]!.chorusSend + 1,
    );
    stepUp(control(strip, MixerField.PartReceiveChannel));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Part, 4, MixerField.PartReceiveChannel, base.parts[4]!.receiveChannel + 1,
    );
    expect(value(control(strip, MixerField.PartLevel))).toBe(String(base.parts[4]!.level));
    press(el, "strip.part.4", MixerField.PartMute);
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Part, 4, MixerField.PartMute, 1);
  });

  it.each([
    ["strip.part.4", MixerScopeKind.Part, 4, MixerField.PartOutputAssign],
    ["strip.comp.1", MixerScopeKind.CompEqUnit, 1, MixerField.CompOutputAssign],
  ] as const)("picks %s's output from core's names", (id, kind, index, field) => {
    const { el, setMixerField } = loaded();
    const picker = control(placed(el, id), field);
    const name = paramFormatter(field);
    const button = picker.querySelector("button")!;
    expect(button.textContent).toBe(name(paramMeta(field).defaultValue));
    click(button);
    const options = [...document.querySelectorAll<HTMLElement>('[role="option"]')];
    const { min, max } = paramMeta(field);
    expect(options.map((o) => o.textContent)).toEqual(Array.from({ length: max - min + 1 }, (_, i) => name(min + i)));
    click(options[2]!);
    expect(setMixerField).toHaveBeenLastCalledWith(kind, index, field, min + 2);
  });

  it("solos a part through the Studio Set's Solo Part, and un-solos it", () => {
    const { el, setMixerField } = loaded((st) => ({ ...st, soloPart: 3 }));
    press(el, "strip.part.4", MixerField.SoloPart);
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.SoloPart, 5);
    press(el, "strip.part.2", MixerField.SoloPart);
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.SoloPart, 0);
  });

  it("assigns the Drum Comp+EQ to a drum kit part, and switches it off from the assigned one", () => {
    const { el, base, setMixerField } = loaded((st) => ({
      ...st,
      drumCompEq: { ...st.drumCompEq, enabled: false },
      // Part 3 plays a PCM drum kit.
      parts: st.parts.map((p, i) => (i === 2 ? { ...p, toneBankMsb: 86 } : p)),
    }), false);
    const assign = (i: number) => control(placed(el, `strip.part.${i}`), MixerField.DrumCompEqPart);
    // Only a drum kit routes keys to the units.
    expect(assign(0).dataset.inactive).toBe("dim");
    expect(assign(2).dataset.inactive).toBeUndefined();
    press(el, "strip.part.2", MixerField.DrumCompEqPart);
    expect(setMixerField.mock.calls).toEqual([
      [MixerScopeKind.Common, 0, MixerField.DrumCompEqPart, 2],
      [MixerScopeKind.Common, 0, MixerField.DrumCompEqSwitch, 1],
    ]);
    expect(base.drumCompEq.part).not.toBe(2);

    const on = loaded();
    const assigned = on.base.drumCompEq.part;
    press(on.el, `strip.part.${assigned}`, MixerField.DrumCompEqPart);
    expect(on.setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.DrumCompEqSwitch, 0);
  });

  it("swaps the FX sends for the ambience send while surround is on", () => {
    const fx = placed(loaded().el, "strip.part.1");
    control(fx, MixerField.PartReverbSend);
    expect(fx.querySelector(`[data-field="${MixerField.SurroundAmbienceSend}"]`)).toBeNull();
    cleanup();

    const { el, state, setMixerField } = loaded((st) => ({ ...st, surround: { ...st.surround, enabled: true } }));
    const amb = placed(el, "strip.part.1");
    expect(amb.querySelector(`[data-field="${MixerField.PartChorusSend}"]`)).toBeNull();
    stepUp(control(amb, MixerField.SurroundAmbienceSend));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.Part, 1, MixerField.SurroundAmbienceSend, state.surround.parts[1]!.ambienceSend + 1,
    );
  });

  it("writes the Ext, master and Comp+EQ strips in their Scopes", () => {
    const { el, base, setMixerField } = loaded();
    press(el, "strip.ext", MixerField.ExtMute);
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Ext, 0, MixerField.ExtMute, 1);
    expect(value(control(placed(el, "strip.master"), MixerField.MasterLevel))).toBe(String(base.masterLevel));
    const unit = placed(el, "strip.comp.5");
    stepUp(control(unit, MixerField.CompAttack));
    expect(setMixerField).toHaveBeenLastCalledWith(
      MixerScopeKind.CompEqUnit, 5, MixerField.CompAttack, base.drumCompEq.units[5]!.compAttack + 1,
    );
  });

  it("dims a switched-off compressor's settings, not its switch", () => {
    const { el } = loaded((st) => ({
      ...st,
      drumCompEq: {
        ...st.drumCompEq,
        units: st.drumCompEq.units.map((u, i) => ({ ...u, compSwitch: i !== 1 })),
      },
    }));
    const unit = (i: number) => placed(el, `strip.comp.${i}`);
    expect(control(unit(1), MixerField.CompThreshold).dataset.inactive).toBe("dim");
    expect(control(unit(1), MixerField.CompOutputGain).dataset.inactive).toBe("dim");
    expect(control(unit(1), MixerField.CompSwitch).dataset.inactive).toBeUndefined();
    expect(control(unit(0), MixerField.CompThreshold).dataset.inactive).toBeUndefined();
  });

  it("dims the Output Assigns while surround routes the strips", () => {
    const out = (el: HTMLElement) => [
      control(placed(el, "strip.part.0"), MixerField.PartOutputAssign),
      control(placed(el, "strip.comp.0"), MixerField.CompOutputAssign),
    ];
    for (const c of out(loaded().el)) expect(c.dataset.inactive).toBeUndefined();
    cleanup();
    const { el } = loaded((st) => ({ ...st, surround: { ...st.surround, enabled: true } }));
    for (const c of out(el)) expect(c.dataset.inactive).toBe("dim");
  });

  it("draws the compressor's transfer curve and envelope as Displays", () => {
    const unit = placed(loaded().el, "strip.comp.0");
    for (const display of [DisplayKind.CompTransferCurve, DisplayKind.CompEnvelope]) {
      expect(unit.querySelector(`[data-display="${display}"] svg`)).not.toBeNull();
    }
  });

  it("shows the Comp+EQ unit strips only while the Comp+EQ is on", () => {
    expect(loaded(undefined, false).el.querySelector('[data-section^="strip.comp."]')).toBeNull();
  });

  it("names each strip up the side of its fader, and puts its category or role under it", () => {
    const { el } = loaded((st) => ({
      ...st,
      parts: st.parts.map((p, i) => (i === 2 ? { ...p, toneName: "Pulse Lead 1" } : p)),
    }));
    const name = (id: string) =>
      placed(el, id).querySelector(`[data-display="${DisplayKind.StripName}"]`)?.textContent;
    expect(name("strip.part.2")).toBe("Pulse Lead 1");
    expect(name("strip.ext")).toBe("External Input");
    expect(name("strip.comp.1")).toBe("Compressor 2");
    expect(name("strip.master")).toBe("Master");
    // The plate under the strip names the role, not the tone again.
    expect(placed(el, "name.part.2").textContent).not.toContain("Pulse Lead 1");
    expect(placed(el, "name.comp.1").textContent).toBe("C+E2");
  });

  it("draws the FX columns on the Grid, writing their strips in the Common Scope", () => {
    const { el, base, setMixerField } = loaded((st) => ({ ...st, reverb: { ...st.reverb, type: 3, level: 90 } }));
    expect(placed(el, "fx.chorus").querySelector('[data-grid-section="Chorus"]')).not.toBeNull();
    expect(placed(el, "strip.chorus").querySelector("[data-grid-section]")!.getAttribute("data-grid-section")).toBe("FX1");
    expect(placed(el, "strip.reverb").querySelector("[data-grid-section]")!.getAttribute("data-grid-section")).toBe("FX2");
    // The type's name runs up the fader.
    expect(placed(el, "strip.reverb").querySelector(`[data-display="${DisplayKind.StripName}"]`)!.textContent).toBe(
      paramFormatter(MixerField.ReverbType)(3),
    );
    const reverb = placed(el, "strip.reverb");
    expect(value(control(reverb, MixerField.ReverbLevel))).toBe("90");
    stepUp(control(reverb, MixerField.ReverbLevel));
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.ReverbLevel, 91);
    press(el, "strip.chorus", MixerField.ChorusSwitch);
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.ChorusSwitch, Number(!base.chorus.enabled));
    const types = control(placed(el, "strip.chorus"), MixerField.ChorusType).querySelectorAll("button");
    act(() => types[2]!.click());
    expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.ChorusType, 2);
    for (const field of [MixerField.ChorusOutputAssign, MixerField.ChorusOutputSelect]) {
      expect(control(placed(el, "strip.chorus"), field).querySelector("button")).not.toBeNull();
    }
  });

  it("selects a part from anywhere on its column", () => {
    const { el, selectPart } = loaded();
    act(() => {
      control(placed(el, "strip.part.6"), MixerField.PartPan).dispatchEvent(
        new Event("pointerdown", { bubbles: true }),
      );
    });
    expect(selectPart).toHaveBeenCalledWith(6);
  });

  it("leaves no CSS-positioned Control on the strips", () => {
    const { el } = loaded();
    const strips = el.querySelectorAll('[data-section^="eq."], [data-section^="strip."], [data-section^="name."]');
    expect(strips.length).toBeGreaterThan(0);
    for (const s of strips) {
      for (const c of s.querySelectorAll('button, select, input, [role="slider"]')) {
        expect(c.closest("[data-field]")).not.toBeNull();
      }
    }
  });
});
