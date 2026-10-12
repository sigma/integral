import { afterEach, describe, expect, it } from "vitest";
import { PcmsPartialField, pcms_partial_spec } from "../../pkg/integral_wasm.js";
import { LfoScope } from "./LfoScope";
import { LFO_SHAPE_NAMES } from "../sn-synth/types";
import { switchOptions } from "../fieldSpec";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathOf(shape: string, rate = 0, steps?: number[], smooth?: boolean): string | null {
  const d = render(<LfoScope shape={shape} rate={rate} steps={steps} smooth={smooth} />)
    .container.querySelector("path")?.getAttribute("d") ?? null;
  cleanup();
  return d;
}

describe("LfoScope", () => {
  it("draws every SN-S LFO shape", () => {
    for (const shape of LFO_SHAPE_NAMES) expect(pathOf(shape), shape).toBeTruthy();
  });

  it("draws every PCM-S LFO waveform, STEP from its pattern", () => {
    const steps = Array.from({ length: 16 }, (_, i) => 28 + i * 4);
    for (const { label } of switchOptions(pcms_partial_spec(PcmsPartialField.Lfo1Waveform))) {
      expect(pathOf(label, 0, steps), label).toBeTruthy();
    }
    expect(pathOf("STEP", 0, steps, true)).not.toEqual(pathOf("STEP", 0, steps, false));
  });

  it("fits more cycles as the rate rises", () => {
    expect(pathOf("SIN", 127)).not.toEqual(pathOf("SIN", 0));
    // except CHAOS, whose rate the device ignores
    expect(pathOf("CHS", 127)).toEqual(pathOf("CHS", 0));
  });
});
