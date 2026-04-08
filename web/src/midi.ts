/**
 * Web MIDI API utilities for communicating with the Roland INTEGRA-7.
 *
 * Uses SysEx Identity Request/Reply to verify device identity.
 */

/** Result of a successful identity check. */
export interface Integra7Identity {
  deviceId: number;
  familyCode: string;
  familyNumber: string;
  revision: string;
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

// Identity Request: F0 7E 7F 06 01 F7
const IDENTITY_REQUEST = new Uint8Array([0xf0, 0x7e, 0x7f, 0x06, 0x01, 0xf7]);

// Expected: manufacturer=41H (Roland), family=64H 02H (INTEGRA-7)
const ROLAND_ID = 0x41;
const INTEGRA7_FAMILY_0 = 0x64;
const INTEGRA7_FAMILY_1 = 0x02;

/**
 * Send an Identity Request to the given port pair and wait for a reply.
 * Returns the parsed identity on success, null on timeout.
 */
export function identifyDevice(
  pair: MidiPortPair,
  timeoutMs = 2000,
): Promise<Integra7Identity | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      pair.input.removeEventListener("midimessage", handler as EventListener);
      resolve(null);
    }, timeoutMs);

    function handler(event: MIDIMessageEvent) {
      const data = event.data;
      if (!data || data.length < 15) return;

      // Identity Reply: F0 7E dev 06 02 41 64 02 nn nn xx xx xx xx F7
      if (
        data[0] === 0xf0 &&
        data[1] === 0x7e &&
        data[3] === 0x06 &&
        data[4] === 0x02 && // Identity Reply
        data[5] === ROLAND_ID &&
        data[6] === INTEGRA7_FAMILY_0 &&
        data[7] === INTEGRA7_FAMILY_1
      ) {
        clearTimeout(timer);
        pair.input.removeEventListener("midimessage", handler as EventListener);

        const hex = (b: number) => b.toString(16).toUpperCase().padStart(2, "0");
        resolve({
          deviceId: data[2]!,
          familyCode: `${hex(data[6]!)} ${hex(data[7]!)}`,
          familyNumber: `${hex(data[8]!)} ${hex(data[9]!)}`,
          revision: [data[10], data[11], data[12], data[13]]
            .map((b) => hex(b!))
            .join(" "),
        });
      }
    }

    pair.input.addEventListener("midimessage", handler as EventListener);
    pair.output.send(IDENTITY_REQUEST);
  });
}
