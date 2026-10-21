/**
 * The plugin transport (ADR 0004, #365): how the page reaches the device
 * when it runs in the plugin's webview.
 *
 * WKWebView has no Web MIDI, and the host does not pass plugin SysEx
 * through, so the plugin opens the device port itself and relays bytes over
 * the webview's IPC. This module is the page's half: a {@link MidiPorts}
 * over a {@link PluginBridge}. It does not pretend to be Web MIDI.
 *
 * The page paces its own sends (`WasmDeviceState.drain`), as on the web; the
 * plugin forwards bytes as they come and does not throttle a second time.
 *
 * Wire format: one JSON object per message, tagged by `type`. The Rust half
 * is `crates/integral-vst/src/webview/ipc.rs`; the two are kept in step by
 * the tests on each side.
 */

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
  | { type: "resize"; width: number; height: number };

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

/**
 * The plugin's ports, once it has listed them. Pairs are named by the
 * plugin and opened through it; one pair is open at a time.
 *
 * A send with a future `timestamp` (the Sequencer's) is held in the page
 * until then, as Web MIDI would hold it in the browser.
 */
export function pluginPorts(bridge: PluginBridge): Promise<MidiPorts> {
  const post = (message: PageMessage) => bridge.post(encodePageMessage(message));
  const listeners = new Map<string, Set<MidiListener>>();
  const openings = new Map<string, Opening[]>();
  const byName = new Map<string, MidiPortPair>();
  let names: string[] | null = null;
  let changed: (() => void) | null = null;

  function settle(port: string, error: Error | null): void {
    const waiting = openings.get(port) ?? [];
    openings.delete(port);
    for (const w of waiting) {
      if (error) w.reject(error);
      else w.resolve();
    }
  }

  function pair(name: string): MidiPortPair {
    const existing = byName.get(name);
    if (existing) return existing;
    const created: MidiPortPair = {
      id: name,
      name,
      open: () =>
        new Promise<void>((resolve, reject) => {
          const timer = setTimeout(() => {
            openings.set(name, (openings.get(name) ?? []).filter((w) => w !== waiting));
            reject(new Error(`no answer opening ${name}`));
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
          openings.set(name, [...(openings.get(name) ?? []), waiting]);
          post({ type: "open", port: name });
        }),
      send(data, timestamp) {
        const message: PageMessage = { type: "send", port: name, data: Array.from(data) };
        const delay = timestamp === undefined ? 0 : timestamp - performance.now();
        if (delay > 0) setTimeout(() => post(message), delay);
        else post(message);
      },
      listen(listener) {
        const set = listeners.get(name) ?? new Set<MidiListener>();
        listeners.set(name, set);
        set.add(listener);
        return () => set.delete(listener);
      },
    };
    byName.set(name, created);
    return created;
  }

  const ports: MidiPorts = {
    pairs: () => (names ?? []).map(pair),
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
          const first = names === null;
          names = message.ports;
          if (first) resolve(ports);
          else changed?.();
          break;
        }
        case "opened":
          settle(message.port, null);
          break;
        case "receive": {
          const data = Uint8Array.from(message.data);
          for (const l of [...(listeners.get(message.port) ?? [])]) l(data);
          break;
        }
        case "openFailed":
          settle(message.port, new Error(message.message));
          break;
        case "error":
          console.error(`[plugin] ${message.port ?? ""} ${message.message}`);
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
