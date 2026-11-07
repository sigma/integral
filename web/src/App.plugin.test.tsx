import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { LAST_PORT_KEY, webMidiPorts } from "./midi";
import type { InstanceInfo } from "./plugin";
import { button, cleanup, click, render } from "./test/dom";
import { fakeMidiAccess, type FakePortSpec } from "./test/midiAccess";
import { fakeBridge } from "./test/pluginBridge";
import { memoryStorage } from "./test/storage";

// jsdom has no Worker or blob URLs (the Sequencer only needs them to exist),
// nor the ResizeObserver the mixer's FX strips measure with.
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

/** Let pending promises and the fake device's replies run. */
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
};

/**
 * The editor of the plugin `info` describes, over a device that answers:
 * the page is up and the plugin has said what the instance is.
 */
async function openPlugin(info: InstanceInfo, devices: FakePortSpec[] = [{ name: "INTEGRA-7", answers: true }]) {
  const midi = fakeMidiAccess(devices);
  const fake = fakeBridge();
  const { container } = render(
    <App
      openPorts={async () => webMidiPorts(midi.access)}
      storage={memoryStorage({ [LAST_PORT_KEY]: "INTEGRA-7" })}
      plugin={fake.bridge}
    />,
  );
  await settle();
  await tell(fake, info);
  return { container, fake };
}

/** The plugin tells the page what the instance is. */
async function tell(fake: ReturnType<typeof fakeBridge>, info: InstanceInfo): Promise<void> {
  act(() => fake.deliver({ type: "instance", ...info }));
  await settle();
}

const tabs = (scope: ParentNode) =>
  [...scope.querySelectorAll("nav button")].map((b) => b.textContent);

const select = (scope: ParentNode, label: string): HTMLSelectElement => {
  const found = [...scope.querySelectorAll("label")].find((l) => l.textContent?.startsWith(label));
  const element = found?.querySelector("select");
  if (!element) throw new Error(`no "${label}" select`);
  return element;
};

/** Choose `value` in `element`, as a user does. */
function choose(element: HTMLSelectElement, value: string): void {
  act(() => {
    element.value = value;
    element.dispatchEvent(new Event("change", { bubbles: true }));
  });
}

const status = (scope: ParentNode) => scope.querySelector('[role="status"]')?.textContent;

describe("the plugin editor's pages", () => {
  it("are the Mixer and Surround for the Studio Set plugin, with nothing else of the app", async () => {
    const { container } = await openPlugin({ plugin: "studio_set", binding: {}, ...CONNECTED });
    expect(tabs(container)).toEqual(["Mixer", "Surround"]);
    // The top bar's tone and Studio Set choice are not the plugin's to make.
    expect(container.textContent).not.toContain("TONE");
    expect(container.querySelector("select")).toBeNull();
    // The Mixer is where it opens: Chorus, Reverb and Master are its strips.
    expect(container.textContent).toContain("Loading mixer state");
    click(button(container, "Surround"));
    expect(container.textContent).not.toContain("Loading mixer state");
    expect(status(container)).toBe("Connected");
  });

  it.each(["sn_synth", "sn_acoustic", "pcm_synth", "sn_drum", "pcm_drum"])(
    "is the tone editor alone for the %s plugin, with no tab bar",
    async (plugin) => {
      const { container } = await openPlugin({ plugin, binding: { part: 0 }, ...CONNECTED });
      expect(tabs(container)).toEqual([]);
      expect(container.textContent).not.toContain("Back to Mixer");
      // The tone editor, not the Mixer: Part 1 holds no tone on a fresh mixer.
      expect(container.textContent).not.toContain("Loading mixer state");
      expect(container.textContent).toContain("no tone to edit");
    },
  );

  it("asks for a Part before it edits anything, when unbound", async () => {
    const { container } = await openPlugin({ plugin: "pcm_synth", binding: null, ...CONNECTED });
    expect(container.textContent).toContain("Choose a Part above");
    expect(select(container, "Part").value).toBe("");
  });
});

describe("the Binding picker", () => {
  it("offers a Tone plugin its Parts and sends the one chosen", async () => {
    const { container, fake } = await openPlugin({ plugin: "sn_synth", binding: null, ...CONNECTED });
    const part = select(container, "Part");
    // The Parts, after the prompt.
    expect(part.options.length).toBe(17);
    expect(() => select(container, "Target")).toThrow();

    choose(part, "2");
    expect(fake.posted).toContainEqual({ type: "bind", part: 2 });
  });

  it("shows the Binding the plugin reports", async () => {
    const { container, fake } = await openPlugin({ plugin: "sn_synth", binding: null, ...CONNECTED });
    await tell(fake, { plugin: "sn_synth", binding: { part: 4 }, ...CONNECTED });
    expect(select(container, "Part").value).toBe("4");
  });

  it("offers a Kit plugin the whole kit or a key of its kit type, once it has a Part", async () => {
    const { container, fake } = await openPlugin({ plugin: "pcm_drum", binding: null, ...CONNECTED });
    expect(select(container, "Target").disabled).toBe(true);

    await tell(fake, { plugin: "pcm_drum", binding: { part: 1 }, ...CONNECTED });
    const target = select(container, "Target");
    expect(target.disabled).toBe(false);
    expect(target.value).toBe("");
    expect(target.options[0]!.textContent).toBe("Whole kit");

    choose(target, "40");
    expect(fake.posted).toContainEqual({ type: "bind", part: 1, key: 40 });
    choose(select(container, "Part"), "3");
    expect(fake.posted).toContainEqual({ type: "bind", part: 3, key: undefined });
  });

  it("keeps a key when the Part changes", async () => {
    const { container, fake } = await openPlugin({
      plugin: "sn_drum",
      binding: { part: 1, key: 38 },
      ...CONNECTED,
    });
    choose(select(container, "Part"), "5");
    expect(fake.posted).toContainEqual({ type: "bind", part: 5, key: 38 });
  });

  it("is absent for the Studio Set plugin, which has one target", async () => {
    const { container } = await openPlugin({ plugin: "studio_set", binding: {}, ...CONNECTED });
    expect(container.querySelector("label select")).toBeNull();
  });
});

describe("a refused request", () => {
  it("shows why the plugin refused a Binding, until it reports the instance again", async () => {
    const { container, fake } = await openPlugin({ plugin: "sn_synth", binding: { part: 1 }, ...CONNECTED });
    act(() => fake.deliver({ type: "refused", message: "cannot be bound" }));
    await settle();
    expect(container.querySelector('[role="alert"]')?.textContent).toBe("cannot be bound");
    // Not the device link's trouble: the status line is as it was.
    expect(status(container)).toBe("Connected");

    await tell(fake, { plugin: "sn_synth", binding: { part: 2 }, ...CONNECTED });
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });
});

describe("a plugin kind the page does not know", () => {
  it("says so rather than showing nothing", async () => {
    const { container } = await openPlugin({ plugin: "a_new_kind", binding: null, ...CONNECTED });
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("a_new_kind");
  });
});

describe("the status line", () => {
  it.each([
    ["serviceNotRunning", "Device service not running"],
    ["deviceOffline", "Device offline"],
    ["serviceIncompatible", "Device service version mismatch"],
  ] as const)("says %s even when there is no device to show", async (code, text) => {
    // No device: the editor has only the plugin's word.
    const { container } = await openPlugin(
      { plugin: "pcm_synth", binding: { part: 0 }, status: code, statusText: text, hostEdits: false, notices: [] },
      [],
    );
    expect(status(container)).toBe(text);
  });

  it("follows the instance as the plugin reports it", async () => {
    const { container, fake } = await openPlugin(
      { plugin: "pcm_synth", binding: { part: 0 }, status: "serviceNotRunning", statusText: "Device service not running", hostEdits: false, notices: [] },
      [],
    );
    await tell(fake, { plugin: "pcm_synth", binding: { part: 0 }, ...CONNECTED });
    expect(status(container)).toBe("Connected");
  });

  it("shows each notice the instance reports, and drops it when it stops", async () => {
    const { container, fake } = await openPlugin({ plugin: "pcm_synth", binding: { part: 0 }, ...CONNECTED });
    expect(container.querySelector("[data-notice]")).toBeNull();

    const notices = [
      { notice: "recalling", text: "Recalling…" },
      { notice: "typeMismatch", text: "The bound Part holds another tone type" },
      { notice: "duplicate", text: "Another instance is bound to this target" },
    ];
    await tell(fake, { plugin: "pcm_synth", binding: { part: 0 }, ...CONNECTED, notices });
    const shown = [...container.querySelectorAll("[data-notice]")].map((n) => [
      n.getAttribute("data-notice"),
      n.textContent,
    ]);
    expect(shown).toEqual(notices.map((n) => [n.notice, n.text]));

    await tell(fake, { plugin: "pcm_synth", binding: { part: 0 }, ...CONNECTED, notices: notices.slice(0, 1) });
    expect(container.querySelectorAll("[data-notice]")).toHaveLength(1);
    await tell(fake, { plugin: "pcm_synth", binding: { part: 0 }, ...CONNECTED });
    expect(container.querySelector("[data-notice]")).toBeNull();
  });
});

describe("in a browser", () => {
  it("shows every page and the top bar, as before", async () => {
    const midi = fakeMidiAccess([{ name: "INTEGRA-7", answers: true }]);
    const { container } = render(
      <App
        openPorts={async () => webMidiPorts(midi.access)}
        storage={memoryStorage({ [LAST_PORT_KEY]: "INTEGRA-7" })}
      />,
    );
    await settle();
    expect(tabs(container)).toEqual(
      expect.arrayContaining(["Mixer", "Surround", "Tone Edit", "Sequencer", "Expansions", "Library"]),
    );
    expect(container.textContent).toContain("TONE");
    expect(container.querySelector('[role="status"]')).toBeNull();
  });
});
