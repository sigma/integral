import { afterEach, describe, expect, it, vi } from "vitest";
import { MixerField, MixerScopeKind, ToneType, WasmDeviceState } from "../pkg/integral_wasm.js";
import { RoutingPage } from "./RoutingPage";
import { defaultMixerState, type MixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { cleanup, click, render } from "./test/dom";

afterEach(cleanup);

/** A core state set up by `setup`, read the way `useMixer` reads it. */
function stateOf(device: WasmDeviceState): MixerState {
  const read = device.readState() as Partial<MixerState>;
  return { ...defaultMixerState(), ...read, loading: false, routing: device.effectiveRouting() };
}

/** A core state with every Part on B, set up further by `setup`. */
function device(setup: (device: WasmDeviceState) => void = () => {}): WasmDeviceState {
  const d = new WasmDeviceState(0x10);
  for (let part = 0; part < 16; part++) {
    d.setMixerField(MixerScopeKind.Part, part, MixerField.PartOutputAssign, 1);
  }
  setup(d);
  return d;
}

/** The Routing page over `d`'s state, its writes and jumps recorded. */
function draw(d: WasmDeviceState) {
  const setMixerField = vi.fn();
  const selectPart = vi.fn();
  const onOpen = vi.fn();
  const sync = vi.fn();
  const mixer = (state: MixerState) => ({ state, setMixerField, selectPart, sync }) as unknown as UseMixerResult;
  const { container, rerender } = render(<RoutingPage mixer={mixer(stateOf(d))} onOpen={onOpen} />);
  return {
    container,
    setMixerField,
    selectPart,
    onOpen,
    sync,
    /** Draw the page again over `d`'s current state, as a mixer sync does. */
    redraw: () => rerender(<RoutingPage mixer={mixer(stateOf(d))} onOpen={onOpen} />),
  };
}

/** The row of the source labelled `name`. */
function row(container: HTMLElement, name: string): HTMLElement {
  const found = [...container.querySelectorAll<HTMLElement>("tbody tr")].find(
    (r) => r.querySelector("th [data-source-name]")?.textContent === name,
  );
  if (!found) throw new Error(`no row "${name}"`);
  return found;
}

/** The column headings, in order. */
const columns = (container: HTMLElement) =>
  [...container.querySelectorAll("thead th[data-column]")].map((th) => th.textContent);

/** The headings of the columns `r` has a mark in. */
function marks(container: HTMLElement, r: HTMLElement): (string | null)[] {
  const heads = columns(container);
  return [...r.querySelectorAll("td[data-column]")]
    .map((td, i) => (td.querySelector("[data-mark]") ? heads[i]! : null))
    .filter((h) => h !== null);
}

/** The chip in `scope` whose text starts with `text`. */
function chip(scope: HTMLElement, text: string): HTMLElement {
  const found = [...scope.querySelectorAll<HTMLElement>("[data-chip]")].find((c) =>
    c.textContent?.startsWith(text),
  );
  if (!found) throw new Error(`no "${text}" chip`);
  return found;
}

/** The stage switch labelled `label`. */
function stageSwitch(container: HTMLElement, label: string): HTMLElement {
  const found = [...container.querySelectorAll<HTMLElement>("[data-stage] button")].find(
    (b) => b.textContent === label,
  );
  if (!found) throw new Error(`no "${label}" switch`);
  return found;
}

describe("RoutingPage", () => {
  describe("a plain routing", () => {
    it("has a column per output, A (MIX) first", () => {
      const { container } = draw(device());
      expect(columns(container)).toEqual(["A (MIX)", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8"]);
    });

    it("marks where each source lands", () => {
      const d = device((d) => {
        d.setMixerField(MixerScopeKind.Part, 2, MixerField.PartOutputAssign, 11);
      });
      const { container } = draw(d);
      expect(marks(container, row(container, "Part 1"))).toEqual(["B"]);
      expect(marks(container, row(container, "Part 3"))).toEqual(["8"]);
      expect(marks(container, row(container, "Ext Part"))).toEqual(["A (MIX)"]);
    });

    it("says which rows sum on one jack", () => {
      const d = device((d) => {
        d.setMixerField(MixerScopeKind.Part, 0, MixerField.PartOutputAssign, 4); // 1, jack A-L
      });
      const { container } = draw(d);
      const mark = row(container, "Part 1").querySelector<HTMLElement>("[data-mark]")!;
      expect(mark.hasAttribute("data-sums")).toBe(true);
      expect(mark.title).toContain("Ext Part");
      const alone = row(container, "Part 3").querySelector<HTMLElement>("[data-mark]")!;
      expect(alone.title).toContain("Part 1"); // every Part but Part 1 is on B
    });

    it("shows each row's stages, the MFX by its type's name", () => {
      const d = device((d) => {
        d.changePartTone(0, 87, 64, 0); // a PCM Synth tone
        d.setMfxType(0, ToneType.PcmSynth, 0);
        d.changePartTone(1, 87, 64, 1);
        d.setMfxType(1, ToneType.PcmSynth, 1);
        d.setMixerField(MixerScopeKind.Part, 1, MixerField.PartChorusSend, 40);
      });
      const { container } = draw(d);
      expect(chip(row(container, "Part 1"), "MFX").textContent).toBe("MFX THRU");
      const part2 = row(container, "Part 2");
      expect(chip(part2, "MFX").textContent).not.toBe("MFX THRU");
      expect(chip(part2, "CHO").textContent).toBe("CHO 40");
    });

    it("links each chip to its editor", () => {
      const { container, selectPart, onOpen } = draw(device());
      click(chip(row(container, "Part 4"), "MFX"));
      expect(selectPart).toHaveBeenLastCalledWith(3);
      expect(onOpen).toHaveBeenLastCalledWith("toneEdit");
      click(chip(row(container, "Part 5"), "REV"));
      expect(selectPart).toHaveBeenLastCalledWith(4);
      expect(onOpen).toHaveBeenLastCalledWith("mixer");
    });

    it("switches the stages through the Mixer's setters", () => {
      const { container, setMixerField } = draw(device());
      click(stageSwitch(container, "Chorus"));
      click(stageSwitch(container, "Master EQ"));
      click(stageSwitch(container, "Motional Surround"));
      click(chip(row(container, "Part 2"), "EQ"));
      expect(setMixerField.mock.calls).toEqual([
        [MixerScopeKind.Common, 0, MixerField.ChorusSwitch, 0],
        [MixerScopeKind.Master, 0, MixerField.EqSwitch, 0],
        [MixerScopeKind.Common, 0, MixerField.SurroundSwitch, 1],
        [MixerScopeKind.Part, 1, MixerField.EqSwitch, 0],
      ]);
    });

    it("copies core's state on opening, for what other pages wrote", () => {
      expect(draw(device()).sync).toHaveBeenCalledOnce();
    });

    it("follows the mixer state", () => {
      const d = device();
      const { container, redraw } = draw(d);
      d.setMixerField(MixerScopeKind.Part, 0, MixerField.PartOutputAssign, 3);
      redraw();
      expect(marks(container, row(container, "Part 1"))).toEqual(["D"]);
    });
  });

  describe("a Part through Comp+EQ", () => {
    /** Part 10 on an SN Drum Kit, its Comp+EQ on, unit 1 to C and unit 2 back to the Part. */
    const compEq = () =>
      device((d) => {
        d.changePartTone(9, 88, 64, 0);
        d.setMfxType(9, ToneType.SnDrum, 1);
        d.setMixerField(MixerScopeKind.Common, 0, MixerField.DrumCompEqPart, 9);
        d.setMixerField(MixerScopeKind.Common, 0, MixerField.DrumCompEqSwitch, 1);
        d.setMixerField(MixerScopeKind.CompEqUnit, 0, MixerField.CompOutputAssign, 3);
        d.setMixerField(MixerScopeKind.CompEqUnit, 1, MixerField.CompOutputAssign, 0);
      });

    it("has a row per unit, landing on its Output Assign past the MFX and EQ", () => {
      const { container } = draw(compEq());
      const unit = row(container, "Comp+EQ 1");
      expect(marks(container, unit)).toEqual(["C"]);
      expect(chip(unit, "MFX").hasAttribute("data-skipped")).toBe(true);
      expect(chip(unit, "EQ").hasAttribute("data-skipped")).toBe(true);
    });

    it("links a unit's Comp+EQ chip to the Mixer, its Part selected", () => {
      const { container, selectPart, onOpen } = draw(compEq());
      click(chip(row(container, "Comp+EQ 1"), "COMP+EQ"));
      expect(selectPart).toHaveBeenLastCalledWith(9);
      expect(onOpen).toHaveBeenLastCalledWith("mixer");
    });

    it("lands a unit on PART where its Part does, through the MFX", () => {
      const { container } = draw(compEq());
      const unit = row(container, "Comp+EQ 2");
      expect(marks(container, unit)).toEqual(["B"]);
      expect(chip(unit, "MFX").hasAttribute("data-skipped")).toBe(false);
      expect(unit.querySelector("[data-assign]")?.textContent).toBe("PART");
    });

    it("switches the Drum Comp+EQ off", () => {
      const { container, setMixerField } = draw(compEq());
      click(stageSwitch(container, "Drum Comp+EQ"));
      expect(setMixerField).toHaveBeenLastCalledWith(MixerScopeKind.Common, 0, MixerField.DrumCompEqSwitch, 0);
    });
  });

  describe("Motional Surround ON", () => {
    const surround = () =>
      device((d) => d.setMixerField(MixerScopeKind.Common, 0, MixerField.SurroundSwitch, 1));

    it("says it overrides every Output Assign, and links to the Surround page", () => {
      const { container, onOpen } = draw(surround());
      const banner = container.querySelector<HTMLElement>('[role="status"]')!;
      expect(banner.textContent).toContain("overrides every Output Assign");
      click(banner.querySelector("button")!);
      expect(onOpen).toHaveBeenLastCalledWith("surround");
    });

    it("has the 2-channel mix and the 5.1 layout as columns", () => {
      const { container } = draw(surround());
      expect(columns(container)).toEqual(["2CH L", "2CH R", "L", "R", "C", "Ls", "Rs", "LFE"]);
      expect(marks(container, row(container, "Part 1"))).toEqual(columns(container));
    });

    it("strikes the Output Assigns through, linking them to the Surround page", () => {
      const { container, onOpen } = draw(surround());
      const assign = row(container, "Part 1").querySelector<HTMLElement>("[data-assign]")!;
      expect(assign.textContent).toBe("B");
      expect(assign.hasAttribute("data-overridden")).toBe(true);
      click(assign);
      expect(onOpen).toHaveBeenLastCalledWith("surround");
    });
  });
});
