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
import { controlsOffGrid, drumKey } from "./test/drum";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

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
});
