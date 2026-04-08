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

/** A paired MIDI input+output port. */
export interface MidiPortPair {
  id: string;
  name: string;
  input: MIDIInput;
  output: MIDIOutput;
}

/** Request MIDI access with SysEx enabled. */
export async function requestMidiAccess(): Promise<MIDIAccess> {
  if (!navigator.requestMIDIAccess) {
    throw new Error("Web MIDI API is not supported in this browser");
  }
  return navigator.requestMIDIAccess({ sysex: true });
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
      if (output) {
        pairs.push({
          id: input.id,
          name: input.name,
          input,
          output,
        });
      }
    }
  }

  return pairs;
}

/** Find the first port whose name contains "integra" (case-insensitive). */
export function findIntegraPort(pairs: MidiPortPair[]): MidiPortPair | undefined {
  return pairs.find((p) => p.name.toLowerCase().includes("integra"));
}

/**
 * Send an Identity Request to the given port pair and wait for a reply.
 *
 * Uses integral-wasm for message construction and reply parsing.
 * Returns the parsed DeviceIdentity on success, null on timeout.
 */
export function identifyDevice(
  pair: MidiPortPair,
  timeoutMs = 2000,
): Promise<DeviceIdentity | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      pair.input.removeEventListener("midimessage", handler as EventListener);
      resolve(null);
    }, timeoutMs);

    function handler(event: MIDIMessageEvent) {
      const data = event.data;
      if (!data || data.length < 15) return;

      try {
        const identity = parse_identity_reply(new Uint8Array(data.buffer));
        clearTimeout(timer);
        pair.input.removeEventListener("midimessage", handler as EventListener);
        resolve(identity);
      } catch {
        // Not an identity reply — ignore and keep listening
      }
    }

    pair.input.addEventListener("midimessage", handler as EventListener);
    pair.output.send(identity_request());
  });
}
