/** Partial Init on a drum kit: each key's header, in both kit editors (#598). */

import { act, type ComponentType, type MutableRefObject } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ToneType, WasmDeviceState, tone_type_from_bank_msb } from "../pkg/integral_wasm.js";
import type { DrumScope } from "./pluginPages";
import type { IntegraService } from "./integra";
import { PcmDrumEditor } from "./PcmDrumEditor";
import { SnDrumEditor } from "./SnDrumEditor";
import { ToneEditorPage } from "./ToneEditorPage";
import type { UseMixerResult } from "./useMixer";
import { button, cleanup, click, render } from "./test/dom";
import { drumKey } from "./test/drum";
import { confirmInit, fakeInit } from "./test/init";
import { fakeToneService } from "./test/toneService";

// jsdom lacks the ResizeObserver a Grid zoom measures with.
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

/** What both kit editors take. */
interface KitEditorProps {
  partIndex: number;
  service: IntegraService;
  scope?: DrumScope;
  initItems?: boolean;
  lastKey?: MutableRefObject<number | undefined>;
}

/** Each kit editor, its tone type, and a preset bank of its kits. */
const KITS: [string, ComponentType<KitEditorProps>, ToneType, number][] = [
  ["PCM Drum", PcmDrumEditor, ToneType.PcmDrum, 86],
  ["SN Drum", SnDrumEditor, ToneType.SnDrum, 88],
];

/** The kit placements the Init flow holds busy, by their screen id. */
const busySections = (scope: ParentNode) =>
  [...scope.querySelectorAll('[aria-busy="true"]')].map((g) => g.closest<HTMLElement>("[data-section]")!.dataset.section);

describe.each(KITS)("the %s editor's key header", (_, Editor, toneType, msb) => {
  /** The editor for Part PART under an Init flow, its kit read. */
  async function editor(props: Partial<KitEditorProps> = { initItems: true }) {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const init = fakeInit(service);
    const { container } = render(init.wrap(<Editor partIndex={PART} service={service} {...props} />));
    await act(async () => {});
    return { container, init };
  }

  it("offers the selected key's Init…, which inits that key by its MIDI key number after its confirmation", async () => {
    const { container, init } = await editor();
    const header = container.querySelector("[data-key-header]")!;
    expect(header.textContent).toContain("C2");
    expect(button(header, "Init key C2…").textContent).toBe("Init…");

    click(drumKey(container, 38));
    await act(async () => {});
    click(button(container.querySelector("[data-key-header]")!, "Init key D2…"));
    expect(document.body.textContent).toContain(`This resets key D2 (38) of Part ${PART + 1}'s kit.`);
    expect(init.asked).toEqual([]);
    await act(async () => click(button(document.body, "Init")));
    expect(init.asked).toEqual([{ kind: "partial", part: PART, toneType, partial: 38 }]);
  });

  it("holds the key's Sections busy, not the kit's MFX, while that key's Init runs", async () => {
    const { container, init } = await editor();
    await confirmInit(container, "Init key C2…");
    const busy = busySections(container);
    expect(busy.length).toBeGreaterThan(0);
    expect(busy).not.toContain("mfx");
    // Another key's Sections are live: the Init resets C2 alone.
    click(drumKey(container, 40));
    await act(async () => {});
    expect(busySections(container)).toEqual([]);
    await init.finish();
  });

  it("offers none without its Init items (a plugin's editor)", async () => {
    const { container } = await editor({ scope: { key: 40 } });
    expect(container.querySelector("[data-key-header]")!.textContent).toContain("E2");
    expect(container.textContent).not.toContain("Init…");
  });

  it("re-reads the kit once the key's Init finishes, and opens on that key again", async () => {
    expect(tone_type_from_bank_msb(msb)).toBe(toneType);
    const mixer = {
      state: { selectedPart: PART, parts: [null, { toneBankMsb: msb, toneBankLsb: 0, tonePC: 0 }] },
      reload: () => {},
    } as unknown as UseMixerResult;
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const init = fakeInit(service);
    const marking = { isUnloaded: () => false, onLoad: () => {} };
    const { container } = render(
      init.wrap(<ToneEditorPage mixer={mixer} service={service} marking={marking} initItems />),
    );
    await act(async () => {});
    click(drumKey(container, 38));
    await act(async () => {});
    const header = container.querySelector("[data-key-header]")!;

    await confirmInit(container, "Init key D2…");
    await init.finish();
    await act(async () => {});
    // A new editor, which read the kit again, on key 38.
    expect(header.isConnected).toBe(false);
    expect(container.querySelector("[data-key-header]")!.textContent).toContain("D2");
    expect(button(container, "Init key D2…")).toBeTruthy();
  });

  it("opens another tone on the default key, not the last one picked", async () => {
    const mixerOn = (tonePC: number) =>
      ({
        state: { selectedPart: PART, parts: [null, { toneBankMsb: msb, toneBankLsb: 0, tonePC }] },
        reload: () => {},
      }) as unknown as UseMixerResult;
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const init = fakeInit(service);
    const marking = { isUnloaded: () => false, onLoad: () => {} };
    const page = (tonePC: number) =>
      init.wrap(<ToneEditorPage mixer={mixerOn(tonePC)} service={service} marking={marking} initItems />);
    const view = render(page(0));
    await act(async () => {});
    click(drumKey(view.container, 38));
    await act(async () => {});
    view.rerender(page(1));
    await act(async () => {});
    expect(view.container.querySelector("[data-key-header]")!.textContent).toContain("C2");
  });
});
