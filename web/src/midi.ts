/**
 * Web MIDI API utilities for communicating with the Roland INTEGRA-7.
 *
 * SysEx message construction and parsing is delegated to the integral-wasm
 * module (Rust core compiled to WebAssembly).
 */

import init, {
  identity_request,
  parse_identity_reply,
  type DeviceIdentity,
} from "../pkg/integral_wasm.js";

export type { DeviceIdentity };

let wasmReady: Promise<void> | null = null;

/** Ensure the WASM module is initialized. Call before using any WASM functions. */
export function initWasm(): Promise<void> {
  if (!wasmReady) {
    wasmReady = init().then(() => {});
  }
  return wasmReady;
}

/** Called with each message a port receives. */
export type MidiListener = (data: Uint8Array) => void;

/**
 * One device link: a paired input and output, named as the user picks it.
 *
 * This is the port interface the app talks through. Web MIDI implements it
 * in the browser ({@link webMidiPorts}); the plugin transport implements it
 * in the plugin's webview (`plugin.ts`), with no Web MIDI underneath.
 */
export interface MidiPortPair {
  /** Stable for the life of the port list it came from. */
  id: string;
  /** What the user sees and the last-port memory keeps. */
  name: string;
  /** Open both halves. Idempotent; resolves once messages flow. */
  open(): Promise<void>;
  /** Send one message at `timestamp` (`performance.now()` clock), or now. */
  send(data: Uint8Array | number[], timestamp?: number): void;
  /** Hand every received message to `listener`. Returns its unsubscribe. */
  listen(listener: MidiListener): () => void;
}

/** The port pairs a transport offers, and word of when they change. */
export interface MidiPorts {
  /** The pairs present now. */
  pairs(): MidiPortPair[];
  /**
   * Call `listener` whenever pairs come or go. One listener at a time;
   * returns its unsubscribe.
   */
  onChange(listener: () => void): () => void;
  /** Stop listening to the transport, where it holds anything open. */
  close?(): void;
}

/** Request MIDI access with SysEx enabled. */
export async function requestMidiAccess(): Promise<MIDIAccess> {
  if (!navigator.requestMIDIAccess) {
    throw new Error("Web MIDI API is not supported in this browser");
  }
  return navigator.requestMIDIAccess({ sysex: true });
}

/** A Web MIDI input and output as one {@link MidiPortPair}. */
export function webMidiPair(input: MIDIInput, output: MIDIOutput): MidiPortPair {
  return {
    id: input.id,
    name: input.name ?? "",
    // The Web MIDI API only opens a port implicitly when the `onmidimessage`
    // attribute is set; `addEventListener` does not. `listen` registers with
    // `addEventListener`, so inputs must be opened explicitly or they
    // silently deliver nothing. `open()` is idempotent.
    async open() {
      await Promise.all([input.open(), output.open()]);
    },
    send(data, timestamp) {
      output.send(data, timestamp);
    },
    listen(listener) {
      const handler = (event: Event) => {
        const data = (event as MIDIMessageEvent).data;
        if (data) listener(data);
      };
      input.addEventListener("midimessage", handler);
      return () => input.removeEventListener("midimessage", handler);
    },
  };
}

/**
 * Find paired input+output ports (matched by name).
 * Only returns ports that have both an input and output with the same name.
 */
export function getPairedPorts(access: MIDIAccess): MidiPortPair[] {
  const outputs = new Map<string, MIDIOutput>();
  for (const output of access.outputs.values()) {
    if (output.name) {
      outputs.set(output.name, output);
    }
  }

  const pairs: MidiPortPair[] = [];
  for (const input of access.inputs.values()) {
    if (input.name) {
      const output = outputs.get(input.name);
      if (output) pairs.push(webMidiPair(input, output));
    }
  }

  return pairs;
}

/** The paired ports of a Web MIDI `access`. */
export function webMidiPorts(access: MIDIAccess): MidiPorts {
  return {
    pairs: () => getPairedPorts(access),
    onChange(listener) {
      access.onstatechange = () => listener();
      return () => {
        access.onstatechange = null;
      };
    },
  };
}

/** Web MIDI's paired ports; rejects where the browser has no Web MIDI. */
export async function webMidi(): Promise<MidiPorts> {
  return webMidiPorts(await requestMidiAccess());
}

/** Find the first port whose name contains "integra" (case-insensitive). */
export function findIntegraPort(pairs: MidiPortPair[]): MidiPortPair | undefined {
  return pairs.find((p) => p.name.toLowerCase().includes("integra"));
}

/** Find the pair named exactly `name`, if there is one. */
export function findPortByName(
  pairs: MidiPortPair[],
  name: string | null,
): MidiPortPair | undefined {
  return name === null ? undefined : pairs.find((p) => p.name === name);
}

/** The storage key holding the name of the last port connected to. */
export const LAST_PORT_KEY = "integral.lastPort";

/**
 * The last port connected to, kept across page loads.
 *
 * Hosts destroy a plugin editor on close, so its page reloads on reopen; the
 * browser tab reloads too. Ports are remembered by name: Web MIDI port ids
 * are not guaranteed stable across sessions.
 */
export interface PortMemory {
  /** The remembered port's name, or `null` if none is remembered. */
  recall(): string | null;
  /** Remember `name` as the last port connected to. */
  remember(name: string): void;
}

/**
 * A `PortMemory` over `storage`. With no storage, or one that throws (a
 * sandboxed frame, a full quota), nothing is remembered.
 */
export function portMemory(storage: Storage | null): PortMemory {
  return {
    recall() {
      try {
        return storage?.getItem(LAST_PORT_KEY) ?? null;
      } catch {
        return null;
      }
    },
    remember(name) {
      try {
        storage?.setItem(LAST_PORT_KEY, name);
      } catch {
        // Not remembering only costs a click on the next open.
      }
    },
  };
}

/** The browser's `localStorage`, or `null` where reading it throws. */
export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** How long `identifyDevice` waits for an Identity Reply by default. */
export const IDENTIFY_TIMEOUT_MS = 2000;

/**
 * Send an Identity Request to the given port pair and wait for a reply.
 *
 * Uses integral-wasm for message construction and reply parsing.
 * Returns the parsed DeviceIdentity on success, null on timeout or when the
 * pair fails to open.
 */
export async function identifyDevice(
  pair: MidiPortPair,
  timeoutMs = IDENTIFY_TIMEOUT_MS,
): Promise<DeviceIdentity | null> {
  try {
    await pair.open();
  } catch {
    // A pair that won't open has no device answering on it.
    return null;
  }
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      unlisten();
      resolve(null);
    }, timeoutMs);

    const unlisten = pair.listen((data) => {
      if (data.length < 15) return;

      try {
        const identity = parse_identity_reply(data);
        clearTimeout(timer);
        unlisten();
        resolve(identity);
      } catch {
        // Not an identity reply — ignore and keep listening
      }
    });

    pair.send(identity_request());
  });
}
