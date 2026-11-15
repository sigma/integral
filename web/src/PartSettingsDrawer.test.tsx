import { afterEach, describe, expect, it, vi } from "vitest";
import { PartSetting, PartSettingGroup, part_setting_name } from "../pkg/integral_wasm.js";
import { PartSettingsDrawer } from "./PartSettingsDrawer";
import { defaultMixerState, type MixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { button, cleanup, click, option, render } from "./test/dom";
import { arrowUp, slider } from "./test/toneService";

afterEach(cleanup);

/** A read mixer whose Part `part` holds `settings` (by field), selected. */
function mixerState(part: number, settings: Partial<Record<PartSetting, number>> = {}): MixerState {
  const base = defaultMixerState();
  const byName = new Map(
    Object.entries(settings).map(([field, value]) => [part_setting_name(Number(field) as PartSetting), value]),
  );
  const parts = base.parts.map((p, i) => (i === part ? { ...p, toneName: "Grand Piano", settings: byName } : p));
  return { ...base, loading: false, parts, selectedPart: part };
}

/** The drawer over `state`, its writes recorded. */
function draw(state: MixerState) {
  const setPartSetting = vi.fn();
  const onClose = vi.fn();
  const mixer = (s: MixerState) => ({ state: s, setPartSetting }) as unknown as UseMixerResult;
  const { container, rerender } = render(<PartSettingsDrawer mixer={mixer(state)} onClose={onClose} />);
  return {
    container,
    setPartSetting,
    onClose,
    show: (next: MixerState) => rerender(<PartSettingsDrawer mixer={mixer(next)} onClose={onClose} />),
  };
}

/** The drawer's group `name` (`KBD`). */
function group(container: HTMLElement, name: string): HTMLElement {
  const found = container.querySelector<HTMLElement>(`[data-grid-section="${name}"]`);
  if (!found) throw new Error(`no group "${name}"`);
  return found;
}

const value = (control: Element) => control.getAttribute("aria-valuenow");

describe("the Part settings drawer", () => {
  it("stacks the registry's groups in its order, and no SCALE yet (#600)", () => {
    const { container } = draw(mixerState(0));
    const titles = [...container.querySelectorAll("[data-grid-section]")].map((s) => s.getAttribute("data-grid-section"));
    expect(titles).toEqual(["KBD", "PITCH", "OFFSET", "MIDI", "VOICE"]);
  });

  it("is headed by the selected Part, its channel and its tone, and closes", () => {
    const { container, onClose } = draw(mixerState(2));
    expect(container.querySelector("header")?.textContent).toContain("Part 3 · ch 1 · Grand Piano");
    click(button(container, "Close"));
    expect(onClose).toHaveBeenCalled();
  });

  it("shows the values read from the device", () => {
    const { container } = draw(
      mixerState(0, { [PartSetting.KeyRangeLower]: 36, [PartSetting.CutoffOffset]: 80, [PartSetting.RxHold1]: 0 }),
    );
    const key = slider(group(container, "KBD"), "Key Lower");
    expect(value(key)).toBe("36");
    expect(key.closest("[data-field]")?.textContent).toContain("C2");
    expect(value(slider(group(container, "OFFSET"), "Cutoff"))).toBe("80");
    expect(group(container, "MIDI").querySelector('[title="Rx Hold-1"] button')?.textContent).toBe("OFF");
  });

  it("shows a field the read hasn't named at its default", () => {
    const { container } = draw(mixerState(0));
    expect(value(slider(group(container, "PITCH"), "Bend Range"))).toBe("25");
  });

  it("follows the selected Part", () => {
    const { container, show, setPartSetting } = draw(mixerState(0, { [PartSetting.VoiceReserve]: 4 }));
    expect(value(slider(group(container, "VOICE"), "Voice Reserve"))).toBe("4");
    show(mixerState(5, { [PartSetting.VoiceReserve]: 9 }));
    expect(container.querySelector("header")?.textContent).toContain("Part 6");
    const reserve = slider(group(container, "VOICE"), "Voice Reserve");
    expect(value(reserve)).toBe("9");
    arrowUp(reserve);
    expect(setPartSetting).toHaveBeenCalledWith(5, PartSetting.VoiceReserve, 10);
  });

  it("is inert until the mixer is read", () => {
    const { container } = draw({ ...mixerState(0), loading: true });
    expect(slider(group(container, "KBD"), "Key Lower").closest("[inert]")).not.toBeNull();
    expect(container.querySelector("header")?.textContent).toBe("Part 1✕");
  });

  describe("edits one field of each group", () => {
    it("KBD: a list", () => {
      const { container, setPartSetting } = draw(mixerState(1));
      const list = group(container, "KBD").querySelector('[role="radiogroup"][aria-label="Mono/Poly"]')!;
      click([...list.querySelectorAll('[role="radio"]')].find((r) => r.textContent === "MONO")!);
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.MonoPoly, 0);
    });

    it("KBD: a picker", () => {
      const { container, setPartSetting } = draw(mixerState(1));
      click(button(group(container, "KBD"), "Velo Curve: OFF"));
      click(option(document.body, "3"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VelocityCurve, 3);
    });

    it("PITCH: a knob", () => {
      const { container, setPartSetting } = draw(mixerState(1, { [PartSetting.CoarseTune]: 70 }));
      arrowUp(slider(group(container, "PITCH"), "Coarse"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.CoarseTune, 71);
    });

    it("OFFSET: a knob", () => {
      const { container, setPartSetting } = draw(mixerState(1));
      arrowUp(slider(group(container, "OFFSET"), "Vib Depth"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VibratoDepth, 65);
    });

    it("MIDI: a switch", () => {
      const { container, setPartSetting } = draw(mixerState(1));
      click(group(container, "MIDI").querySelector('[title="Rx PC"] button')!);
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.RxProgramChange, 0);
    });

    it("VOICE: a knob", () => {
      const { container, setPartSetting } = draw(mixerState(1));
      arrowUp(slider(group(container, "VOICE"), "Voice Reserve"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VoiceReserve, 1);
    });
  });

  it("holds the seams Init Part and Sound Ctrl Init will fill (#591)", () => {
    const state = mixerState(0);
    const mixer = { state, setPartSetting: vi.fn() } as unknown as UseMixerResult;
    const { container } = render(
      <PartSettingsDrawer
        mixer={mixer}
        onClose={() => {}}
        headerActions={<button>Init Part…</button>}
        groupActions={{ [PartSettingGroup.Offset]: <button>Sound Ctrl Init…</button> }}
      />,
    );
    expect(container.querySelector("header")?.textContent).toContain("Init Part…");
    const offset = container.querySelector('[data-part-settings-group="Offset"]')!;
    expect(offset.textContent).toContain("Sound Ctrl Init…");
  });
});
