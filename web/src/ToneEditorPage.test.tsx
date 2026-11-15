import { act, type ComponentProps } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { ToneType, WasmDeviceState } from "../pkg/integral_wasm.js";
import { InitProvider, useInit } from "./InitFlow";
import { ToneEditorPage } from "./ToneEditorPage";
import type { UseMixerResult } from "./useMixer";
import { button, cleanup, click, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";

// jsdom lacks the ResizeObserver the editors' Grid zoom measures with.
globalThis.ResizeObserver ??= class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
};

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

    it("re-reads the editor when a finished refresh says so (#562)", async () => {
      // The editor has read its blocks; a risen `reread` (the plugin's count
      // of finished refreshes) remounts it, as a tone change does.
      const mixer = {
        state: { selectedPart: 0, parts: [{ toneBankMsb: OTHER_MSB, toneBankLsb: 0, tonePC: 0 }] },
      } as unknown as UseMixerResult;
      const { service } = fakeToneService(new WasmDeviceState(0x10));
      const marking = { isUnloaded: () => false, onLoad: () => {} };
      const props = { mixer, service, marking, part: 0, only: ToneType.PcmSynth } as const;
      const view = render(<ToneEditorPage {...props} reread={0} />);
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
      expect(view.container.textContent).not.toContain("Loading PCM Synth tone data");
      view.rerender(<ToneEditorPage {...props} reread={1} />);
      expect(view.container.textContent).toContain("Loading PCM Synth tone data");
    });

    it("re-reads the editor once a Tone Init of its Part finishes", async () => {
      const mixer = {
        state: { selectedPart: 0, parts: [{ toneBankMsb: OTHER_MSB, toneBankLsb: 0, tonePC: 0 }] },
        reload: () => {},
      } as unknown as UseMixerResult;
      const { service } = fakeToneService(new WasmDeviceState(0x10));
      service.initialize = async () => undefined;
      const answer = service.requestData.bind(service);
      let reads = 0;
      service.requestData = (address, size) => {
        reads++;
        return answer(address, size);
      };
      const marking = { isUnloaded: () => false, onLoad: () => {} };
      function Item() {
        const init = useInit();
        return <button onClick={() => init.request({ kind: "tone", part: 0 })}>Init…</button>;
      }
      const { container } = render(
        <InitProvider service={service} mixer={mixer}>
          <Item />
          <ToneEditorPage mixer={mixer} service={service} marking={marking} part={0} />
        </InitProvider>,
      );
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
      expect(container.textContent).not.toContain("Loading PCM Synth tone data");
      const before = reads;
      expect(before).toBeGreaterThan(0);
      click(button(container, "Init…"));
      await act(async () => click(button(container, "Init")));
      // The remounted editor read its blocks again.
      expect(reads).toBe(2 * before);
    });
  });
});
