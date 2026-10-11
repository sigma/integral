import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  sna_common_address,
  sna_inst_params_by_category,
} from "../pkg/integral_wasm.js";
import { SnAcousticEditor } from "./SnAcousticEditor";
import { cleanup, render } from "./test/dom";
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
});
