/**
 * The plugin transport (ADR 0004, #365): how the page reaches the device
 * when it runs in the plugin's webview.
 *
 * WKWebView has no Web MIDI, and the host does not pass plugin SysEx
 * through, so the plugin relays bytes over the webview's IPC to the Device
 * service (ADR 0006), the one process that holds the device port. The only
 * port it offers is the service, and none while the service or the device
 * is unavailable. This module is the page's half: a {@link MidiPorts}
 * over a {@link PluginBridge}. It does not pretend to be Web MIDI.
 *
 * The page paces its own sends (`WasmDeviceState.drain`), as on the web; the
 * plugin forwards bytes as they come and does not throttle a second time.
 *
 * Wire format: one JSON object per message, tagged by `type`. The Rust half
 * is `crates/integral-vst/src/webview/ipc.rs`; the two are kept in step by
 * the tests on each side.
 */

import { DEVICE_SERVICE_ID, DEVICE_SERVICE_NAME } from "./deviceService";
import { mixerScreen } from "./layouts/mixer/screen";
import { naturalSize, type NaturalSize } from "./layouts/zoom";
import type { MidiListener, MidiPortPair, MidiPorts } from "./midi";
import type { Cells } from "./synth-ui/grid";

/** A message from the page to the plugin. */
export type PageMessage =
  /** The page is up: the plugin answers with `ports`, and delivers from now on. */
  | { type: "ready" }
  /** Open the pair named `port` (closing any other); answered by `opened` or `error`. */
  | { type: "open"; port: string }
  /** Send one MIDI message to the open pair `port`. */
  | { type: "send"; port: string; data: number[] }
  /** Resize the plugin window to `width` × `height` logical px. */
  | { type: "resize"; width: number; height: number }
  /**
   * Bind the instance to a Part and, for a Kit plugin, a key (absent: the
   * whole kit). The plugin takes only what its kind accepts; the answer is
   * `instance`, or `error`.
   */
  | { type: "bind"; part?: number; key?: number }
  /** Begin a host gesture on the host parameter `id`: the first `input` of a Control drag. */
  | { type: "beginEdit"; id: string }
  /** Move the parameter `id`, which has a gesture open, to the raw `value`. */
  | { type: "performEdit"; id: string; value: number }
  /** End the gesture on the parameter `id`: the `change` of a Control drag. */
  | { type: "endEdit"; id: string };

/** What an instance is bound to: a Part (0-based) and, for a key, the key. */
export interface PluginBinding {
  part?: number;
  key?: number;
}

/** Something the instance reports besides its status (recalling, a mismatch, a duplicate). */
export interface PluginNotice {
  /** Which notice, as the plugin names it. */
  notice: string;
  /** What the editor shows. */
  text: string;
}

/** What the plugin says about the instance the page is the editor of. */
export interface InstanceInfo {
  /** The plugin kind, as core names it (`plugin_kind_from_wire`). */
  plugin: string;
  /** Where the instance is bound; `null` before it has taken a target. */
  binding: PluginBinding | null;
  /** The status code as the plugin names it; the page shows `statusText` and knows no list of codes. */
  status: string;
  /** The status as the editor shows it. */
  statusText: string;
  /** Whether the instance can tell the host of edits; if not, the page sends no host gestures. */
  hostEdits: boolean;
  notices: PluginNotice[];
}

/** A message from the plugin to the page. */
export type PluginMessage =
  /** The pairs the plugin can open, by name. Sent on `ready` and on change. */
  | { type: "ports"; ports: string[] }
  /** The pair `port` is open. */
  | { type: "opened"; port: string }
  /** The pair `port` would not open. */
  | { type: "openFailed"; port: string; message: string }
  /** A message `port` received. */
  | { type: "receive"; port: string; data: number[] }
  /** What the instance is and how it is doing. Sent on `ready` and on change. */
  | ({ type: "instance" } & InstanceInfo)
  /** The plugin refused a request about the instance (a `bind`, a host gesture). */
  | { type: "refused"; message: string }
  /** A request other than an open failed; `port` names the pair it was about, if any. */
  | { type: "error"; port: string | null; message: string };

/** The window event the plugin dispatches each message on, as a JSON string `detail`. */
export const PLUGIN_EVENT = "integral-plugin";

/** Encode a page message for the bridge. */
export function encodePageMessage(message: PageMessage): string {
  return JSON.stringify(message);
}

const isByte = (v: unknown): v is number =>
  typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= 0xff;

const isBytes = (v: unknown): v is number[] => Array.isArray(v) && v.every(isByte);

const isBinding = (v: unknown): v is PluginBinding => {
  if (typeof v !== "object" || v === null) return false;
  const b = v as Record<string, unknown>;
  return (b.part === undefined || isByte(b.part)) && (b.key === undefined || isByte(b.key));
};

const isNotice = (v: unknown): v is PluginNotice => {
  if (typeof v !== "object" || v === null) return false;
  const n = v as Record<string, unknown>;
  return typeof n.notice === "string" && typeof n.text === "string";
};

/** Decode a plugin message, or `null` for anything malformed. */
export function decodePluginMessage(text: string): PluginMessage | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) return null;
  const m = value as Record<string, unknown>;
  switch (m.type) {
    case "ports":
      return Array.isArray(m.ports) && m.ports.every((p) => typeof p === "string")
        ? { type: "ports", ports: m.ports as string[] }
        : null;
    case "opened":
      return typeof m.port === "string" ? { type: "opened", port: m.port } : null;
    case "openFailed":
      return typeof m.port === "string" && typeof m.message === "string"
        ? { type: "openFailed", port: m.port, message: m.message }
        : null;
    case "receive":
      return typeof m.port === "string" && isBytes(m.data)
        ? { type: "receive", port: m.port, data: m.data }
        : null;
    case "instance":
      return typeof m.plugin === "string" &&
        (m.binding === null || isBinding(m.binding)) &&
        typeof m.status === "string" &&
        typeof m.statusText === "string" &&
        typeof m.hostEdits === "boolean" &&
        Array.isArray(m.notices) &&
        m.notices.every(isNotice)
        ? {
            type: "instance",
            plugin: m.plugin,
            binding: m.binding,
            status: m.status,
            statusText: m.statusText,
            hostEdits: m.hostEdits,
            notices: m.notices,
          }
        : null;
    case "refused":
      return typeof m.message === "string" ? { type: "refused", message: m.message } : null;
    case "error":
      return (m.port === null || typeof m.port === "string") && typeof m.message === "string"
        ? { type: "error", port: m.port, message: m.message }
        : null;
    default:
      return null;
  }
}

/** The page's end of the plugin's IPC: strings out, strings in. */
export interface PluginBridge {
  /** Hand `message` to the plugin. */
  post(message: string): void;
  /** Call `listener` with each message from the plugin. Returns its unsubscribe. */
  subscribe(listener: (message: string) => void): () => void;
}

/** What wry gives the page: `window.ipc.postMessage`. */
interface WryWindow {
  ipc?: { postMessage?: (message: string) => void };
}

/**
 * The bridge to the plugin hosting this page, or `null` in a browser. wry
 * defines `window.ipc`; the plugin delivers by dispatching {@link PLUGIN_EVENT}
 * on `win`.
 */
export function windowBridge(win: Window): PluginBridge | null {
  const ipc = (win as Window & WryWindow).ipc;
  const postMessage = ipc?.postMessage;
  if (typeof postMessage !== "function") return null;
  return {
    post: (message) => postMessage.call(ipc, message),
    subscribe(listener) {
      const handler = (event: Event) => {
        const detail = (event as CustomEvent<unknown>).detail;
        if (typeof detail === "string") listener(detail);
      };
      win.addEventListener(PLUGIN_EVENT, handler);
      return () => win.removeEventListener(PLUGIN_EVENT, handler);
    },
  };
}

/** How long an `open` waits for the plugin's answer. */
export const OPEN_TIMEOUT_MS = 2000;

/** What waits on an `open` answer. */
interface Opening {
  resolve: () => void;
  reject: (error: Error) => void;
}

/** What {@link MidiPortPair.failure} says when the plugin gave no reason. */
export const SERVICE_UNAVAILABLE_HINT = "The Device service is unavailable.";

/**
 * The plugin's one port, the Device service, once the plugin has answered
 * `ready`. The pair is always listed, so the page can say why it is
 * unreachable ({@link MidiPortPair.failure}): the plugin lists the port
 * only while the service and the device are up, and says why otherwise.
 * The pair's `watch` tells the page when the plugin starts or stops
 * offering the port (the service or the device going away or coming back).
 * One pair is open at a time. The plugin carries SysEx only
 * (`carriesNotes` is `false`).
 *
 * A send with a future `timestamp` (the Sequencer's) is held in the page
 * until then, as Web MIDI would hold it in the browser.
 */
export function pluginPorts(bridge: PluginBridge): Promise<MidiPorts> {
  const post = (message: PageMessage) => bridge.post(encodePageMessage(message));
  const listeners = new Set<MidiListener>();
  const watchers = new Set<(up: boolean) => void>();
  let openings: Opening[] = [];
  let available = false;
  let reason: string | null = null;
  let heard = false;
  let changed: (() => void) | null = null;

  function settle(error: Error | null): void {
    const waiting = openings;
    openings = [];
    for (const w of waiting) {
      if (error) w.reject(error);
      else w.resolve();
    }
  }

  const pair: MidiPortPair = {
    id: DEVICE_SERVICE_ID,
    name: DEVICE_SERVICE_NAME,
    // The plugin's link carries SysEx only (#480 brings notes).
    carriesNotes: false,
    open: () =>
      new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => {
          openings = openings.filter((w) => w !== waiting);
          reject(new Error(`no answer opening ${DEVICE_SERVICE_NAME}`));
        }, OPEN_TIMEOUT_MS);
        const waiting: Opening = {
          resolve: () => {
            clearTimeout(timer);
            resolve();
          },
          reject: (error) => {
            clearTimeout(timer);
            reject(error);
          },
        };
        openings.push(waiting);
        post({ type: "open", port: DEVICE_SERVICE_NAME });
      }),
    send(data, timestamp) {
      const message: PageMessage = {
        type: "send",
        port: DEVICE_SERVICE_NAME,
        data: Array.from(data),
      };
      const delay = timestamp === undefined ? 0 : timestamp - performance.now();
      if (delay > 0) setTimeout(() => post(message), delay);
      else post(message);
    },
    listen(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    watch(listener) {
      watchers.add(listener);
      return () => {
        watchers.delete(listener);
      };
    },
    failure: () => (available ? null : (reason ?? SERVICE_UNAVAILABLE_HINT)),
  };

  const ports: MidiPorts = {
    pairs: () => (heard ? [pair] : []),
    onChange(listener) {
      changed = listener;
      return () => {
        if (changed === listener) changed = null;
      };
    },
  };

  let unsubscribe: () => void = () => {};
  ports.close = () => unsubscribe();

  return new Promise((resolve) => {
    unsubscribe = bridge.subscribe((text) => {
      const message = decodePluginMessage(text);
      if (!message) {
        console.warn("[plugin] ignored a malformed message", text);
        return;
      }
      switch (message.type) {
        case "ports": {
          const first = !heard;
          heard = true;
          const was = available;
          available = message.ports.includes(DEVICE_SERVICE_NAME);
          if (available) reason = null;
          if (available !== was) for (const w of [...watchers]) w(available);
          if (first) resolve(ports);
          else changed?.();
          break;
        }
        case "opened":
          if (message.port === DEVICE_SERVICE_NAME) settle(null);
          break;
        case "receive": {
          if (message.port !== DEVICE_SERVICE_NAME) break;
          const data = Uint8Array.from(message.data);
          for (const l of [...listeners]) l(data);
          break;
        }
        case "openFailed":
          if (message.port === DEVICE_SERVICE_NAME) settle(new Error(message.message));
          break;
        case "error":
          // Said without a port, it is why the plugin has no port to offer.
          if (message.port === null) reason = message.message;
          console.error(`[plugin] ${message.port ?? ""} ${message.message}`);
          break;
        case "instance":
        case "refused":
          // The instance is the editor shell's business ({@link subscribeInstance}).
          break;
      }
    });
    post({ type: "ready" });
  });
}

/**
 * The Grid the plugin window is sized from: the mixer screen's, the larger
 * of its two variants (with and without the Drum Comp+EQ strips), so the
 * window fits either. The mixer screen reads the device spec from core, so
 * call this after the wasm core has started.
 */
export function pluginGrid(): Cells {
  const variants = [mixerScreen(false).layout, mixerScreen(true).layout];
  return { w: Math.max(...variants.map((l) => l.w)), h: Math.max(...variants.map((l) => l.h)) };
}

/** The screen area a window may take, as `window.screen` gives it. */
export interface ScreenArea {
  /** Usable width, logical px; 0 when unknown. */
  availWidth: number;
  /** Usable height, logical px; 0 when unknown. */
  availHeight: number;
}

/**
 * The plugin window for a Grid of `natural` size at `zoom` (ADR 0004):
 * natural × zoom, in whole logical px, no larger than the screen. A window
 * the screen clips scrolls, as the browser does.
 */
export function pluginWindowSize(
  natural: NaturalSize,
  zoom: number,
  screen: ScreenArea,
): { width: number; height: number } {
  const fit = (px: number, avail: number) => {
    const size = Math.round(px * zoom);
    return avail > 0 ? Math.min(size, avail) : size;
  };
  return {
    width: fit(natural.px.width, screen.availWidth),
    height: fit(natural.px.height, screen.availHeight),
  };
}

/**
 * Ask the plugin to size its window for {@link pluginGrid} at `zoom`. Zoom
 * is the only size control; pages fit their Grid to the window they get.
 * Call after the wasm core has started.
 */
export function resizePluginWindow(bridge: PluginBridge, zoom: number, screen: ScreenArea): void {
  const { width, height } = pluginWindowSize(naturalSize(pluginGrid()), zoom, screen);
  bridge.post(encodePageMessage({ type: "resize", width, height }));
}

/**
 * Call `listener` with the instance each time the plugin reports it, which
 * it does on `ready` and whenever any of it changes. Returns its
 * unsubscribe. Subscribe before the page says `ready`
 * ({@link pluginPorts}).
 */
export function subscribeInstance(
  bridge: PluginBridge,
  listener: (instance: InstanceInfo) => void,
): () => void {
  return bridge.subscribe((text) => {
    const message = decodePluginMessage(text);
    if (message?.type === "instance") {
      const { plugin, binding, status, statusText, hostEdits, notices } = message;
      listener({ plugin, binding, status, statusText, hostEdits, notices });
    }
  });
}

/** Call `listener` with why the plugin refused a request about the instance. Returns its unsubscribe. */
export function subscribeRefusals(
  bridge: PluginBridge,
  listener: (message: string) => void,
): () => void {
  return bridge.subscribe((text) => {
    const message = decodePluginMessage(text);
    if (message?.type === "refused") listener(message.message);
  });
}

/** Ask the plugin to bind the instance to `binding`. */
export function bindInstance(bridge: PluginBridge, binding: PluginBinding): void {
  bridge.post(encodePageMessage({ type: "bind", ...binding }));
}
