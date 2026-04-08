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
  tone_name_address,
  tone_name_size,
  setup_studio_set_pc_address,
  setup_studio_set_bs_msb_address,
  build_studio_set_catalog_request,
  parse_catalog_entry,
} from "../pkg/integral_wasm.js";
import type { MidiPortPair } from "./midi";

/** Minimum interval between SysEx sends (ms). */
const THROTTLE_MS = 40;

/** Timeout for RQ1 responses after the message is actually sent (ms). */
const REQUEST_TIMEOUT_MS = 2000;

type Dt1Callback = (address: Uint8Array, data: Uint8Array) => void;

interface QueuedMessage {
  key: string;
  bytes: Uint8Array;
  onSent?: () => void;
}

interface PendingRequest {
  addressKey: string;
  resolve: (data: Uint8Array) => void;
  reject: (err: Error) => void;
  timer?: ReturnType<typeof setTimeout>;
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

  destroy(): void {
    this.port.input.removeEventListener(
      "midimessage",
      this.boundHandler as EventListener,
    );
    this.queue = [];
    this.dt1Listeners.clear();
    for (const req of this.pendingRequests) {
      if (req.timer) clearTimeout(req.timer);
    }
    this.pendingRequests = [];
  }

  // -----------------------------------------------------------------------
  // Send queue with throttle + coalescing
  // -----------------------------------------------------------------------

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
    const key = "dt1:" + addressKey(address);

    // Coalesce: replace any queued DT1 for the same address
    const idx = this.queue.findIndex((m) => m.key === key);
    if (idx >= 0) {
      this.queue[idx] = { key, bytes };
    } else {
      this.queue.push({ key, bytes });
    }

    this.drain();
  }

  private enqueue(key: string, bytes: Uint8Array, onSent?: () => void): void {
    this.queue.push({ key, bytes, onSent });
    this.drain();
  }

  private drain(): void {
    if (this.sending || this.queue.length === 0) return;
    this.sending = true;

    const msg = this.queue.shift()!;
    this.port.output.send(msg.bytes);
    msg.onSent?.();

    setTimeout(() => {
      this.sending = false;
      this.drain();
    }, THROTTLE_MS);
  }

  // -----------------------------------------------------------------------
  // RQ1 request/response
  // -----------------------------------------------------------------------

  requestData(address: number[], size: number[]): Promise<Uint8Array> {
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
      const pending: PendingRequest = { addressKey: key, resolve, reject };
      this.pendingRequests.push(pending);

      // Start timeout only when actually sent
      this.enqueue("rq1:" + key, bytes, () => {
        pending.timer = setTimeout(() => {
          this.pendingRequests = this.pendingRequests.filter(
            (r) => r !== pending,
          );
          reject(new Error(`RQ1 timeout for address ${key}`));
        }, REQUEST_TIMEOUT_MS);
      });
    });
  }

  // -----------------------------------------------------------------------
  // DT1 receive
  // -----------------------------------------------------------------------

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
      return;
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
      if (req.timer) clearTimeout(req.timer);
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

  /** Read the current Studio Set PC (0-63). */
  async requestStudioSetPC(): Promise<number> {
    const data = await this.requestData(
      Array.from(setup_studio_set_pc_address()),
      Array.from(single_byte_size()),
    );
    return data[0]!;
  }

  /**
   * Switch to a different Studio Set by PC number (0-63).
   * Writes BS MSB=85, BS LSB=0, PC=pc to the Setup block.
   * The device needs a moment to load the new set.
   */
  switchStudioSet(pc: number): void {
    this.sendDt1(Array.from(setup_studio_set_bs_msb_address()), [85]);
    // LSB is at MSB+1
    const lsbAddr = Array.from(setup_studio_set_bs_msb_address());
    lsbAddr[3] = lsbAddr[3]! + 1;
    this.sendDt1(lsbAddr, [0]);
    this.sendDt1(Array.from(setup_studio_set_pc_address()), [pc]);
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

  async requestToneName(part: number, bankMsb: number): Promise<string> {
    const addr = tone_name_address(part, bankMsb);
    if (addr.length === 0) return "";
    try {
      const data = await this.requestData(
        Array.from(addr),
        Array.from(tone_name_size()),
      );
      return String.fromCharCode(...data).trimEnd();
    } catch {
      return "";
    }
  }

  // -----------------------------------------------------------------------
  // Catalog queries (undocumented)
  // -----------------------------------------------------------------------

  /**
   * Request all 64 Studio Set names.
   *
   * Sends the undocumented 2-byte catalog query. The device responds with
   * all entries in a stream with delimiter messages interspersed.
   */
  async requestStudioSetNames(): Promise<Map<number, string>> {
    const names = new Map<number, string>();
    const msg = new Uint8Array(
      build_studio_set_catalog_request(this.deviceId),
    );

    return new Promise((resolve) => {
      let timeoutId: ReturnType<typeof setTimeout>;
      // Absolute deadline: 15s max regardless of activity
      const absoluteTimeout = setTimeout(() => {
        done();
      }, 15000);

      const done = () => {
        clearTimeout(timeoutId);
        clearTimeout(absoluteTimeout);
        cleanup();
        resolve(names);
      };

      const resetTimeout = () => {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(done, 3000);
      };

      const handler = (event: MIDIMessageEvent) => {
        const raw = event.data;
        if (!raw || raw.length < 14 || raw[0] !== 0xf0) return;

        let parsed;
        try {
          parsed = parse_dt1(new Uint8Array(raw));
        } catch {
          return;
        }

        const data = new Uint8Array(parsed.data());
        const entry = parse_catalog_entry(data);
        if (entry) {
          names.set(entry.pc, entry.name());
          entry.free();
          if (names.size >= 64) {
            done();
            return;
          }
        }
        // Reset silence timer on any catalog-related response
        resetTimeout();
      };

      const cleanup = () => {
        this.port.input.removeEventListener(
          "midimessage",
          handler as EventListener,
        );
      };

      this.port.input.addEventListener(
        "midimessage",
        handler as EventListener,
      );

      this.enqueue("catalog", msg, () => {
        resetTimeout();
      });
    });
  }

  // -----------------------------------------------------------------------
  // Standard MIDI messages
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

function addressKey(address: number[]): string {
  return address.map((b) => b.toString(16).padStart(2, "0")).join("");
}
