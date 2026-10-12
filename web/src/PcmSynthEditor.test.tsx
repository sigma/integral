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

  it("takes knob ranges from the core field specs", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(
      <PcmSynthEditor partIndex={PART} service={service} />,
    );
    await act(async () => {});

    const range = (label: string) => {
      const s = slider(container, label);
      return [s.getAttribute("aria-valuemin"), s.getAttribute("aria-valuemax")].map(Number);
    };
    // Each disagreed with the MIDI Implementation while ranges lived in TSX.
    expect(range("Up")).toEqual([0, 48]);
    expect(range("Bias Lvl")).toEqual([54, 74]);
    expect(range("1")).toEqual([28, 100]);
    expect(range("Random")).toEqual([0, 30]);
  });
});
