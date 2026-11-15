import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  PartSetting,
  PartSettingGroup,
  SystemField,
  WasmDeviceState,
  part_scale_ignored_note,
  part_scale_switched_off_note,
  part_setting_name,
} from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { PartSettingsDrawer } from "./PartSettingsDrawer";
import { SystemPage } from "./SystemPage";
import { defaultMixerState, type MixerState, type PartState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { button, cleanup, click, option, render } from "./test/dom";
import { systemCommonDump } from "./test/system";
import { arrowUp, fakeToneService, sentRequests, slider } from "./test/toneService";

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

/** A fake device whose System Common reads with the Scale Tune Switch at `scaleTune` (ON by default). */
function systemService(scaleTune = 1): IntegraService {
  const { service } = fakeToneService(new WasmDeviceState(0x10));
  service.requestData = async () => systemCommonDump([[SystemField.ScaleTuneSwitch, scaleTune]]);
  return service;
}

/** Let the drawer's System Common read answer. */
const settle = () => act(async () => {});

/** The drawer over `state`, its writes recorded, once System Common is read through `service`. */
async function draw(state: MixerState, service = systemService()) {
  const setPartSetting = vi.fn();
  const onClose = vi.fn();
  const onOpenSystem = vi.fn();
  const mixer = (s: MixerState) => ({ state: s, setPartSetting }) as unknown as UseMixerResult;
  const drawer = (s: MixerState) => (
    <PartSettingsDrawer mixer={mixer(s)} service={service} onClose={onClose} onOpenSystem={onOpenSystem} />
  );
  const { container, rerender } = render(drawer(state));
  await settle();
  return {
    container,
    setPartSetting,
    onClose,
    onOpenSystem,
    show: (next: MixerState) => rerender(drawer(next)),
  };
}

/** Part `part` of a real core, selected: the drawer's edits go to it, and `show` draws what it holds. */
async function drawDevice(part: number) {
  const device = new WasmDeviceState(0x10);
  const state = (): MixerState => ({
    ...mixerState(part),
    parts: (device.readState() as { parts: PartState[] }).parts,
  });
  const mixer = (): UseMixerResult =>
    ({
      state: state(),
      setPartSetting: (p: number, field: PartSetting, value: number) => device.setPartSetting(p, field, value),
    }) as unknown as UseMixerResult;
  const service = systemService();
  const drawer = () => <PartSettingsDrawer mixer={mixer()} service={service} onClose={() => {}} />;
  const { container, rerender } = render(drawer());
  await settle();
  return { device, container, show: () => rerender(drawer()) };
}


/** The drawer's group `name` (`KBD`). */
function group(container: HTMLElement, name: string): HTMLElement {
  const found = container.querySelector<HTMLElement>(`[data-grid-section="${name}"]`);
  if (!found) throw new Error(`no group "${name}"`);
  return found;
}

const value = (control: Element) => control.getAttribute("aria-valuenow");

describe("the Part settings drawer", () => {
  it("stacks the registry's groups in its order", async () => {
    const { container } = await draw(mixerState(0));
    const titles = [...container.querySelectorAll("[data-grid-section]")].map((s) => s.getAttribute("data-grid-section"));
    expect(titles).toEqual(["KBD", "PITCH", "OFFSET", "MIDI", "VOICE", "SCALE"]);
  });

  it("is headed by the selected Part, its channel and its tone, and closes", async () => {
    const { container, onClose } = await draw(mixerState(2));
    expect(container.querySelector("header")?.textContent).toContain("Part 3 · ch 1 · Grand Piano");
    click(button(container, "Close Part settings"));
    expect(onClose).toHaveBeenCalled();
  });

  it("shows the values read from the device", async () => {
    const { container } = await draw(
      mixerState(0, { [PartSetting.KeyRangeLower]: 36, [PartSetting.CutoffOffset]: 80, [PartSetting.RxHold1]: 0 }),
    );
    const key = slider(group(container, "KBD"), "Key Lower");
    expect(value(key)).toBe("36");
    expect(key.closest("[data-field]")?.textContent).toContain("C2");
    expect(value(slider(group(container, "OFFSET"), "Cutoff"))).toBe("80");
    expect(group(container, "MIDI").querySelector('[title="Rx Hold-1"] button')?.textContent).toBe("OFF");
  });

  it("shows a field the read hasn't named at its default", async () => {
    const { container } = await draw(mixerState(0));
    expect(value(slider(group(container, "PITCH"), "Bend Range"))).toBe("25");
  });

  it("follows the selected Part", async () => {
    const { container, show, setPartSetting } = await draw(mixerState(0, { [PartSetting.VoiceReserve]: 4 }));
    expect(value(slider(group(container, "VOICE"), "Voice Reserve"))).toBe("4");
    show(mixerState(5, { [PartSetting.VoiceReserve]: 9 }));
    expect(container.querySelector("header")?.textContent).toContain("Part 6");
    const reserve = slider(group(container, "VOICE"), "Voice Reserve");
    expect(value(reserve)).toBe("9");
    arrowUp(reserve);
    expect(setPartSetting).toHaveBeenCalledWith(5, PartSetting.VoiceReserve, 10);
  });

  it("is inert until the mixer is read", async () => {
    const { container } = await draw({ ...mixerState(0), loading: true });
    expect(slider(group(container, "KBD"), "Key Lower").closest("[inert]")).not.toBeNull();
    expect(container.querySelector("header")?.textContent).toBe("Part 1✕");
  });

  describe("edits one field of each group", () => {
    it("KBD: a list", async () => {
      const { container, setPartSetting } = await draw(mixerState(1));
      const list = group(container, "KBD").querySelector('[role="radiogroup"][aria-label="Mono/Poly"]')!;
      click([...list.querySelectorAll('[role="radio"]')].find((r) => r.textContent === "MONO")!);
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.MonoPoly, 0);
    });

    it("KBD: a picker", async () => {
      const { container, setPartSetting } = await draw(mixerState(1));
      click(button(group(container, "KBD"), "Velo Curve: OFF"));
      click(option(document.body, "3"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VelocityCurve, 3);
    });

    it("PITCH: a knob", async () => {
      const { container, setPartSetting } = await draw(mixerState(1, { [PartSetting.CoarseTune]: 70 }));
      arrowUp(slider(group(container, "PITCH"), "Coarse"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.CoarseTune, 71);
    });

    it("OFFSET: a knob", async () => {
      const { container, setPartSetting } = await draw(mixerState(1));
      arrowUp(slider(group(container, "OFFSET"), "Vib Depth"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VibratoDepth, 65);
    });

    it("MIDI: a switch", async () => {
      const { container, setPartSetting } = await draw(mixerState(1));
      click(group(container, "MIDI").querySelector('[title="Rx PC"] button')!);
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.RxProgramChange, 0);
    });

    it("VOICE: a knob", async () => {
      const { container, setPartSetting } = await draw(mixerState(1));
      arrowUp(slider(group(container, "VOICE"), "Voice Reserve"));
      expect(setPartSetting).toHaveBeenCalledWith(1, PartSetting.VoiceReserve, 1);
    });
  });

  it("holds the seams Init Part and Sound Ctrl Init will fill (#591)", async () => {
    const state = mixerState(0);
    const mixer = { state, setPartSetting: vi.fn() } as unknown as UseMixerResult;
    const { container } = render(
      <PartSettingsDrawer
        mixer={mixer}
        service={systemService()}
        onClose={() => {}}
        headerActions={<button>Init Part…</button>}
        groupActions={{ [PartSettingGroup.Offset]: <button>Sound Ctrl Init…</button> }}
      />,
    );
    expect(container.querySelector("header")?.textContent).toContain("Init Part…");
    const offset = container.querySelector('[data-part-settings-group="Offset"]')!;
    expect(offset.textContent).toContain("Sound Ctrl Init…");
    await settle();
  });

  describe("SCALE (#600)", () => {
    /** The offsets' knobs, C to B. */
    const NOTES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    /** A knob's top on the Grid, px. */
    const top = (control: Element) => parseFloat((control.closest("[data-field]") as HTMLElement).style.top);

    it("draws Type, Key and the 12 offsets as one octave, the black keys above", async () => {
      const { container } = await draw(mixerState(0, { [PartSetting.ScaleTuneCSharp]: 80 }));
      const scale = group(container, "SCALE");
      expect(button(scale, "Scale Type: EQUAL")).toBeDefined();
      expect(button(scale, "Scale Key: C")).toBeDefined();
      const knobs = NOTES.map((note) => slider(scale, note));
      expect(value(knobs[1]!)).toBe("80");
      expect(knobs[1]!.closest("[data-field]")?.textContent).toContain("+16");
      const black = new Set(["C#", "D#", "F#", "G#", "A#"]);
      const tops = new Set(knobs.filter((_, i) => black.has(NOTES[i]!)).map(top));
      const whiteTops = new Set(knobs.filter((_, i) => !black.has(NOTES[i]!)).map(top));
      expect(tops.size).toBe(1);
      expect(whiteTops.size).toBe(1);
      expect([...tops][0]!).toBeLessThan([...whiteTops][0]!);
    });

    it("edits the Type, the Key and an offset", async () => {
      const { container, setPartSetting } = await draw(mixerState(3));
      const scale = group(container, "SCALE");
      click(button(scale, "Scale Type: EQUAL"));
      click(option(document.body, "WERCKMEIS"));
      expect(setPartSetting).toHaveBeenCalledWith(3, PartSetting.ScaleTuneType, 7);
      arrowUp(slider(scale, "G#"));
      expect(setPartSetting).toHaveBeenCalledWith(3, PartSetting.ScaleTuneGSharp, 65);
    });

    it("shows Type CUSTOM once an offset is written, as the device sets it", async () => {
      const { device, container, show } = await drawDevice(15);
      arrowUp(slider(group(container, "SCALE"), "E"));
      show();
      const scale = group(container, "SCALE");
      expect(button(scale, "Scale Type: CUSTOM")).toBeDefined();
      expect(value(slider(scale, "E"))).toBe("65");
      device.free();
    });

    it("shows the offsets the device recomputes after a Type write", async () => {
      const { device, container, show } = await drawDevice(15);
      click(button(group(container, "SCALE"), "Scale Type: EQUAL"));
      click(option(document.body, "JUST-MAJ"));
      // The device fills C … B with the preset and says nothing: core reads them again.
      const reread = sentRequests(device);
      expect(reread).toHaveLength(1);
      const { address, size } = reread[0]!;
      const data = new Uint8Array(size).fill(1);
      // Type JUST-MAJ, Key C, then C … B (docs/midi/99-undocumented.md §11).
      data.set([2, 0, 0x50, 0x32, 0x54, 0x5f, 0x42, 0x4e, 0x30, 0x52, 0x34, 0x40, 0x61, 0x44]);
      device.handleDt1(address, data, 100_000);
      show();
      const scale = group(container, "SCALE");
      expect(button(scale, "Scale Type: JUST-MAJ")).toBeDefined();
      expect(value(slider(scale, "C"))).toBe(String(0x50));
      expect(value(slider(scale, "B"))).toBe(String(0x44));
      device.free();
    });

    it("shows the offsets the device recomputes after a Key write", async () => {
      const { device, container, show } = await drawDevice(15);
      click(button(group(container, "SCALE"), "Scale Key: C"));
      click(option(document.body, "G"));
      const reread = sentRequests(device);
      expect(reread).toHaveLength(1);
      const { address, size } = reread[0]!;
      const data = new Uint8Array(size).fill(1);
      // Type JUST-MAJ, Key G, then C … B (docs/midi/99-undocumented.md §11).
      data.set([2, 7, 0x3a, 0x1c, 0x3e, 0x20, 0x2c, 0x4d, 0x30, 0x3c, 0x1e, 0x40, 0x4b, 0x2e]);
      device.handleDt1(address, data, 100_000);
      show();
      const scale = group(container, "SCALE");
      expect(button(scale, "Scale Key: G")).toBeDefined();
      expect(value(slider(scale, "C"))).toBe(String(0x3a));
      device.free();
    });

    it("shows the System Scale Tune Switch, and links to the System page's Tuning", async () => {
      const { container, onOpenSystem } = await draw(mixerState(0));
      const scale = container.querySelector('[data-part-settings-group="Scale"]')!;
      expect(scale.textContent).toContain("System Scale Tune Switch: ON");
      expect(scale.textContent).not.toContain(part_scale_switched_off_note());
      expect(group(container, "SCALE").querySelector("[data-inactive]")).toBeNull();
      click(button(scale, "System Tuning…"));
      expect(onOpenSystem).toHaveBeenCalled();
    });

    it("dims while the System Scale Tune Switch is OFF, and stays editable", async () => {
      const { container, setPartSetting } = await draw(mixerState(2), systemService(0));
      const scale = container.querySelector('[data-part-settings-group="Scale"]')!;
      expect(scale.textContent).toContain("System Scale Tune Switch: OFF");
      expect(scale.textContent).toContain(part_scale_switched_off_note());
      const c = slider(group(container, "SCALE"), "C");
      expect(c.closest("[data-inactive]")?.getAttribute("data-inactive")).toBe("dim");
      arrowUp(c);
      expect(setPartSetting).toHaveBeenCalledWith(2, PartSetting.ScaleTuneC, 65);
      // The other groups don't depend on it.
      expect(slider(group(container, "KBD"), "Key Lower").closest("[data-inactive]")).toBeNull();
    });

    it("follows the switch the System page edits beside it", async () => {
      const service = systemService(1);
      const { container } = await draw(mixerState(0), service);
      const page = render(<SystemPage service={service} />).container;
      await settle();
      const scale = container.querySelector('[data-part-settings-group="Scale"]')!;
      expect(scale.textContent).not.toContain(part_scale_switched_off_note());
      click(page.querySelector(`[data-field="scale-tune-switch"] button`)!);
      expect(scale.textContent).toContain(part_scale_switched_off_note());
    });

    it("says when the Part's tone ignores the scale, and stays editable", async () => {
      const notes = (state: MixerState) =>
        draw(state).then(({ container, setPartSetting }) => ({
          text: container.querySelector('[data-part-settings-group="Scale"]')!.textContent,
          container,
          setPartSetting,
        }));
      const tone = (toneBankMsb: number, toneCategory: number): MixerState => {
        const state = mixerState(1);
        const parts = state.parts.map((p, i) => (i === 1 ? { ...p, toneBankMsb, toneCategory } : p));
        return { ...state, parts };
      };
      const IGNORED = part_scale_ignored_note();
      const kit = await notes(tone(88, 46)); // an SN Drum Kit
      expect(kit.text).toContain(IGNORED);
      arrowUp(slider(group(kit.container, "SCALE"), "C"));
      expect(kit.setPartSetting).toHaveBeenCalledWith(1, PartSetting.ScaleTuneC, 65);
      cleanup();
      expect((await notes(tone(89, 6))).text).toContain(IGNORED); // an SN-A Organ
      cleanup();
      expect((await notes(tone(87, 6))).text).not.toContain(IGNORED); // a PCM organ
    });
  });
});
