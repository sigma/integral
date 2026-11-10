import { describe, expect, it, vi } from "vitest";
import {
  DEVICE_SERVICE_ID,
  DEVICE_SERVICE_NAME,
  LINK_DOWN_CLOSE_CODE,
  LINK_DOWN_HINT,
  NOT_RUNNING_HINT,
  RECONNECT_MAX_MS,
  RECONNECT_MIN_MS,
  browserPorts,
  deviceServicePair,
  withDeviceService,
  type SocketLike,
  type Timers,
} from "./deviceService";
import { findIntegraPort, type MidiPortPair, type MidiPorts } from "./midi";
import {
  devServerPort,
  deviceServicePairId,
  deviceServicePairName,
  deviceServiceUrl,
  previewPort,
} from "../pkg/integral_wasm.js";
import { DEV_SERVER_PORT, PREVIEW_PORT } from "./devPorts";

const URL = "ws://127.0.0.1:1";

/** The pair's clock, which stands still in these tests. */
const NOW = 1000;

/** A WebSocket the test drives by hand. */
class FakeSocket implements SocketLike {
  binaryType = "blob";
  readyState = 0;
  sent: Uint8Array[] = [];
  closed = false;
  onopen: SocketLike["onopen"] = null;
  onclose: SocketLike["onclose"] = null;
  onerror: SocketLike["onerror"] = null;
  onmessage: SocketLike["onmessage"] = null;

  constructor(readonly url: string) {}
  send(data: Uint8Array) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
  }
  /** The server accepted. */
  accept() {
    this.readyState = 1;
    this.onopen?.(new Event("open"));
  }
  /** The connection ended, or never began. */
  end() {
    this.readyState = 3;
    this.onclose?.(new CloseEvent("close"));
  }
  receive(data: unknown) {
    this.onmessage?.(new MessageEvent("message", { data }));
  }
}

/** Timers that run only when the test says. */
function manualTimers() {
  const pending = new Map<number, { callback: () => void; ms: number }>();
  let next = 0;
  const timers: Timers = {
    set(callback, ms) {
      pending.set(++next, { callback, ms });
      return next;
    },
    clear(handle) {
      pending.delete(handle as number);
    },
  };
  return {
    timers,
    pending,
    /** Run the one scheduled retry; returns the wait it was given. */
    fire(): number {
      const first = [...pending][0];
      if (!first) throw new Error("no retry scheduled");
      const [id, { callback, ms }] = first;
      pending.delete(id);
      callback();
      return ms;
    },
  };
}

function setup() {
  const sockets: FakeSocket[] = [];
  const clock = manualTimers();
  const pair = deviceServicePair(
    URL,
    (url) => {
      const socket = new FakeSocket(url);
      sockets.push(socket);
      return socket;
    },
    clock.timers,
    () => NOW,
  );
  const last = () => {
    const socket = sockets[sockets.length - 1];
    if (!socket) throw new Error("no socket opened");
    return socket;
  };
  return { pair, sockets, clock, last };
}

const bytes = (...b: number[]) => new Uint8Array(b);

describe("deviceServicePair", () => {
  it("is named as the picker shows it and does not connect until opened", () => {
    const { pair, sockets } = setup();
    expect(pair.name).toBe(DEVICE_SERVICE_NAME);
    expect(sockets).toHaveLength(0);
  });

  it("opens one binary socket to the URL, and open is idempotent", async () => {
    const { pair, sockets, last } = setup();
    const first = pair.open();
    const second = pair.open();
    expect(sockets).toHaveLength(1);
    expect(last().url).toBe(URL);
    expect(last().binaryType).toBe("arraybuffer");
    last().accept();
    await Promise.all([first, second]);
    await pair.open();
    expect(sockets).toHaveLength(1);
  });

  it("sends each message as one binary frame", async () => {
    const { pair, last } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    pair.send([0xf0, 0x7e, 0xf7]);
    pair.send(bytes(0xf0, 0x41, 0xf7), NOW);

    expect(last().sent).toEqual([
      bytes(0xf0, 0x7e, 0xf7),
      bytes(0xf0, 0x41, 0xf7),
    ]);
  });

  it("forwards notes as frames of their own, as the service plays them", async () => {
    const { pair, last } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    pair.send([0x90, 0x3c, 0x7f]);
    pair.send(bytes(0x80, 0x3c, 0x00));
    pair.send([0xb0, 0x7b, 0x00]);

    expect(last().sent).toEqual([
      bytes(0x90, 0x3c, 0x7f),
      bytes(0x80, 0x3c, 0x00),
      bytes(0xb0, 0x7b, 0x00),
    ]);
  });

  it("holds a send with a future timestamp until then, as Web MIDI does", async () => {
    const { pair, last, clock } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    pair.send([0x90, 0x3c, 0x7f], NOW + 50);
    expect(last().sent).toEqual([]);
    expect(clock.fire()).toBe(50);

    expect(last().sent).toEqual([bytes(0x90, 0x3c, 0x7f)]);
  });

  it("drops a held send whose time comes while the link is down", async () => {
    const { pair, last, clock } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    pair.send([0x90, 0x3c, 0x7f], NOW + 50);
    const socket = last();
    pair.release();
    clock.fire();

    expect(socket.sent).toEqual([]);
  });

  it("never sends a held message on a later connection", async () => {
    const { pair, last, clock } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    pair.send([0x90, 0x3c, 0x7f], NOW + 50);
    pair.release();
    const reopening = pair.open();
    last().accept();
    await reopening;
    clock.fire();

    // A note-on whose note-off was lost with the first connection.
    expect(last().sent).toEqual([]);
  });

  it("drops a message sent while the link is down", () => {
    const { pair, last } = setup();
    pair.open().catch(() => {});
    pair.send([0xf0, 0xf7]);
    expect(last().sent).toEqual([]);
  });

  it("hands every binary frame to every listener, and skips text", async () => {
    const { pair, last } = setup();
    const a = vi.fn();
    const b = vi.fn();
    pair.listen(a);
    const stopB = pair.listen(b);
    const opening = pair.open();
    last().accept();
    await opening;

    last().receive(bytes(0xf0, 0x7e, 0xf7).buffer);
    last().receive("text");
    stopB();
    last().receive(bytes(0xf0, 0x41, 0xf7).buffer);

    expect(a.mock.calls).toEqual([
      [bytes(0xf0, 0x7e, 0xf7)],
      [bytes(0xf0, 0x41, 0xf7)],
    ]);
    expect(b.mock.calls).toEqual([[bytes(0xf0, 0x7e, 0xf7)]]);
  });

  it("says the service is not running when it cannot connect", async () => {
    const { pair, last } = setup();
    expect(pair.failure?.()).toBeNull();
    const opening = pair.open();
    last().end();

    await expect(opening).rejects.toThrow(NOT_RUNNING_HINT);
    expect(pair.failure?.()).toBe(NOT_RUNNING_HINT);
  });

  it("declares that it carries notes", () => {
    expect(setup().pair.carriesNotes).toBe(true);
  });

  it("says the device link is down when the service closes with try-again-later", async () => {
    const { pair, last } = setup();
    const opening = pair.open();
    last().accept();
    await opening;

    last().onclose?.(new CloseEvent("close", { code: LINK_DOWN_CLOSE_CODE }));

    expect(pair.failure?.()).toBe(LINK_DOWN_HINT);
  });

  it("names the origin among the reasons it may be refused", () => {
    expect(NOT_RUNNING_HINT).toContain("not running");
    expect(NOT_RUNNING_HINT).toContain("origin isn't allowed (--ws-origin)");
  });

  it("retries with a growing wait, connects when the service appears, and tells the watcher", async () => {
    const { pair, sockets, clock, last } = setup();
    const heard: boolean[] = [];
    pair.watch?.((up) => heard.push(up));
    const listener = vi.fn();
    pair.listen(listener);
    pair.open().catch(() => {});
    last().end();

    const waits = [clock.fire()];
    last().end();
    waits.push(clock.fire());
    last().accept();

    expect(waits).toEqual([RECONNECT_MIN_MS, RECONNECT_MIN_MS * 2]);
    expect(sockets).toHaveLength(3);
    expect(heard).toEqual([true]);
    expect(pair.failure?.()).toBeNull();
    // The listener registered before the connection still hears it.
    last().receive(bytes(0xf0, 0xf7).buffer);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("never waits longer than the maximum between retries", () => {
    const { pair, clock, last } = setup();
    pair.open().catch(() => {});
    const waits: number[] = [];
    for (let i = 0; i < 8; i++) {
      last().end();
      waits.push(clock.fire());
    }
    expect(Math.max(...waits)).toBe(RECONNECT_MAX_MS);
  });

  it("goes down and reconnects when the service restarts", async () => {
    const { pair, sockets, clock, last } = setup();
    const heard: boolean[] = [];
    pair.watch?.((up) => heard.push(up));
    const opening = pair.open();
    last().accept();
    await opening;

    last().end();
    expect(heard).toEqual([true, false]);
    expect(clock.fire()).toBe(RECONNECT_MIN_MS);
    last().accept();

    expect(sockets).toHaveLength(2);
    expect(heard).toEqual([true, false, true]);
  });

  it("stops retrying and closes once released", async () => {
    const { pair, clock, last } = setup();
    const opening = pair.open();
    last().accept();
    await opening;
    const socket = last();

    pair.release();

    expect(socket.closed).toBe(true);
    socket.end(); // a late close from the dropped socket changes nothing
    expect(clock.pending.size).toBe(0);
  });

  it("lets go without telling the watchers", async () => {
    const { pair, last } = setup();
    const heard: boolean[] = [];
    pair.watch?.((up) => heard.push(up));
    const opening = pair.open();
    last().accept();
    await opening;

    pair.release();

    expect(heard).toEqual([true]);
  });

  it("cancels a scheduled retry when released", () => {
    const { pair, clock, last } = setup();
    pair.open().catch(() => {});
    last().end();
    expect(clock.pending.size).toBe(1);

    pair.release();

    expect(clock.pending.size).toBe(0);
  });
});

describe("the ports of a browser", () => {
  const midiPair = { id: "m", name: "INTEGRA-7" } as MidiPortPair;
  const midi: MidiPorts = {
    pairs: () => [midiPair],
    onChange: () => () => {},
    close: vi.fn(),
  };

  it("lists the service after the Web MIDI pairs", () => {
    const service = setup().pair;
    const ports = withDeviceService(midi, service);
    expect(ports.pairs().map((p) => p.name)).toEqual([
      "INTEGRA-7",
      DEVICE_SERVICE_NAME,
    ]);
  });

  it("closes both on close", () => {
    const service = setup().pair;
    const dispose = vi.spyOn(service, "release");
    withDeviceService(midi, service).close?.();
    expect(midi.close).toHaveBeenCalled();
    expect(dispose).toHaveBeenCalled();
  });

  it("offers the service alone where there is no Web MIDI", async () => {
    const ports = await browserPorts(() =>
      Promise.reject(new Error("no Web MIDI")),
    );
    expect(ports.pairs().map((p) => p.name)).toEqual([DEVICE_SERVICE_NAME]);
    ports.close?.();
  });

  it("pins the servers to the ports the service admits", () => {
    expect(DEV_SERVER_PORT).toBe(devServerPort());
    expect(PREVIEW_PORT).toBe(previewPort());
  });

  it("names the pair as the core does, as the plugin offers it", () => {
    expect(DEVICE_SERVICE_NAME).toBe(deviceServicePairName());
    expect(DEVICE_SERVICE_ID).toBe(deviceServicePairId());
  });

  it("takes the URL from the core", async () => {
    expect(deviceServiceUrl()).toBe("ws://127.0.0.1:4707");
  });

  it("is never the default port, though its name says Integral", () => {
    const service = setup().pair;
    expect(findIntegraPort([service])).toBeUndefined();
    expect(findIntegraPort([service, midiPair])).toBe(midiPair);
  });
});
