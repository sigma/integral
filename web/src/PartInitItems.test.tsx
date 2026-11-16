import { afterEach, describe, expect, it, vi } from "vitest";
import { PartSetting, PartSettingGroup, part_setting_name } from "../pkg/integral_wasm.js";
import { deviceSpec } from "./deviceSpec";
import { InitProvider } from "./InitFlow";
import { InitPartItem, SoundCtrlInitItem } from "./PartInitItems";
import { PartSettingsDrawer } from "./PartSettingsDrawer";
import { defaultMixerState, type MixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { button, cleanup, click, render } from "./test/dom";
import { fakeInitService } from "./test/initService";
import { slider } from "./test/toneService";

afterEach(cleanup);

/** A read mixer whose Part `part`, selected, holds Cutoff Offset `cutoff` and Key Lower `keyLower`. */
function mixerState(part: number, cutoff: number, keyLower: number): MixerState {
  const base = defaultMixerState();
  const settings = new Map([
    [part_setting_name(PartSetting.CutoffOffset), cutoff],
    [part_setting_name(PartSetting.KeyRangeLower), keyLower],
  ]);
  const parts = base.parts.map((p, i) => (i === part ? { ...p, toneName: "Grand Piano", settings } : p));
  return { ...base, loading: false, parts, selectedPart: part };
}

/** The drawer with its Init items, as `App` shows it, over a service whose Init answers on `finish`. */
function draw(state: MixerState) {
  const fake = fakeInitService();
  const tree = (s: MixerState) => (
    <InitProvider service={fake.service} mixer={{ reload: vi.fn() }}>
      <PartSettingsDrawer
        mixer={{ state: s, setPartSetting: vi.fn() } as unknown as UseMixerResult}
        service={fake.service}
        onClose={() => {}}
        headerActions={<InitPartItem part={s.selectedPart} />}
        groupActions={{ [PartSettingGroup.Offset]: <SoundCtrlInitItem /> }}
      />
    </InitProvider>
  );
  const { container, rerender } = render(tree(state));
  const group = (name: string) => container.querySelector<HTMLElement>(`[data-grid-section="${name}"]`)!;
  const inert = (name: string) => group(name).closest("[inert]") !== null;
  const dialog = () => container.querySelector('[role="dialog"]');
  return { ...fake, container, group, inert, dialog, show: (next: MixerState) => rerender(tree(next)) };
}

describe("the Part settings drawer's Init items", () => {
  it("Init Part… in the header resets the shown Part, then shows its re-read values", async () => {
    const d = draw(mixerState(2, 90, 36));
    const item = button(d.container.querySelector("header")!, "Init Part…");
    click(item);
    expect(d.dialog()?.getAttribute("aria-label")).toBe("Init Part 3");
    expect(d.dialog()?.textContent).toContain("This resets Part 3's settings.");
    expect(d.initialize).not.toHaveBeenCalled();

    click(button(d.container, "Init"));
    expect(d.initialize).toHaveBeenCalledWith({ kind: "part", part: 2 });
    // Every group of the Part's block is blocked until "finished", and the items with it.
    for (const name of ["KBD", "PITCH", "OFFSET", "MIDI"]) expect(d.inert(name)).toBe(true);
    // The Voice Reserve is Studio Set Common's: a Part Init leaves it.
    expect(d.inert("VOICE")).toBe(false);
    expect((item as HTMLButtonElement).disabled).toBe(true);

    await d.finish();
    expect(d.inert("KBD")).toBe(false);
    expect(d.reReadPartBlock).toHaveBeenCalledWith(2);
    // The re-read reaches the mixer's state, and the drawer shows it.
    d.show(mixerState(2, 64, 0));
    expect(slider(d.group("OFFSET"), "Cutoff").getAttribute("aria-valuenow")).toBe("64");
    expect(slider(d.group("KBD"), "Key Lower").getAttribute("aria-valuenow")).toBe("0");
  });

  it("Sound Ctrl Init… on OFFSET says it resets every Part, and blocks only the offsets", async () => {
    const parts = deviceSpec().part_count;
    const d = draw(mixerState(4, 90, 36));
    const offset = d.container.querySelector('[data-part-settings-group="Offset"]')!;
    click(button(offset, `Sound Ctrl Init (all ${parts} Parts)…`));
    expect(d.dialog()?.getAttribute("aria-label")).toBe("Init Sound Ctrl");
    expect(d.dialog()?.textContent).toContain(`the Sound Ctrl offsets of all ${parts} Parts.`);

    click(button(d.container, "Init"));
    expect(d.initialize).toHaveBeenCalledWith({ kind: "sound-ctrl" });
    expect(d.inert("OFFSET")).toBe(true);
    // The Part's other settings stay live.
    expect(d.inert("KBD")).toBe(false);
    expect(d.inert("MIDI")).toBe(false);

    await d.finish();
    expect(d.inert("OFFSET")).toBe(false);
    expect(d.reReadPart).toHaveBeenCalledTimes(parts);
    d.show(mixerState(4, 64, 36));
    expect(slider(d.group("OFFSET"), "Cutoff").getAttribute("aria-valuenow")).toBe("64");
  });

  it("follows the selected Part", () => {
    const d = draw(mixerState(0, 64, 0));
    d.show(mixerState(7, 64, 0));
    click(button(d.container.querySelector("header")!, "Init Part…"));
    expect(d.dialog()?.getAttribute("aria-label")).toBe("Init Part 8");
  });
});
