/**
 * The Device service as a port (ADR 0006): the browser reaches the INTEGRA-7
 * through the service's localhost WebSocket instead of Web MIDI.
 *
 * The page is a raw SysEx and note client of the service, so the pair behaves
 * as a shared MIDI port: the SysEx it sends goes through the service's paced queue,
 * and every SysEx the device sends comes back. Its notes (channel voice
 * messages) go to the device at once, ahead of that queue, as a DAW track's
 * do. Each send is one binary frame, one whole message each way (documented
 * beside the server, in `crates/integral-device-service/src/websocket.rs`);
 * the service refuses a frame that is neither. A send with a future
 * `timestamp` (the Sequencer's) is held in the page until then, as Web MIDI
 * would hold it in the browser, on timers a background tab does not throttle
 * (`timedSend.ts`).
 *
 * The service may not be running, and may be restarted. The pair stays in the
 * port list either way, says so when a connect fails, and keeps retrying once
 * asked to open, so the page recovers when the service appears.
 */

import { deviceServiceUrl } from "../pkg/integral_wasm.js";
import type { MidiListener, MidiPortPair, MidiPorts } from "./midi";
import { holdUntil, workerTimers, type Timers } from "./timedSend";

/** What the port picker shows and the last-port memory keeps (the core's `PAIR_NAME`, checked by the tests). */
export const DEVICE_SERVICE_NAME = "Integral Device Service";

/** The pair's `id` (the core's `PAIR_ID`, checked by the tests). */
export const DEVICE_SERVICE_ID = "integral-device-service";

/** Wait before the first reconnect, then doubled up to {@link RECONNECT_MAX_MS}. */
export const RECONNECT_MIN_MS = 500;

/** Longest wait between reconnects. */
export const RECONNECT_MAX_MS = 5000;

/** What the user is told when the service does not answer. */
export const NOT_RUNNING_HINT =
  "Device service not running, or this page's origin isn't allowed (--ws-origin). Start it with `integral-cli bridge` (or `just bridge`); this page connects as soon as it can.";

/** What the user is told when the service closes the page: no device link. */
export const LINK_DOWN_HINT =
  "The Device service has lost the INTEGRA-7. This page reconnects when the service has it back.";

/** The WebSocket close code the service uses for "link down" (try again later). */
export const LINK_DOWN_CLOSE_CODE = 1013;

/** The part of the browser's `WebSocket` the pair uses, so tests can fake it. */
export interface SocketLike {
  binaryType: string;
  readonly readyState: number;
  send(data: Uint8Array): void;
  close(): void;
  onopen: ((event: Event) => void) | null;
  onclose: ((event: CloseEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  onmessage: ((event: MessageEvent) => void) | null;
}

/** Opens a WebSocket to `url`. */
export type SocketFactory = (url: string) => SocketLike;

/** `WebSocket.OPEN`, without needing the global in tests. */
const SOCKET_OPEN = 1;

/** A {@link MidiPortPair} over the service, with word of its link. */
export interface DeviceServicePair extends MidiPortPair {
  /** Stop reconnecting and close the socket. A later `open()` starts over. */
  release(): void;
}

/**
 * The Device service as a port pair over the WebSocket at `url`.
 *
 * Nothing connects until `open()`. From then on the pair holds a connection
 * and, whenever it is lost or cannot be made, retries with a growing wait
 * until `release()`. `timers` hold timed sends and reconnects; `now` is the
 * clock a send's `timestamp` is on.
 */
export function deviceServicePair(
  url: string,
  makeSocket: SocketFactory = (u) => new WebSocket(u),
  timers: Timers = workerTimers(),
  now: () => number = () => performance.now(),
): DeviceServicePair {
  const listeners = new Set<MidiListener>();
  const watchers = new Set<(up: boolean) => void>();
  let socket: SocketLike | null = null;
  let wanted = false;
  let up = false;
  let failure: string | null = null;
  let retry: unknown = null;
  let wait = RECONNECT_MIN_MS;
  // Settles the `open()` calls waiting for the first connection.
  let waiting: { resolve: () => void; reject: (e: Error) => void }[] = [];

  const setUp = (next: boolean) => {
    if (up === next) return;
    up = next;
    for (const watcher of [...watchers]) watcher(next);
  };

  const settle = (error: Error | null) => {
    const settled = waiting;
    waiting = [];
    for (const w of settled) error ? w.reject(error) : w.resolve();
  };

  const connect = () => {
    retry = null;
    const s = makeSocket(url);
    s.binaryType = "arraybuffer";
    socket = s;
    s.onopen = () => {
      if (socket !== s) return;
      failure = null;
      wait = RECONNECT_MIN_MS;
      setUp(true);
      settle(null);
    };
    s.onmessage = (event) => {
      if (socket !== s || !(event.data instanceof ArrayBuffer)) return;
      const data = new Uint8Array(event.data);
      for (const listener of [...listeners]) listener(data);
    };
    // `onerror` is always followed by `onclose`, which does the work.
    s.onclose = (event) => {
      if (socket !== s) return;
      socket = null;
      failure =
        event.code === LINK_DOWN_CLOSE_CODE ? LINK_DOWN_HINT : NOT_RUNNING_HINT;
      setUp(false);
      settle(new Error(failure));
      if (wanted) {
        retry = timers.set(connect, wait);
        wait = Math.min(wait * 2, RECONNECT_MAX_MS);
      }
    };
  };

  return {
    id: DEVICE_SERVICE_ID,
    name: DEVICE_SERVICE_NAME,
    carriesNotes: true,
    open() {
      wanted = true;
      if (up) return Promise.resolve();
      return new Promise<void>((resolve, reject) => {
        waiting.push({ resolve, reject });
        if (socket === null && retry === null) connect();
        // A retry already scheduled has its turn; this call waits for it.
      });
    },
    send(data, timestamp) {
      // Like a MIDI output that has lost its device, a closed link drops
      // the message; `watch` tells the page the link is down. A held message
      // goes only on the connection it was sent to: after a release or a
      // reconnect, a note-on could otherwise land with no note-off.
      const target = socket;
      if (target?.readyState !== SOCKET_OPEN) return;
      const frame = Uint8Array.from(data);
      const deliver = () => {
        if (socket === target && target.readyState === SOCKET_OPEN)
          target.send(frame);
      };
      holdUntil(timestamp, deliver, timers, now);
    },
    listen(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    watch(listener) {
      watchers.add(listener);
      return () => {
        watchers.delete(listener);
      };
    },
    failure: () => failure,
    release() {
      wanted = false;
      if (retry !== null) timers.clear(retry);
      retry = null;
      const s = socket;
      socket = null;
      s?.close();
      // Letting go is not the link dropping: nobody is told.
      up = false;
      settle(new Error("closed"));
    },
  };
}

/**
 * `ports` with the Device service listed beside them. Where the browser has
 * no Web MIDI, `ports` is `null` and the service is the only port.
 */
export function withDeviceService(
  ports: MidiPorts | null,
  service: DeviceServicePair,
): MidiPorts {
  return {
    pairs: () => [...(ports?.pairs() ?? []), service],
    onChange: (listener) => ports?.onChange(listener) ?? (() => {}),
    close() {
      ports?.close?.();
      service.release();
    },
  };
}

/**
 * The ports of a browser: Web MIDI's, if there is Web MIDI, and the Device
 * service.
 */
export async function browserPorts(
  webMidi: () => Promise<MidiPorts>,
): Promise<MidiPorts> {
  let midi: MidiPorts | null = null;
  try {
    midi = await webMidi();
  } catch {
    // A browser without Web MIDI can still use the service.
  }
  return withDeviceService(midi, deviceServicePair(deviceServiceUrl()));
}
