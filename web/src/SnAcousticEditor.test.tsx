import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  sna_common_address,
  sna_inst_params_by_category,
} from "../pkg/integral_wasm.js";
import { SnAcousticEditor } from "./SnAcousticEditor";
import { cleanup, click, render } from "./test/dom";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

describe("SnAcousticEditor", () => {
  it("sends a Common and a Modify Parameter edit to their fields", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<SnAcousticEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // A zeroed Common is Category 0 (Ac.Piano); its first Modify Parameter.
    const params = sna_inst_params_by_category("Ac.Piano") as [number, string][];
    const firstParam = params[0]?.[1] ?? "";
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

    // Concert Grand: Inst Variation 64 (Common `00 20`), Inst Number 0 (`00 21`).
    const common = [...sna_common_address(PART).slice(0, 3)];
    expect(sentDt1s()).toEqual([
      { address: [...common, 0x20], data: [64] },
      { address: [...common, 0x21], data: [0] },
    ]);
  });
});
