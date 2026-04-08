/**
 * IntegraService — high-level SysEx communication with the Roland INTEGRA-7.
 *
 * Handles DT1 send/receive with 20ms throttling, address coalescing,
 * RQ1 request/response, and standard MIDI messages (Note On/Off).
 *
 * All SysEx construction and parsing is delegated to integral-wasm.
 */

import {
  build_dt1,
  build_rq1,
  parse_dt1,
  part_level_address,
  part_pan_address,
  part_mute_address,
  part_receive_channel_address,
  part_mixer_size,
  master_level_address,
  studio_set_name_address,
  studio_set_name_size,
  single_byte_size,
} from "../pkg/integral_wasm.js";
import type { MidiPortPair } from "./midi";

/** Minimum interval between SysEx sends (ms). */
const THROTTLE_MS = 20;

/** Timeout for RQ1 responses (ms). */
const REQUEST_TIMEOUT_MS = 500;

type Dt1Callback = (address: Uint8Array, data: Uint8Array) => void;

interface QueuedMessage {
  address: string; // 4-byte key for coalescing
  bytes: Uint8Array;
}

interface PendingRequest {
  addressKey: string;
  resolve: (data: Uint8Array) => void;
  timer: ReturnType<typeof setTimeout>;
}

/** High-level communication service for the INTEGRA-7. */
export class IntegraService {
  private port: MidiPortPair;
  private deviceId: number;
  private queue: QueuedMessage[] = [];
  private sending = false;
  private dt1Listeners: Set<Dt1Callback> = new Set();
  private pendingRequests: PendingRequest[] = [];
  private boundHandler: (event: MIDIMessageEvent) => void;

  constructor(port: MidiPortPair, deviceId: number) {
    this.port = port;
    this.deviceId = deviceId;
    this.boundHandler = this.handleMidiMessage.bind(this);
    this.port.input.addEventListener(
      "midimessage",
      this.boundHandler as EventListener,
    );
  }

  /** Clean up listeners. */
  destroy(): void {
    this.port.input.removeEventListener(
      "midimessage",
      this.boundHandler as EventListener,
    );
    this.queue = [];
    this.dt1Listeners.clear();
    for (const req of this.pendingRequests) {
      clearTimeout(req.timer);
    }
    this.pendingRequests = [];
  }

  // -----------------------------------------------------------------------
  // DT1 send (with throttle + coalescing)
  // -----------------------------------------------------------------------

  /** Send a DT1 message. Coalesces by address and throttles to 20ms. */
  sendDt1(address: number[], data: number[]): void {
    const bytes = new Uint8Array(
      build_dt1(
        this.deviceId,
        address[0]!,
        address[1]!,
        address[2]!,
        address[3]!,
        new Uint8Array(data),
      ),
    );
    const key = addressKey(address);

    // Coalesce: replace any queued message for the same address
    const idx = this.queue.findIndex((m) => m.address === key);
    if (idx >= 0) {
      this.queue[idx] = { address: key, bytes };
    } else {
      this.queue.push({ address: key, bytes });
    }

    this.drain();
  }

  private drain(): void {
    if (this.sending || this.queue.length === 0) return;
    this.sending = true;

    const msg = this.queue.shift()!;
    this.port.output.send(msg.bytes);

    setTimeout(() => {
      this.sending = false;
      this.drain();
    }, THROTTLE_MS);
  }

  // -----------------------------------------------------------------------
  // RQ1 request/response
  // -----------------------------------------------------------------------

  /** Send an RQ1 and wait for the matching DT1 response. */
  requestData(
    address: number[],
    size: number[],
  ): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const bytes = new Uint8Array(
        build_rq1(
          this.deviceId,
          address[0]!,
          address[1]!,
          address[2]!,
          address[3]!,
          size[0]!,
          size[1]!,
          size[2]!,
          size[3]!,
        ),
      );

      const key = addressKey(address);

      const timer = setTimeout(() => {
        this.pendingRequests = this.pendingRequests.filter(
          (r) => r.addressKey !== key,
        );
        reject(new Error(`RQ1 timeout for address ${key}`));
      }, REQUEST_TIMEOUT_MS);

      this.pendingRequests.push({ addressKey: key, resolve, timer });

      // RQ1 goes through the throttle queue too
      this.queue.push({ address: key + ":rq1", bytes });
      this.drain();
    });
  }

  // -----------------------------------------------------------------------
  // DT1 receive
  // -----------------------------------------------------------------------

  /** Register a callback for incoming DT1 messages. Returns unsubscribe fn. */
  onDt1(callback: Dt1Callback): () => void {
    this.dt1Listeners.add(callback);
    return () => this.dt1Listeners.delete(callback);
  }

  private handleMidiMessage(event: MIDIMessageEvent): void {
    const raw = event.data;
    if (!raw || raw.length < 14) return;
    if (raw[0] !== 0xf0) return;

    let parsed;
    try {
      parsed = parse_dt1(new Uint8Array(raw));
    } catch {
      return; // not a DT1 we understand
    }

    const addr = new Uint8Array(parsed.address());
    const data = new Uint8Array(parsed.data());
    const key = addressKey(Array.from(addr));

    // Resolve pending RQ1 requests
    const reqIdx = this.pendingRequests.findIndex(
      (r) => r.addressKey === key,
    );
    if (reqIdx >= 0) {
      const req = this.pendingRequests[reqIdx]!;
      clearTimeout(req.timer);
      this.pendingRequests.splice(reqIdx, 1);
      req.resolve(data);
    }

    // Notify DT1 listeners
    for (const cb of this.dt1Listeners) {
      cb(addr, data);
    }
  }

  // -----------------------------------------------------------------------
  // Convenience: mixer parameters
  // -----------------------------------------------------------------------

  setPartLevel(part: number, value: number): void {
    this.sendDt1(Array.from(part_level_address(part)), [value]);
  }

  setPartPan(part: number, value: number): void {
    this.sendDt1(Array.from(part_pan_address(part)), [value]);
  }

  setPartMute(part: number, muted: boolean): void {
    this.sendDt1(Array.from(part_mute_address(part)), [muted ? 1 : 0]);
  }

  setMasterLevel(value: number): void {
    this.sendDt1(Array.from(master_level_address()), [value]);
  }

  async requestStudioSetName(): Promise<string> {
    const data = await this.requestData(
      Array.from(studio_set_name_address()),
      Array.from(studio_set_name_size()),
    );
    return String.fromCharCode(...data).trimEnd();
  }

  async requestPartMixerState(part: number): Promise<Uint8Array> {
    const addr = part_receive_channel_address(part);
    const size = part_mixer_size();
    return this.requestData(Array.from(addr), Array.from(size));
  }

  async requestMasterLevel(): Promise<number> {
    const data = await this.requestData(
      Array.from(master_level_address()),
      Array.from(single_byte_size()),
    );
    return data[0]!;
  }

  // -----------------------------------------------------------------------
  // Standard MIDI messages (not SysEx)
  // -----------------------------------------------------------------------

  sendNoteOn(channel: number, note: number, velocity: number): void {
    this.port.output.send(
      new Uint8Array([0x90 | (channel & 0x0f), note & 0x7f, velocity & 0x7f]),
    );
  }

  sendNoteOff(channel: number, note: number): void {
    this.port.output.send(
      new Uint8Array([0x80 | (channel & 0x0f), note & 0x7f, 0]),
    );
  }
}

/** Convert a 4-byte address to a string key for map lookups. */
function addressKey(address: number[]): string {
  return address.map((b) => b.toString(16).padStart(2, "0")).join("");
}
