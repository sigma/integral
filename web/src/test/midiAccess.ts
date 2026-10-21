/** A fake Web MIDI `MIDIAccess` for connection tests. */

import { identity_request } from "../../pkg/integral_wasm.js";

/** An Identity Reply from an INTEGRA-7 at device ID 0x10: test data, as the
 *  device sends it. */
const INTEGRA7_IDENTITY_REPLY = new Uint8Array([
  0xf0, 0x7e, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x00, 0xf7,
]);

/** A fake device port. */
export interface FakePortSpec {
  /** The name both halves of the pair carry. */
  name: string;
  /** Answers an Identity Request as an INTEGRA-7 would. */
  answers?: boolean;
}

/** A fake `MIDIAccess`, and what the app sent through it. */
export interface FakeMidiAccess {
  /** The access to hand the app. */
  access: MIDIAccess;
  /** The names of the ports an Identity Request went to, in order. */
  identified: string[];
  /** Every other message sent, with the name of the port it went to. */
  sent: { port: string; bytes: number[] }[];
  /** Plug in a port, firing `onstatechange` as the browser does. */
  plug(spec: FakePortSpec): void;
}

/** A `MIDIAccess` exposing one paired input+output per spec. Every message
 *  but an Identity Request is recorded and left unanswered. */
export function fakeMidiAccess(specs: FakePortSpec[]): FakeMidiAccess {
  const identified: string[] = [];
  const sent: { port: string; bytes: number[] }[] = [];
  const inputs = new Map<string, MIDIInput>();
  const outputs = new Map<string, MIDIOutput>();
  const access = { inputs, outputs, onstatechange: null } as unknown as MIDIAccess;

  function add(spec: FakePortSpec): void {
    const n = inputs.size;
    const input = Object.assign(new EventTarget(), {
      id: `in-${n}`,
      name: spec.name,
      open: async () => input,
    });
    const output = {
      id: `out-${n}`,
      name: spec.name,
      open: async () => output,
      send: (bytes: number[] | Uint8Array) => {
        const request = identity_request();
        const isIdentityRequest =
          bytes.length === request.length &&
          request.every((b, j) => bytes[j] === b);
        if (!isIdentityRequest) {
          sent.push({ port: spec.name, bytes: [...bytes] });
          return;
        }
        identified.push(spec.name);
        if (!spec.answers) return;
        queueMicrotask(() =>
          input.dispatchEvent(
            Object.assign(new Event("midimessage"), {
              data: INTEGRA7_IDENTITY_REPLY,
            }),
          ),
        );
      },
    };
    inputs.set(input.id, input as unknown as MIDIInput);
    outputs.set(output.id, output as unknown as MIDIOutput);
  }

  specs.forEach(add);
  return {
    access,
    identified,
    sent,
    plug(spec) {
      add(spec);
      access.onstatechange?.(new Event("statechange") as MIDIConnectionEvent);
    },
  };
}
