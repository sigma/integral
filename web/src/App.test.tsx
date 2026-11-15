import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import {
  DEVICE_SERVICE_NAME,
  NOT_RUNNING_HINT,
  deviceServicePair,
  withDeviceService,
} from "./deviceService";
import { identity_request, initFinishedAddress } from "../pkg/integral_wasm.js";
import { pluginPorts, type PageMessage, type PluginBridge } from "./plugin";
import { IDENTIFY_TIMEOUT_MS, LAST_PORT_KEY, webMidiPorts } from "./midi";
import { button, cleanup, click, render } from "./test/dom";
import { dt1, fakeMidiAccess, type FakePortSpec } from "./test/midiAccess";
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
async function open(
  specs: FakePortSpec[],
  storage = memoryStorage(),
  connectOnlyPort = false,
) {
  const midi = fakeMidiAccess(specs);
  const ports = async () => webMidiPorts(midi.access);
  const { container } = render(
    <App openPorts={ports} storage={storage} connectOnlyPort={connectOnlyPort} />,
  );
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

  it("connects to the only port without a click where that is the choice", async () => {
    const { container, midi } = await open(
      [{ name: "INTEGRA-7", answers: true }],
      memoryStorage(),
      true,
    );
    expect(midi.identified).toEqual(["INTEGRA-7"]);
    expect(onMixer(container)).toBe(true);
  });

  it("still waits for a click when there is a choice of ports", async () => {
    const { midi } = await open(
      [{ name: "INTEGRA-7", answers: true }, { name: "IAC Bus" }],
      memoryStorage(),
      true,
    );
    expect(midi.identified).toEqual([]);
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

/** The `index`th socket the service port opened. */
function socket<T>(sockets: T[], index: number): T {
  const found = sockets[index];
  if (!found) throw new Error(`no socket ${index}`);
  return found;
}

describe("the Device service port", () => {
  /** A service port whose sockets the test holds. */
  function serviceApp(midi?: ReturnType<typeof fakeMidiAccess>) {
    const sockets: {
      readyState: number;
      sent: Uint8Array[];
      onopen: ((e: Event) => void) | null;
      onclose: ((e: CloseEvent) => void) | null;
      onmessage: ((e: MessageEvent) => void) | null;
    }[] = [];
    let retry: () => void = () => {};
    const pair = deviceServicePair(
      "ws://service",
      () => {
        const socket = {
          binaryType: "",
          readyState: 0,
          sent: [] as Uint8Array[],
          send(data: Uint8Array) {
            this.sent.push(data);
          },
          close() {},
          onopen: null,
          onclose: null,
          onerror: null,
          onmessage: null,
        };
        sockets.push(socket);
        return socket;
      },
      {
        set: (callback) => {
          retry = callback;
          return 1;
        },
        clear: () => {},
      },
    );
    const ports = async () =>
      withDeviceService(midi ? webMidiPorts(midi.access) : null, pair);
    const { container } = render(
      <App
        openPorts={ports}
        storage={memoryStorage({ [LAST_PORT_KEY]: DEVICE_SERVICE_NAME })}
      />,
    );
    return { container, sockets, retry: () => retry() };
  }

  it("says the service is not running, and asks the device once it is up", async () => {
    const { container, sockets, retry } = serviceApp();
    await settle();
    act(() => socket(sockets, 0).onclose?.(new CloseEvent("close")));
    await settle();
    expect(container.textContent).toContain(NOT_RUNNING_HINT);
    expect(onMixer(container)).toBe(false);

    // The next try finds the service: the page asks the device again.
    act(() => retry());
    expect(sockets).toHaveLength(2);
    const second = socket(sockets, 1);
    second.readyState = 1;
    act(() => second.onopen?.(new Event("open")));
    await settle();
    expect(second.sent.length).toBeGreaterThan(0);

    // The device answers: the page is connected, and notes are carried.
    const reply = Uint8Array.from([
      0xf0, 0x7e, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0xf7,
    ]);
    act(() =>
      second.onmessage?.(new MessageEvent("message", { data: reply.buffer })),
    );
    await settle();
    expect(onMixer(container)).toBe(true);
    expect(container.textContent).not.toContain("Notes do not reach the device");
  });

  it("lets go of the service quietly when another port is chosen", async () => {
    const midi = fakeMidiAccess([{ name: "INTEGRA-7", answers: true }]);
    const { container, sockets } = serviceApp(midi);
    await settle();
    // The service socket is open and the device has not answered yet.
    const service = socket(sockets, 0);
    service.readyState = 1;
    act(() => service.onopen?.(new Event("open")));
    await settle();
    expect(container.textContent).toContain("Identifying");

    const select = container.querySelector("select");
    if (!select) throw new Error("no port selector");
    act(() => {
      select.value = [...select.options].find((o) => o.text === "INTEGRA-7")?.value ?? "";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await settle();

    // Asked once, not twice, and never told the device was gone.
    expect(midi.identified).toEqual(["INTEGRA-7"]);
    expect(container.textContent).not.toContain("Waiting for reconnection");
    expect(onMixer(container)).toBe(true);
  });
});

describe("the plugin transport", () => {
  it("connects once the service, unavailable at start, comes up", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const listeners = new Set<(message: string) => void>();
    const deliver = (message: object) => {
      for (const l of [...listeners]) l(JSON.stringify(message));
    };
    let up = false;
    const request = [...identity_request()].join();
    const reply = [
      0xf0, 0x7e, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0xf7,
    ];
    const bridge: PluginBridge = {
      post(text) {
        const m = JSON.parse(text) as PageMessage;
        const port = DEVICE_SERVICE_NAME;
        queueMicrotask(() => {
          if (m.type === "ready") {
            // As the plugin does: the reason, then the (empty) list.
            if (!up)
              deliver({
                type: "error",
                port: null,
                message: "Device service not running",
              });
            deliver({ type: "ports", ports: up ? [port] : [] });
          } else if (m.type === "open") {
            deliver(
              up
                ? { type: "opened", port }
                : { type: "openFailed", port, message: "Device service not running" },
            );
          } else if (m.type === "send" && up && m.data.join() === request) {
            deliver({ type: "receive", port, data: reply });
          }
        });
      },
      subscribe(listener) {
        listeners.add(listener);
        return () => listeners.delete(listener);
      },
    };
    const { container } = render(
      <App
        openPorts={() => pluginPorts(bridge)}
        connectOnlyPort
        storage={memoryStorage()}
      />,
    );
    await settle();
    expect(container.textContent).toContain("Device service not running");
    expect(onMixer(container)).toBe(false);

    up = true;
    deliver({ type: "ports", ports: [DEVICE_SERVICE_NAME] });
    await settle();
    expect(onMixer(container)).toBe(true);

    // The service goes away again: the page says so.
    up = false;
    deliver({ type: "ports", ports: [] });
    await settle();
    expect(onMixer(container)).toBe(false);
  });
});

describe("the browser's navigation", () => {
  it("is one row by scope, whose Edit opens the tone editor", async () => {
    const { container } = await open([{ name: "INTEGRA-7", answers: true }], remembering("INTEGRA-7"));
    const navs = container.querySelectorAll("nav");
    expect(navs).toHaveLength(1);
    const groups = [...navs[0]!.querySelectorAll('[role="group"]')].map((g) => g.getAttribute("aria-label"));
    expect(groups).toEqual(["Device", "Studio Set", "Part", "Tone"]);
    const edit = navs[0]!.querySelector('[aria-label="Tone"] button[data-page="toneEdit"]');
    if (!edit) throw new Error("no Edit in the Tone group");
    click(edit);
    expect(edit.getAttribute("aria-current")).toBe("page");
    expect(navs[0]!.querySelector('[data-page="mixer"]')?.getAttribute("aria-current")).toBeNull();
  });
});

describe("Studio Set Init", () => {
  it("runs from the navigation row, holds the Mixer until \"finished\", then reads it again", async () => {
    const { container, midi } = await open([{ name: "INTEGRA-7", answers: true }], remembering("INTEGRA-7"));
    // The Mixer page's notice that it is still reading the silent device:
    // the innermost div holding only that text.
    const mixer = () =>
      [...container.querySelectorAll("div")]
        .reverse()
        .find((d) => d.textContent === "Loading mixer state from device...")!;
    const studioSet = container.querySelector<HTMLElement>('nav [aria-label="Studio Set"]')!;
    click(button(studioSet, "Init…"));
    click(button(document.querySelector('[role="dialog"]')!, "Init"));
    await settle();
    expect(mixer().closest('[inert][aria-busy="true"]')).not.toBeNull();

    // The Init leaves after what the first mixer read queued, then the device
    // answers it. The Init's address shares its first three bytes with "finished".
    const initArea = [...initFinishedAddress()].slice(0, 3);
    const isInit = (bytes: number[]) => initArea.every((b, i) => bytes[7 + i] === b);
    await vi.waitFor(() => expect(midi.sent.some(({ bytes }) => isInit(bytes))).toBe(true), { timeout: 10_000 });
    const before = midi.sent.length;
    const input = [...midi.access.inputs.values()][0]!;
    act(() => {
      input.dispatchEvent(Object.assign(new Event("midimessage"), { data: new Uint8Array(dt1([...initFinishedAddress()], [0, 0, 0, 0])) }));
    });
    await settle();
    expect(mixer().closest("[inert]")).toBeNull();
    // The mixer is read again: data requests follow the Init.
    await vi.waitFor(() => expect(midi.sent.slice(before).some(({ bytes }) => bytes[6] === 0x11)).toBe(true));
  });
});
