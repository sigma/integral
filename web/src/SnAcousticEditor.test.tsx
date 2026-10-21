import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  sna_common_address,
  sna_inst_params,
  sna_common_spec,
  ParameterFamily,
  SnaCommonField,
} from "../pkg/integral_wasm.js";
import { SnAcousticEditor } from "./SnAcousticEditor";
import { familyVar } from "./synth-ui";
import { cleanup, click, render } from "./test/dom";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

describe("SnAcousticEditor", () => {
  it("sends a Common and a Modify Parameter edit to their fields", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const common0 = [...sna_common_address(PART)];
    const base = service.requestData.bind(service);
    // Pure Vintage EP1: Inst Variation 0, Inst Number 4 (Common `00 21`).
    service.requestData = async (address: number[], size: number[]) => {
      const data = await base(address, size);
      if (address.every((b, i) => b === common0[i])) data[0x21] = 4;
      return data;
    };
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // Its first Modify Parameter is Noise Level, at slot 1.
    const params = sna_inst_params(0, 4) as [number, string][];
    const firstParam = params[0]?.[1] ?? "";
    expect(firstParam).toBe("Noise Level");
    arrowUp(slider(container, "Cutoff"));
    arrowUp(slider(container, firstParam));

    // Cutoff Offset is Common `00 13`; Modify Parameter 1 is `00 22`.
    const common = [...sna_common_address(PART).slice(0, 3)];
    expect(sentDt1s()).toEqual([
      { address: [...common, 19], data: [1] },
      { address: [...common, 34], data: [1] },
    ]);
  });

  it("disables the offsets the selected instrument ignores (TW Organ)", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const common = [...sna_common_address(PART)];
    const base = service.requestData.bind(service);
    // TW Organ: Inst Variation 65, Inst Number 0 (Common `00 20`, `00 21`).
    service.requestData = async (address: number[], size: number[]) => {
      const data = await base(address, size);
      if (address.every((b, i) => b === common[i])) data[0x20] = 65;
      return data;
    };
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    const inert = (label: string) => slider(container, label).closest("[inert]") !== null;
    expect(inert("Cutoff")).toBe(true);
    expect(inert("Atk")).toBe(true);
    expect(inert("Port")).toBe(true);
    expect(inert("Level")).toBe(false);
  });

  it("switches instrument by writing Inst Variation and Inst Number", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // The picker's current entry, then Concert Grand (list entry 1) in its popover.
    const current = [...container.querySelectorAll("button")].find((b) => /^\d{4} /.test(b.textContent ?? ""))!;
    click(current);
    const grand = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.startsWith("0001"),
    )!;
    click(grand);

    // Concert Grand: Inst Variation 64 (Common `00 20`), Inst Number 0 (`00 21`),
    // then its Modify Parameters' defaults (slot n is Common `00 21` + n).
    const common = [...sna_common_address(PART).slice(0, 3)];
    const defaults = (sna_inst_params(64, 0) as [number, string, number, number, number][]).map(
      ([slot, , , , def]) => ({ address: [...common, 0x21 + slot], data: [def] }),
    );
    expect(defaults.length).toBeGreaterThan(0);
    expect(sentDt1s()).toEqual([
      { address: [...common, 0x20], data: [64] },
      { address: [...common, 0x21], data: [0] },
      ...defaults,
    ]);
  });

  it("draws every tone Control on the Grid, in a Grid Section", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // The MFX placement hosts MfxPanel, a CSS Section with its own selects and send knobs.
    const controls = [...container.querySelectorAll('[role="slider"], button')].filter(
      (el) => el.closest('[data-section="mfx"]') === null,
    );
    const outside = controls.filter((el) => el.closest("[data-grid-section]") === null);
    // Every tone field (11) has its Control; only the instrument picker, which
    // writes two fields, stays off the Grid.
    expect(controls.length).toBe(12);
    expect(outside.map((el) => el.textContent)).toEqual(["0000 (unlisted)"]);
    expect(container.querySelector('[data-grid-screen="SN-A editor"]')).not.toBeNull();
  });

  it("keeps every Section's Cells when the instrument changes", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const common0 = [...sna_common_address(PART)];
    const base = service.requestData.bind(service);
    // The device's instrument (Common `00 20`, `00 21`): Pure Vintage EP1 first.
    let inst: [number, number] = [0, 4];
    service.requestData = async (address: number[], size: number[]) => {
      const data = await base(address, size);
      if (address.every((b, i) => b === common0[i])) [data[0x20], data[0x21]] = inst;
      return data;
    };
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    /** Each placed Section's box, and the size of what it draws. */
    const boxes = () =>
      [...container.querySelectorAll<HTMLElement>("[data-section]")].map((el) => {
        const drawn = el.firstElementChild as HTMLElement | null;
        return [el.dataset.section, el.style.cssText, drawn?.style.width, drawn?.style.height];
      });
    const before = boxes();
    const instTitle = () =>
      container.querySelector('[data-section="inst"] [data-grid-section]')?.getAttribute("data-grid-section");
    const generic = instTitle();
    expect(slider(container, "Noise Level")).toBeTruthy();

    // Concert Grand (list entry 1): the Ac.Piano page, by hand.
    const current = [...container.querySelectorAll("button")].find((b) => /^\d{4} /.test(b.textContent ?? ""))!;
    click(current);
    const grand = [...document.body.querySelectorAll('[role="option"]')].find((o) =>
      o.textContent?.startsWith("0001"),
    )!;
    inst = [64, 0];
    click(grand);
    await act(async () => {});

    expect(instTitle()).toBe("Ac.Piano");
    expect(instTitle()).not.toBe(generic);
    expect(slider(container, "String Res")).toBeTruthy();
    expect(boxes()).toEqual(before);
  });

  it("lights a knob in its field's family, not its section's", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // Octave Shift sits in COMMON (Common) but its spec says Pitch.
    const family = sna_common_spec(SnaCommonField.OctaveShift).family;
    expect(family).toBe(ParameterFamily.Pitch);
    const fills = [...slider(container, "Oct").querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain(familyVar(family));
  });
});
