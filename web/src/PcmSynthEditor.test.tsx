import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  WasmDeviceState,
  pcms_common_address,
  pcms_partial_address,
} from "../pkg/integral_wasm.js";
import { PcmSynthEditor } from "./PcmSynthEditor";
import { cleanup, render } from "./test/dom";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

describe("PcmSynthEditor", () => {
  it("sends a Common and a Partial edit to their fields", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(
      <PcmSynthEditor partIndex={PART} service={service} />,
    );
    await act(async () => {});

    const up = slider(container, "Up");
    const bendUp = Number(up.getAttribute("aria-valuenow"));
    arrowUp(up);
    arrowUp(slider(container, "FX1"));

    // Pitch Bend Range Up is Common `00 29`; Chorus Send is Partial `00 0F`.
    expect(sentDt1s()).toEqual([
      { address: [...pcms_common_address(PART).slice(0, 3), 41], data: [bendUp + 1] },
      { address: [...pcms_partial_address(PART, 0).slice(0, 3), 15], data: [1] },
    ]);
  });
});
