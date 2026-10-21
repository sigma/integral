import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { IDENTIFY_TIMEOUT_MS, LAST_PORT_KEY, webMidiPorts } from "./midi";
import { button, cleanup, click, render } from "./test/dom";
import { fakeMidiAccess, type FakePortSpec } from "./test/midiAccess";
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
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

/** Let pending promises and the fake device's replies run. */
async function settle(): Promise<void> {
  for (let i = 0; i < 10; i++) {
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
  }
}

/** Mount the app on fake ports and storage, and let it start up. */
async function open(specs: FakePortSpec[], storage = memoryStorage()) {
  const midi = fakeMidiAccess(specs);
  const ports = async () => webMidiPorts(midi.access);
  const { container } = render(<App openPorts={ports} storage={storage} />);
  await settle();
  return { container, storage, midi };
}

/** Whether the connected shell (its Mixer tab) is showing. */
const onMixer = (scope: ParentNode) =>
  [...scope.querySelectorAll("button")].some((b) => b.textContent === "Mixer");

const remembering = (name: string) => memoryStorage({ [LAST_PORT_KEY]: name });

describe("reconnect on open", () => {
  it("reconnects to the remembered port without a click and reads the device", async () => {
    const { container, midi } = await open(
      [{ name: "INTEGRA-7" }, { name: "Network Integra", answers: true }],
      remembering("Network Integra"),
    );
    expect(midi.identified).toEqual(["Network Integra"]);
    expect(onMixer(container)).toBe(true);
    // The state read of a first connection goes out to that port.
    expect(midi.sent.length).toBeGreaterThan(0);
    expect(midi.sent.every((m) => m.port === "Network Integra")).toBe(true);
  });

  it("waits for a click when no port is remembered", async () => {
    const { container, midi } = await open([{ name: "INTEGRA-7", answers: true }]);
    expect(midi.identified).toEqual([]);
    expect(onMixer(container)).toBe(false);
    expect(button(container, "Identify Device")).toBeTruthy();
  });

  it("falls back to the default port when the remembered one is gone", async () => {
    const { container, midi } = await open(
      [{ name: "IAC Bus" }, { name: "INTEGRA-7", answers: true }],
      remembering("Network Integra"),
    );
    expect(midi.identified).toEqual([]);
    expect(onMixer(container)).toBe(false);
    expect(container.querySelector("select")?.selectedOptions[0]?.text).toBe(
      "INTEGRA-7",
    );
  });

  it("connects when the remembered port appears after load", async () => {
    const { container, midi } = await open(
      [{ name: "IAC Bus" }],
      remembering("Network Integra"),
    );
    expect(onMixer(container)).toBe(false);
    act(() => midi.plug({ name: "Network Integra", answers: true }));
    await settle();
    expect(midi.identified).toEqual(["Network Integra"]);
    expect(onMixer(container)).toBe(true);
  });

  it("leaves a port the user picked when the remembered one appears", async () => {
    const { container, midi } = await open(
      [{ name: "INTEGRA-7", answers: true }],
      remembering("Network Integra"),
    );
    click(button(container, "Identify Device"));
    await settle();
    act(() => midi.plug({ name: "Network Integra", answers: true }));
    await settle();
    expect(midi.identified).toEqual(["INTEGRA-7"]);
  });

  it("remembers the port it connects to", async () => {
    const { container, storage } = await open([{ name: "INTEGRA-7", answers: true }]);
    click(button(container, "Identify Device"));
    await settle();
    expect(onMixer(container)).toBe(true);
    expect(storage.getItem(LAST_PORT_KEY)).toBe("INTEGRA-7");
  });

  it("does not remember a port that did not answer", async () => {
    const { container, storage } = await open([{ name: "INTEGRA-7" }]);
    vi.useFakeTimers();
    click(button(container, "Identify Device"));
    await act(() => vi.advanceTimersByTimeAsync(IDENTIFY_TIMEOUT_MS));
    expect(container.textContent).toContain("No response");
    expect(storage.getItem(LAST_PORT_KEY)).toBeNull();
  });
});
