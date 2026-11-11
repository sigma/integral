import { readFileSync } from "node:fs";
import { join } from "node:path";
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { LAST_PORT_KEY, webMidiPorts } from "./midi";
import type { InstanceInfo } from "./plugin";
import { PluginKind, WasmDeviceState } from "../pkg/integral_wasm.js";
import { PluginBar } from "./PluginBar";
import { ToneCatalog } from "./toneCatalog";
import { defaultPartState, type PartState } from "./types";
import type { UseExpansionsResult } from "./useExpansions";
import { cleanup, click, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";
import { fakeMidiAccess } from "./test/midiAccess";
import { fakeBridge } from "./test/pluginBridge";
import { memoryStorage } from "./test/storage";

beforeEach(() => {
  vi.stubGlobal(
    "Worker",
    class {
      onmessage: (() => void) | null = null;
      postMessage(): void {}
    },
  );
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    },
  );
  vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:tick");
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function settle(): Promise<void> {
  for (let i = 0; i < 10; i++) {
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
  }
}

const CONNECTED: Omit<InstanceInfo, "plugin" | "binding"> = {
  status: "connected",
  statusText: "Connected",
  hostEdits: true,
  notices: [],
  defaultTone: null,
  defaultTonePending: false,
  recallOwed: false,
};

const MISMATCH = { notice: "typeMismatch", text: "The bound Part holds another tone type" };

async function openPlugin(info: InstanceInfo) {
  const midi = fakeMidiAccess([{ name: "INTEGRA-7", answers: true }]);
  const fake = fakeBridge();
  const { container } = render(
    <App
      openPorts={async () => webMidiPorts(midi.access)}
      storage={memoryStorage({ [LAST_PORT_KEY]: "INTEGRA-7" })}
      plugin={fake.bridge}
    />,
  );
  await settle();
  act(() => fake.deliver({ type: "instance", ...info }));
  await settle();
  return { container };
}

const toneButton = (scope: ParentNode) =>
  scope.querySelector<HTMLButtonElement>("button[data-tone-button]");

// The bar's stylesheet, read as text: jsdom applies no CSS-module rules, so
// what the disabled style targets is asserted on the source.
const pluginBarCss = readFileSync(join(import.meta.dirname, "PluginBar.module.css"), "utf8");

/** The labels of the groups the open picker lists. */
const groups = (scope: ParentNode) =>
  [...scope.querySelectorAll("summary")]
    .filter((s) => s.parentElement?.parentElement?.className.includes("bankList"))
    .map((s) => s.textContent);

const SN_SYNTH_MSB = 95;

describe("the picker in the plugin editor", () => {
  it("changes the tone of the Part the plugin is bound to", async () => {
    // Making the fake device answer the whole mixer read is out of proportion
    // here: the mixer is declared read, and the device state's call is spied.
    vi.spyOn(WasmDeviceState.prototype, "mixerLoading").mockReturnValue(false);
    const change = vi.spyOn(WasmDeviceState.prototype, "changePartTone");
    const { container } = await openPlugin({
      plugin: "sn_synth",
      binding: { part: 4 },
      ...CONNECTED,
    });
    // The mixer notices the load is over on its next poll.
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 600)));
    await settle();
    expect(toneButton(container)?.disabled).toBe(false);
    click(toneButton(container) as HTMLButtonElement);
    click(container.querySelector<HTMLButtonElement>('[class*="bankButton"]') as HTMLButtonElement);
    await settle();
    const tones = container.querySelectorAll<HTMLButtonElement>('[class*="toneItem"]');
    click(tones[1] as HTMLButtonElement);
    await settle();
    expect(change).toHaveBeenCalledTimes(1);
    const [part, msb, , pc] = change.mock.calls[0] as [number, number, number, number];
    expect([part, msb, pc]).toEqual([4, SN_SYNTH_MSB, 1]);
  });
});

describe("the tone button of a Tone or Kit plugin, in the editor", () => {
  it("is disabled until the mixer is read, and says why", async () => {
    // The fake device answers nothing but its identity: the mixer stays unread.
    const { container } = await openPlugin({
      plugin: "sn_synth",
      binding: { part: 2 },
      ...CONNECTED,
    });
    expect(toneButton(container)?.disabled).toBe(true);
    expect(toneButton(container)?.title).toBe("Waiting for the device");
  });

  it("is disabled while the plugin is unbound", async () => {
    const { container } = await openPlugin({ plugin: "sn_synth", binding: {}, ...CONNECTED });
    expect(toneButton(container)?.disabled).toBe(true);
    click(toneButton(container) as HTMLButtonElement);
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
  });

  it("is disabled while the Part holds another tone type", async () => {
    const { container } = await openPlugin({
      plugin: "sn_synth",
      binding: { part: 1 },
      ...CONNECTED,
      notices: [MISMATCH],
    });
    expect(toneButton(container)?.disabled).toBe(true);
  });

  it("is not in the Studio Set plugin's bar", async () => {
    const { container } = await openPlugin({ plugin: "studio_set", binding: {}, ...CONNECTED });
    expect(toneButton(container)).toBeNull();
    expect(container.textContent).not.toContain("TONE");
  });
});

const KINDS = [
  ["sn_synth", PluginKind.SnSynth],
  ["sn_acoustic", PluginKind.SnAcoustic],
  ["pcm_synth", PluginKind.PcmSynth],
  ["sn_drum", PluginKind.SnDrum],
  ["pcm_drum", PluginKind.PcmDrum],
] as const;

const expansions = {
  refresh: () => {},
  slots: null,
  isUnloaded: () => false,
  unloadedExpansionOf: () => null,
} as unknown as UseExpansionsResult;

/** A Part in the mixer, with the fields a test sets. */
const part = (over: Partial<PartState>): PartState => ({ ...defaultPartState(), ...over });

/** A PluginBar over a real tone catalog, with the props a test changes. */
function bar() {
  const { service } = fakeToneService(new WasmDeviceState(0x10));
  // User banks come from the device, which here lists nothing.
  Object.assign(service, { requestToneCatalogPage: async () => [] });
  const catalog = new ToneCatalog(service);
  const changed: [number, number, number, number][] = [];
  const part: PartState = { ...defaultPartState(), toneBankMsb: SN_SYNTH_MSB };
  const view = (
    kind: PluginKind,
    info: Partial<InstanceInfo>,
    props: { part?: PartState } = { part },
  ) => (
    <PluginBar
      kind={kind}
      info={{ plugin: "x", binding: { part: 0 }, ...CONNECTED, ...info }}
      onBind={() => {}}
      part={props.part}
      catalog={catalog}
      expansions={expansions}
      onChangeTone={(...args) => changed.push(args)}
      onLoadExpansion={() => {}}
    />
  );
  return { view, changed, part };
}

describe("the tone button of a Tone or Kit plugin", () => {
  it.each(KINDS)("is enabled in the %s plugin's bar once bound and read", (_, kind) => {
    const { view } = bar();
    const { container } = render(view(kind, { binding: { part: 2 } }));
    expect(toneButton(container)?.disabled).toBe(false);
  });

  it("is disabled for a Kit plugin bound to a key, which never selects a kit", () => {
    const { view } = bar();
    const { container, rerender } = render(view(PluginKind.PcmDrum, { binding: { part: 0 } }));
    expect(toneButton(container)?.disabled).toBe(false);
    rerender(view(PluginKind.PcmDrum, { binding: { part: 0, key: 38 } }));
    expect(toneButton(container)?.disabled).toBe(true);
    expect(toneButton(container)?.title).toContain("whole kit");
  });

  it("is disabled while the project's tone is being recalled, says why, and closes the picker", () => {
    // A pick then would be lost: the push selects the saved tone over it.
    const { view } = bar();
    const { container, rerender } = render(view(PluginKind.SnSynth, {}));
    click(toneButton(container) as HTMLButtonElement);
    expect(container.querySelector('[class*="overlay"]')).not.toBeNull();
    rerender(view(PluginKind.SnSynth, { recallOwed: true }));
    expect(toneButton(container)?.disabled).toBe(true);
    expect(toneButton(container)?.title).toBe("Waiting to recall the project's tone");
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
    rerender(view(PluginKind.SnSynth, {}));
    expect(toneButton(container)?.disabled).toBe(false);
  });

  it("is disabled and reads as disabled while the Part holds another tone type", () => {
    const { view } = bar();
    const props = {
      part: part({ toneBankMsb: SN_SYNTH_MSB, toneBankLsb: 70, tonePC: 45, toneName: "Saw Lead 3" }),
    };
    const { container, rerender } = render(view(PluginKind.SnSynth, {}, props));
    expect(toneButton(container)?.disabled).toBe(false);
    rerender(view(PluginKind.SnSynth, { notices: [MISMATCH] }, props));
    const button = toneButton(container) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(button.title).toBe("The Part holds another tone type");
    // The stylesheet dims that attribute and drops the hover highlight.
    expect(pluginBarCss).toMatch(/\.toneButton:disabled\W[^}]*opacity/s);
    expect(pluginBarCss).toMatch(/\.toneButton:disabled\W[^}]*cursor: default/s);
    expect(pluginBarCss).toMatch(/\.toneButton:disabled:hover\W[^}]*border-color/s);
  });

  it("shows the Part's current tone once the device changed its type", () => {
    const { view } = bar();
    const sawLead = part({
      toneBankMsb: SN_SYNTH_MSB,
      toneBankLsb: 70,
      tonePC: 45,
      toneName: "Saw Lead 3",
    });
    const { container, rerender } = render(view(PluginKind.SnSynth, {}, { part: sawLead }));
    expect(toneButton(container)?.textContent).toContain("Saw Lead 3");

    // The device switched the Part to a PCM Synth tone; the mixer follows.
    const jp8 = part({ toneBankMsb: 87, toneBankLsb: 0, tonePC: 8, toneName: "JP8 Strings1" });
    rerender(view(PluginKind.SnSynth, { notices: [MISMATCH] }, { part: jp8 }));
    const label = toneButton(container)?.textContent ?? "";
    expect(label).toContain("JP8 Strings1");
    expect(label).not.toContain("Saw Lead 3");

    // A tone not yet named falls back to its bank and program, never the
    // stale name of the plugin's own type.
    rerender(view(PluginKind.SnSynth, { notices: [MISMATCH] }, { part: { ...jp8, toneName: "" } }));
    const unnamed = toneButton(container)?.textContent ?? "";
    expect(unnamed).toContain("Bank 87-0 : PC 9");
    expect(unnamed).not.toContain("Saw Lead 3");
    expect(unnamed).not.toContain("JP8 Strings1");
  });
});

describe("the picker of a Tone or Kit plugin", () => {
  it("lists only the banks of the plugin's type, Expansion banks included", () => {
    const { view } = bar();
    const lists: Record<string, (string | null)[]> = {};
    for (const [plugin, kind] of KINDS) {
      const { container } = render(view(kind, {}));
      click(toneButton(container) as HTMLButtonElement);
      lists[plugin] = groups(container);
      expect(lists[plugin]?.length).toBeGreaterThan(0);
      cleanup();
    }
    // Each plugin lists its own built-in group, and no other type's.
    const builtIn = {
      sn_synth: "SN Synth",
      sn_acoustic: "SN Acoustic",
      pcm_synth: "PCM Synth",
      sn_drum: "SN Drum",
      pcm_drum: "PCM Drum",
    };
    for (const [plugin, label] of Object.entries(builtIn)) {
      expect(lists[plugin]).toContain(label);
      for (const other of Object.values(builtIn).filter((l) => l !== label)) {
        expect(lists[plugin]).not.toContain(other);
      }
    }
  });

  it("changes the bound Part's tone, and closes", async () => {
    const { view, changed } = bar();
    const { container } = render(view(PluginKind.SnSynth, { binding: { part: 3 } }));
    click(toneButton(container) as HTMLButtonElement);
    click(container.querySelector<HTMLButtonElement>('[class*="bankButton"]') as HTMLButtonElement);
    await settle();
    const tones = container.querySelectorAll<HTMLButtonElement>('[class*="toneItem"]');
    expect(tones.length).toBeGreaterThan(2);
    click(tones[2] as HTMLButtonElement);
    expect(changed).toHaveLength(1);
    const [part, msb, , pc] = changed[0] as [number, number, number, number];
    expect([part, msb, pc]).toEqual([3, SN_SYNTH_MSB, 2]);
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
  });

  it("closes when the plugin turns inert, and stays closed once it is not", () => {
    const { view } = bar();
    const { container, rerender } = render(view(PluginKind.SnSynth, {}));
    click(toneButton(container) as HTMLButtonElement);
    expect(container.querySelector('[class*="overlay"]')).not.toBeNull();
    rerender(view(PluginKind.SnSynth, { notices: [MISMATCH] }));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
    rerender(view(PluginKind.SnSynth, {}));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
  });

  it("closes when the plugin is unbound, and stays closed once it is bound again", () => {
    const { view } = bar();
    const { container, rerender } = render(view(PluginKind.SnSynth, {}));
    click(toneButton(container) as HTMLButtonElement);
    rerender(view(PluginKind.SnSynth, { binding: {} }, { part: undefined }));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
    rerender(view(PluginKind.SnSynth, {}));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
  });

  it("closes when the Binding moves to another Part, so a pick never reaches the new one", () => {
    const { view, changed } = bar();
    const { container, rerender } = render(view(PluginKind.SnSynth, { binding: { part: 0 } }));
    click(toneButton(container) as HTMLButtonElement);
    rerender(view(PluginKind.SnSynth, { binding: { part: 1 } }));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
    expect(changed).toEqual([]);
    rerender(view(PluginKind.SnSynth, { binding: { part: 0 } }));
    expect(container.querySelector('[class*="overlay"]')).toBeNull();
  });
});
