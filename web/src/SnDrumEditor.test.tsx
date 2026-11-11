import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  snd_common_address,
  snd_note_address,
  snd_note_spec,
  ParameterFamily,
  SndNoteField,
  sn_drum_inst_supports_variation,
} from "../pkg/integral_wasm.js";
import { SnDrumEditor } from "./SnDrumEditor";
import { familyVar } from "./synth-ui";
import { cleanup, click, render, typeInto } from "./test/dom";
import { controlsOffGrid, drumKey, kitPlacement } from "./test/drum";
import { arrowUp, fakeToneService, flakyToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

describe("SnDrumEditor", () => {
  it("sends a Common and a Note edit to their fields", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    // The editor opens on key 36 (C2, kick drum): the Note edit below goes there.
    const key = 36;

    arrowUp(slider(container, "Amb Lv"));
    arrowUp(slider(container, "Attack"));

    // Ambience Level is Common `00 11`; Attack is Note `00 0C`.
    expect(sentDt1s()).toEqual([
      { address: [...snd_common_address(PART).slice(0, 3), 17], data: [1] },
      { address: [...snd_note_address(PART, key).slice(0, 3), 12], data: [1] },
    ]);
  });

  it("lights a knob in its field's family, not its section's", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // Stereo Width sits in TONE (Pitch) but its spec says Time.
    const family = snd_note_spec(SndNoteField.StereoWidth).family;
    expect(family).toBe(ParameterFamily.Time);
    const fills = [...slider(container, "Width").querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain(familyVar(family));
  });

  it("places the KIT Section beside the key matrix, out of the key Sections", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    expect(kitPlacement(container, 36)).toEqual({ besideKeys: true, inKeyScreen: false });
  });

  it("lays every key Control out on the Grid", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    expect(container.querySelector('[data-grid-screen="SN Drum"]')).not.toBeNull();
    expect(controlsOffGrid(container)).toEqual([]);
  });

  it("rebinds the Note Sections to another key without remounting them", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    const attack = slider(container, "Attack");
    const tone = attack.closest("[data-grid-section]");

    click(drumKey(container, 38));
    await act(async () => {});

    // The same Section and knob, now writing key 38's Attack (`00 0C`).
    expect(slider(container, "Attack")).toBe(attack);
    expect(attack.closest("[data-grid-section]")).toBe(tone);
    arrowUp(attack);
    expect(sentDt1s()).toEqual([{ address: [...snd_note_address(PART, 38).slice(0, 3), 12], data: [1] }]);
  });

  it("picks the key's Inst by name and writes its number", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    const inst = container.querySelector<HTMLElement>(`[data-grid-section="TONE"] [data-field="${SndNoteField.InstNumber}"]`)!;
    // The zeroed kit's Inst is 0: core names it OFF.
    const current = inst.querySelector("button")!;
    expect(current.textContent).toBe("OFF");

    click(current);
    typeInto(document.querySelector("input")!, "studio kick");
    click([...document.querySelectorAll<HTMLElement>('[role="option"]')].find((o) => o.textContent === "Studio Kick")!);

    // Inst Number is Note `00 00`, four nibbles: Inst 1 on key 36.
    expect(sentDt1s()).toEqual([{ address: [...snd_note_address(PART, 36).slice(0, 3), 0], data: [0, 0, 0, 1] }]);
    expect(current.textContent).toBe("Studio Kick");
  });

  it("lists only the Variations the key's Inst supports", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // The zeroed kit's Inst is 0, which supports only OFF.
    const supported = [0, 1, 2, 3, 4, 5, 6, 7].filter((v) => sn_drum_inst_supports_variation(0, v));
    const list = container.querySelector(`[data-field="${SndNoteField.Variation}"] [role="radiogroup"]`);
    if (!list) throw new Error("no Variation list");
    expect(list.querySelectorAll('[role="radio"]')).toHaveLength(supported.length);
  });

  describe("for a Kit plugin's Binding", () => {
    it("edits only the kit when bound to the kit", async () => {
      const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
      const { container } = render(<SnDrumEditor partIndex={PART} service={service} scope="kit" />);
      await act(async () => {});

      // The kit's Controls, without the key matrix or the key Sections.
      expect(container.querySelector("[data-drum-matrix]")).toBeNull();
      expect(container.querySelector('[data-grid-screen="SN Drum"]')).toBeNull();
      arrowUp(slider(container, "Amb Lv"));
      expect(sentDt1s()).toEqual([{ address: [...snd_common_address(PART).slice(0, 3), 17], data: [1] }]);
      expect(container.querySelector('[role="slider"][aria-label="Attack"]')).toBeNull();
    });

    it("edits only that key when bound to a key, and cannot pick another", async () => {
      const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
      const { container } = render(<SnDrumEditor partIndex={PART} service={service} scope={{ key: 40 }} />);
      await act(async () => {});

      expect(container.querySelector("[data-drum-matrix]")).toBeNull();
      expect(container.querySelector('[role="slider"][aria-label="Amb Lv"]')).toBeNull();
      arrowUp(slider(container, "Attack"));
      expect(sentDt1s()).toEqual([{ address: [...snd_note_address(PART, 40).slice(0, 3), 12], data: [1] }]);
    });
  });

  describe("when the kit reads fail", () => {
    /** The editor's read error, if any. */
    const readError = (container: HTMLElement) => container.querySelector('[role="alert"]');

    /** The error banner's Retry button. */
    const retryButton = (container: HTMLElement) => {
      const button = [...container.querySelectorAll("button")].find((b) => b.textContent === "Retry");
      if (!button) throw new Error("no Retry button");
      return button;
    };

    it("keeps failing: shows the error and inert placeholders, not values", async () => {
      const { service } = flakyToneService(new WasmDeviceState(0x10), Infinity);
      const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
      await act(async () => {});

      expect(readError(container)?.textContent).toContain("Couldn't read");
      expect(retryButton(container)).toBeTruthy();
      // The kit's Controls stay inert placeholders at their spec defaults...
      const kitLv = slider(container, "Kit Lv");
      expect(kitLv.getAttribute("aria-valuenow")).toBe("127");
      expect(kitLv.closest("[data-field]")!.hasAttribute("inert")).toBe(true);
      // ...and the MFX panel, gated on its block, is absent.
      expect(container.textContent).not.toContain("MACROS");
    });

    it("recovers through Retry once the reads answer again", async () => {
      const { service } = flakyToneService(new WasmDeviceState(0x10), 1);
      const { container } = render(<SnDrumEditor partIndex={PART} service={service} />);
      await act(async () => {});
      expect(readError(container)).not.toBeNull();

      click(retryButton(container));
      await act(async () => {});

      expect(readError(container)).toBeNull();
      // The answered (zeroed) kit, not the spec defaults.
      expect(slider(container, "Kit Lv").getAttribute("aria-valuenow")).toBe("0");
      expect(slider(container, "Amb Lv").getAttribute("aria-valuenow")).toBe("0");
      // The MFX panel is back with its block.
      expect(container.textContent).toContain("MACROS");
    });
  });
});
