import { afterEach, describe, expect, it, vi } from "vitest";
import { identity_request } from "../pkg/integral_wasm.js";
import { mixerScreen } from "./layouts/mixer/screen";
import { naturalSize } from "./layouts/zoom";
import { identifyDevice } from "./midi";
import {
  OPEN_TIMEOUT_MS,
  PLUGIN_EVENT,
  pluginGrid,
  decodePluginMessage,
  encodePageMessage,
  pluginPorts,
  pluginWindowSize,
  resizePluginWindow,
  windowBridge,
  type PageMessage,
  type PluginBridge,
  type PluginMessage,
} from "./plugin";

/** A bridge standing in for the plugin: records what the page posts and
 *  delivers what the test sends. */
function fakeBridge() {
  const posted: PageMessage[] = [];
  const listeners = new Set<(message: string) => void>();
  let answer: ((message: PageMessage) => PluginMessage[]) | null = null;
  const deliver = (message: PluginMessage) => {
    for (const l of [...listeners]) l(JSON.stringify(message));
  };
  const bridge: PluginBridge = {
    post(text) {
      const message = JSON.parse(text) as PageMessage;
      posted.push(message);
      for (const reply of answer?.(message) ?? []) queueMicrotask(() => deliver(reply));
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  return {
    bridge,
    posted,
    deliver,
    /** Answer each posted message with what `reply` returns. */
    answerWith(reply: (message: PageMessage) => PluginMessage[]) {
      answer = reply;
    },
    rawDeliver(text: string) {
      for (const l of [...listeners]) l(text);
    },
  };
}

/** Ports from a fake plugin that lists `names` on `ready`. */
async function listed(names: string[]) {
  const fake = fakeBridge();
  fake.answerWith((m) => (m.type === "ready" ? [{ type: "ports", ports: names }] : []));
  const ports = await pluginPorts(fake.bridge);
  return { fake, ports };
}

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("plugin message codec", () => {
  it("encodes page messages as tagged JSON", () => {
    expect(encodePageMessage({ type: "send", port: "I7", data: [0xf0, 0xf7] })).toBe(
      '{"type":"send","port":"I7","data":[240,247]}',
    );
    expect(encodePageMessage({ type: "resize", width: 800, height: 600 })).toBe(
      '{"type":"resize","width":800,"height":600}',
    );
  });

  it("decodes every plugin message", () => {
    expect(decodePluginMessage('{"type":"ports","ports":["A","B"]}')).toEqual({
      type: "ports",
      ports: ["A", "B"],
    });
    expect(decodePluginMessage('{"type":"opened","port":"A"}')).toEqual({
      type: "opened",
      port: "A",
    });
    expect(decodePluginMessage('{"type":"receive","port":"A","data":[240,1,247]}')).toEqual({
      type: "receive",
      port: "A",
      data: [240, 1, 247],
    });
    expect(decodePluginMessage('{"type":"openFailed","port":"A","message":"gone"}')).toEqual({
      type: "openFailed",
      port: "A",
      message: "gone",
    });
    expect(decodePluginMessage('{"type":"error","port":null,"message":"x"}')).toEqual({
      type: "error",
      port: null,
      message: "x",
    });
  });

  it("rejects malformed messages", () => {
    for (const text of [
      "not json",
      "null",
      '"ports"',
      '{"type":"nope"}',
      '{"type":"ports","ports":[1]}',
      '{"type":"receive","port":"A","data":[256]}',
      '{"type":"receive","port":"A","data":[1.5]}',
      '{"type":"receive","port":7,"data":[]}',
      '{"type":"error","port":"A"}',
    ]) {
      expect(decodePluginMessage(text), text).toBeNull();
    }
  });
});

describe("windowBridge", () => {
  it("is null in a browser, which has no window.ipc", () => {
    expect(windowBridge(window)).toBeNull();
  });

  it("posts through window.ipc and hears the plugin's events", () => {
    const postMessage = vi.fn();
    const win = Object.assign(new EventTarget(), { ipc: { postMessage } }) as unknown as Window;
    const bridge = windowBridge(win)!;
    const heard: string[] = [];
    const stop = bridge.subscribe((m) => heard.push(m));

    bridge.post("hello");
    win.dispatchEvent(new CustomEvent(PLUGIN_EVENT, { detail: '{"type":"opened","port":"A"}' }));
    stop();
    win.dispatchEvent(new CustomEvent(PLUGIN_EVENT, { detail: "late" }));

    expect(postMessage).toHaveBeenCalledWith("hello");
    expect(heard).toEqual(['{"type":"opened","port":"A"}']);
  });
});

describe("pluginPorts", () => {
  it("says ready, then offers the pairs the plugin lists", async () => {
    const { fake, ports } = await listed(["INTEGRA-7", "Network Integra"]);
    expect(fake.posted[0]).toEqual({ type: "ready" });
    expect(ports.pairs().map((p) => [p.id, p.name])).toEqual([
      ["INTEGRA-7", "INTEGRA-7"],
      ["Network Integra", "Network Integra"],
    ]);
  });

  it("reports a changed list, keeping each pair's identity", async () => {
    const { fake, ports } = await listed(["A"]);
    const a = ports.pairs()[0];
    const onChange = vi.fn();
    ports.onChange(onChange);
    fake.deliver({ type: "ports", ports: ["A", "B"] });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(ports.pairs()[0]).toBe(a);
    expect(ports.pairs().map((p) => p.name)).toEqual(["A", "B"]);
  });

  it("opens a pair through the plugin", async () => {
    const { fake, ports } = await listed(["A"]);
    fake.answerWith((m) => (m.type === "open" ? [{ type: "opened", port: m.port }] : []));
    await ports.pairs()[0]!.open();
    expect(fake.posted).toContainEqual({ type: "open", port: "A" });
  });

  it("rejects an open the plugin refuses", async () => {
    const { fake, ports } = await listed(["A"]);
    fake.answerWith((m) =>
      m.type === "open" ? [{ type: "openFailed", port: m.port, message: "gone" }] : [],
    );
    await expect(ports.pairs()[0]!.open()).rejects.toThrow("gone");
  });

  it("keeps an open waiting through another request's error", async () => {
    const { fake, ports } = await listed(["A"]);
    const open = ports.pairs()[0]!.open();
    vi.spyOn(console, "error").mockImplementation(() => {});
    fake.deliver({ type: "error", port: "A", message: "send failed" });
    fake.deliver({ type: "opened", port: "A" });
    await expect(open).resolves.toBeUndefined();
  });

  it("gives up on an open the plugin never answers", async () => {
    const { ports } = await listed(["A"]);
    vi.useFakeTimers();
    const open = ports.pairs()[0]!.open();
    vi.advanceTimersByTime(OPEN_TIMEOUT_MS);
    await expect(open).rejects.toThrow("no answer");
  });

  it("stops listening once closed", async () => {
    const { fake, ports } = await listed(["A"]);
    const onChange = vi.fn();
    ports.onChange(onChange);
    ports.close?.();
    fake.deliver({ type: "ports", ports: ["A", "B"] });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("sends bytes to its own pair", async () => {
    const { fake, ports } = await listed(["A"]);
    ports.pairs()[0]!.send(new Uint8Array([0x90, 60, 100]));
    expect(fake.posted[fake.posted.length - 1]).toEqual({ type: "send", port: "A", data: [0x90, 60, 100] });
  });

  it("holds a timestamped send until its time", async () => {
    const { fake, ports } = await listed(["A"]);
    vi.useFakeTimers();
    ports.pairs()[0]!.send([0x80, 60, 0], performance.now() + 50);
    expect(fake.posted.some((m) => m.type === "send")).toBe(false);
    vi.advanceTimersByTime(50);
    expect(fake.posted[fake.posted.length - 1]).toEqual({ type: "send", port: "A", data: [0x80, 60, 0] });
  });

  it("hands each pair only its own messages, until unsubscribed", async () => {
    const { fake, ports } = await listed(["A", "B"]);
    const [a, b] = ports.pairs();
    const heardA: number[][] = [];
    const heardB: number[][] = [];
    const stopA = a!.listen((d) => heardA.push([...d]));
    b!.listen((d) => heardB.push([...d]));

    fake.deliver({ type: "receive", port: "A", data: [1] });
    fake.deliver({ type: "receive", port: "B", data: [2] });
    stopA();
    fake.deliver({ type: "receive", port: "A", data: [3] });

    expect(heardA).toEqual([[1]]);
    expect(heardB).toEqual([[2]]);
  });

  it("ignores a malformed message", async () => {
    const { fake, ports } = await listed(["A"]);
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const heard = vi.fn();
    ports.pairs()[0]!.listen(heard);
    fake.rawDeliver('{"type":"receive","port":"A","data":"nope"}');
    expect(heard).not.toHaveBeenCalled();
  });

  it("identifies an INTEGRA-7 over the plugin transport", async () => {
    const reply: PluginMessage = {
      type: "receive",
      port: "A",
      data: [0xf0, 0x7e, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf7],
    };
    const request = [...identity_request()];
    const { fake, ports } = await listed(["A"]);
    fake.answerWith((m) => {
      if (m.type === "open") return [{ type: "opened", port: m.port }];
      if (m.type === "send" && m.data.join() === request.join()) return [reply];
      return [];
    });
    const identity = await identifyDevice(ports.pairs()[0]!, 1000);
    expect(identity?.isIntegra7()).toBe(true);
    expect(identity?.device_id).toBe(0x10);
  });

  it("identifies nothing on a pair that fails to open", async () => {
    const { fake, ports } = await listed(["A"]);
    fake.answerWith((m) =>
      m.type === "open" ? [{ type: "openFailed", port: m.port, message: "busy" }] : [],
    );
    expect(await identifyDevice(ports.pairs()[0]!, 1000)).toBeNull();
  });
});

describe("plugin window size", () => {
  const roomy = { availWidth: 10_000, availHeight: 10_000 };

  it("is the Grid's natural size times zoom", () => {
    expect(pluginWindowSize(naturalSize({ w: 40, h: 20 }), 1, roomy)).toEqual({
      width: 960,
      height: 480,
    });
    expect(pluginWindowSize(naturalSize({ w: 40, h: 20 }), 1.25, roomy)).toEqual({
      width: 1200,
      height: 600,
    });
  });

  it("is no larger than the screen", () => {
    expect(
      pluginWindowSize(naturalSize({ w: 40, h: 20 }), 1, { availWidth: 800, availHeight: 400 }),
    ).toEqual({ width: 800, height: 400 });
  });

  it("ignores an unknown screen", () => {
    expect(
      pluginWindowSize(naturalSize({ w: 40, h: 20 }), 1, { availWidth: 0, availHeight: 0 }),
    ).toEqual({ width: 960, height: 480 });
  });

  it("sizes the plugin Grid from the larger mixer screen", () => {
    const wide = mixerScreen(true).layout;
    expect(wide.w).toBeGreaterThan(mixerScreen(false).layout.w);
    expect(pluginGrid()).toEqual({ w: wide.w, h: Math.max(wide.h, mixerScreen(false).layout.h) });
  });

  it("asks the plugin for the plugin Grid's window", () => {
    const fake = fakeBridge();
    resizePluginWindow(fake.bridge, 1, roomy);
    const { width, height } = naturalSize(pluginGrid()).px;
    expect(fake.posted).toEqual([{ type: "resize", width, height }]);
  });
});
