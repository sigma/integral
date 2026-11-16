import { act, type ComponentProps } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ToneType, WasmDeviceState, is_editable_bank, tone_type_from_bank_msb } from "../pkg/integral_wasm.js";
import { InitProvider, useInit } from "./InitFlow";
import type { IntegraService } from "./integra";
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

/** Count `service`'s data requests: the reads the editor makes. */
function countReads(service: IntegraService): () => number {
  const answer = service.requestData.bind(service);
  let reads = 0;
  service.requestData = (address, size) => {
    reads++;
    return answer(address, size);
  };
  return () => reads;
}

/** Every Bank Select MSB. */
const MSBS = [...Array(128).keys()];

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
      const reads = countReads(service);
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
      const before = reads();
      expect(before).toBeGreaterThan(0);
      click(button(container, "Init…"));
      await act(async () => click(button(container, "Init")));
      // The remounted editor read its blocks again.
      expect(reads()).toBe(2 * before);
    });
  });

  describe("Tone Init", () => {
    /** An editable bank MSB of each tone type, as core maps them. */
    const editable = new Map<ToneType, number>();
    for (const msb of MSBS) {
      const type = tone_type_from_bank_msb(msb);
      if (type !== undefined && is_editable_bank(msb) && !editable.has(type)) editable.set(type, msb);
    }

    /** The web page for Part 1, holding a tone from bank `msb`, in the Init flow. */
    function initPage(msb: number) {
      const mixer = {
        state: { selectedPart: 0, parts: [{ toneBankMsb: msb, toneBankLsb: 0, tonePC: 0 }] },
        reload: () => {},
      } as unknown as UseMixerResult;
      const { service } = fakeToneService(new WasmDeviceState(0x10));
      // The Init answers "finished" when the test says so.
      let finish: () => void = () => {};
      const initialize = vi.fn(
        () =>
          new Promise<string | undefined>((resolve) => {
            finish = () => resolve(undefined);
          }),
      );
      service.initialize = initialize;
      const reReadPart = vi.fn();
      service.device.reReadPart = reReadPart;
      const reads = countReads(service);
      const marking = { isUnloaded: () => false, onLoad: () => {} };
      const { container } = render(
        <InitProvider service={service} mixer={mixer}>
          <ToneEditorPage mixer={mixer} service={service} marking={marking} onBack={() => {}} />
        </InitProvider>,
      );
      return {
        container,
        initialize,
        reReadPart,
        reads,
        finish: () => act(async () => finish()),
      };
    }

    /** Let the editor's reads answer. */
    const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

    it("knows an editable bank of every tone type", () => {
      const types = Object.values(ToneType).filter((t) => typeof t === "number");
      expect([...editable.keys()].sort()).toEqual(types.sort());
    });

    for (const [type, msb] of editable) {
      it(`resets the ${ToneType[type]} tone from the header, then re-reads the editor`, async () => {
        const page = initPage(msb);
        await settle();
        const before = page.reads();
        expect(before).toBeGreaterThan(0);

        click(button(page.container, "Init…"));
        const dialog = page.container.querySelector('[role="dialog"]')!;
        expect(dialog.getAttribute("aria-label")).toBe("Init Part 1's tone");
        expect(dialog.textContent).toContain("keeps its type");
        expect(page.initialize).not.toHaveBeenCalled();

        click(button(dialog, "Init"));
        expect(page.initialize).toHaveBeenCalledWith({ kind: "tone", part: 0 });
        // Until "finished", the editor is inert and the item can't ask again.
        const guard = page.container.querySelector('[aria-busy="true"]');
        expect(guard?.hasAttribute("inert")).toBe(true);
        expect(button(page.container, "Init…").disabled).toBe(true);
        expect(page.reads()).toBe(before);

        await page.finish();
        await settle();
        expect(page.container.querySelector('[aria-busy="true"]')).toBeNull();
        expect(button(page.container, "Init…").disabled).toBe(false);
        // The Part's dump, with the tone's name, and the remounted editor's blocks.
        expect(page.reReadPart).toHaveBeenCalledWith(0);
        expect(page.reads()).toBeGreaterThan(before);
      });
    }

    it("resets nothing when cancelled", () => {
      const page = initPage(editable.get(ToneType.SnSynth)!);
      click(button(page.container, "Init…"));
      click(button(page.container, "Cancel"));
      expect(page.container.querySelector('[role="dialog"]')).toBeNull();
      expect(page.initialize).not.toHaveBeenCalled();
    });

    it("has no item without a tone to edit", () => {
      // No tone bank, and a GM2 or ExPCM tone, which can't be edited.
      expect(page(0).textContent).not.toContain("Init…");
      cleanup();
      const locked = MSBS.find(
        (msb) => tone_type_from_bank_msb(msb) !== undefined && !is_editable_bank(msb),
      )!;
      expect(page(locked).textContent).not.toContain("Init…");
    });

    it("isn't in a plugin's editor, which has no header (web first)", () => {
      const text = page(OTHER_MSB, { onBack: undefined, part: 0, only: ToneType.PcmSynth }).textContent;
      expect(text).not.toContain("Init…");
    });
  });
});
