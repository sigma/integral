import { afterEach, describe, expect, it } from "vitest";
import { WasmDeviceState } from "../pkg/integral_wasm.js";
import { ToneEditorPage } from "./ToneEditorPage";
import type { UseMixerResult } from "./useMixer";
import { cleanup, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";

afterEach(cleanup);

/** The page for part 1 holding a tone from bank `msb`. */
function page(msb: number): HTMLElement {
  const mixer = {
    state: { selectedPart: 0, parts: [{ toneBankMsb: msb, toneBankLsb: 0, tonePC: 0 }] },
  } as unknown as UseMixerResult;
  const { service } = fakeToneService(new WasmDeviceState(0x10));
  const marking = { isUnloaded: () => false, onLoad: () => {} };
  return render(<ToneEditorPage mixer={mixer} service={service} marking={marking} onBack={() => {}} />).container;
}

describe("ToneEditorPage", () => {
  it("opens the PCM Synth editor for SRX tones too", () => {
    for (const msb of [87, 93]) {
      expect(page(msb).textContent, `MSB ${msb}`).toContain("Loading PCM Synth tone data");
      cleanup();
    }
  });

  it("explains that GM2 and ExPCM tones can't be edited", () => {
    for (const msb of [96, 97, 120, 121]) {
      expect(page(msb).textContent, `MSB ${msb}`).toContain("can't edit GM2 or ExPCM tones");
      cleanup();
    }
  });

  it("says so when the bank isn't a tone bank", () => {
    expect(page(0).textContent).toContain("Bank MSB 0 isn't a tone bank");
  });
});
