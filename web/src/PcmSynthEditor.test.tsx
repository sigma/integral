import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  WasmDeviceState,
  pcms_common_address,
  pcms_partial_address,
} from "../pkg/integral_wasm.js";
import { PcmSynthEditor } from "./PcmSynthEditor";
import { PmtStrip } from "./pcm-synth/PmtStrip";
import { cleanup, render } from "./test/dom";
import { arrowUp, fakeToneService, slider } from "./test/toneService";

// jsdom lacks the ResizeObserver the page's zoom measures with.
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    },
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

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

  it("labels switch and select options from the core field specs", () => {
    const pmt = { structureType12: 0, booster12: 0, structureType34: 0, booster34: 0, pmtVelocityControl: 0, partialEntries: [] };
    const { container } = render(<PmtStrip pmt={pmt} onChange={() => {}} />);
    // The first select is Structure 1 & 2; it read "1: Independent"… from a TS table.
    const structure = container.querySelector("select")!;
    expect([...structure.querySelectorAll("option")].map((o) => o.textContent)).toEqual(
      ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"],
    );
  });
});
