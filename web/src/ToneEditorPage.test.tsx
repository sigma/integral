import type { ComponentProps } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { ToneType, WasmDeviceState } from "../pkg/integral_wasm.js";
import { ToneEditorPage } from "./ToneEditorPage";
import type { UseMixerResult } from "./useMixer";
import { cleanup, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";

afterEach(cleanup);

/** The bank MSB of the other part's tone: an SRX bank, which holds PCM Synth tones. */
const OTHER_MSB = 87;

/** The page for part 1 holding a tone from bank `msb`. */
function page(msb: number, extra: Partial<ComponentProps<typeof ToneEditorPage>> = {}): HTMLElement {
  const mixer = {
    state: {
      selectedPart: 0,
      parts: [
        { toneBankMsb: msb, toneBankLsb: 0, tonePC: 0 },
        // Another type of tone, for the plugin tests.
        { toneBankMsb: OTHER_MSB, toneBankLsb: 0, tonePC: 0 },
      ],
    },
  } as unknown as UseMixerResult;
  const { service } = fakeToneService(new WasmDeviceState(0x10));
  const marking = { isUnloaded: () => false, onLoad: () => {} };
  return render(
    <ToneEditorPage mixer={mixer} service={service} marking={marking} onBack={() => {}} {...extra} />,
  ).container;
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

  describe("as a plugin's editor", () => {
    it("has no way back to a Mixer it does not have", () => {
      expect(page(0).textContent).toContain("Back to Mixer");
      cleanup();
      const bound = page(0, { onBack: undefined });
      expect(bound.textContent).not.toContain("Back to Mixer");
    });

    it("edits the bound Part's tone, not the mixer's selected one", () => {
      // The selected Part (the first) holds no tone; the bound one (the second) a PCM Synth.
      const text = page(0, { part: 1 }).textContent;
      expect(text).toContain("Loading PCM Synth tone data");
      expect(text).not.toContain("isn't a tone bank");
    });

    it("edits a tone of its own type", () => {
      expect(page(OTHER_MSB, { part: 0, only: ToneType.PcmSynth }).textContent).toContain(
        "Loading PCM Synth tone data",
      );
    });

    it("stays inert, with the reason, on a tone of another type", () => {
      const text = page(OTHER_MSB, { part: 0, only: ToneType.SnSynth }).textContent;
      expect(text).toContain("another type of tone");
      expect(text).not.toContain("Loading");
    });
  });
});
