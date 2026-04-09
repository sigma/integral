/**
 * IntegraService — thin MIDI bridge for the Roland INTEGRA-7.
 *
 * Business logic (state management, send queue, echo suppression) is
 * delegated to the Rust `WasmDeviceState`.  This TS layer handles:
 * - Web MIDI I/O (platform-specific)
 * - RQ1 request/response pairing (async Promises)
 * - Catalog query handlers (event-driven with timeouts)
 * - The drain loop (polls Rust queue via setInterval)
 */

import {
  build_rq1,
  parse_dt1,
  part_eq_address,
  part_eq_size,
  master_eq_address,
  master_eq_size,
  master_eq_switch_address,
  chorus_address,
  chorus_switch_address,
  chorus_core_size,
  reverb_address,
  reverb_switch_address,
  reverb_core_size,
  ext_part_level_address,
  ext_part_mute_address,
  setup_studio_set_pc_address,
  studio_set_name_address,
  studio_set_name_size,
  master_level_address,
  single_byte_size,
  tone_name_address,
  tone_name_size,
  part_mixer_size,
  part_receive_channel_address,
  build_studio_set_catalog_request,
  build_tone_catalog_request,
  parse_catalog_entry,
  decode_nib_params,
  WasmDeviceState,
} from "../pkg/integral_wasm.js";
import type { MidiPortPair } from "./midi";

/** Timeout for RQ1 responses after the message is actually sent (ms). */
const REQUEST_TIMEOUT_MS = 2000;

/** Drain loop interval (ms). */
const DRAIN_INTERVAL_MS = 5;

type Dt1Callback = (address: Uint8Array, data: Uint8Array) => void;

interface PendingRequest {
  addressKey: string;
  resolve: (data: Uint8Array) => void;
  reject: (err: Error) => void;
  timer?: ReturnType<typeof setTimeout>;
}

/** High-level communication service for the INTEGRA-7. */
export class IntegraService {
  private port: MidiPortPair;
  readonly deviceId: number;
  /** Rust-side state machine (queue, echo suppression, state). */
  readonly device: WasmDeviceState;
  private dt1Listeners: Set<Dt1Callback> = new Set();
  private pendingRequests: PendingRequest[] = [];
  private boundHandler: (event: MIDIMessageEvent) => void;
  private drainTimer: ReturnType<typeof setInterval>;

  constructor(port: MidiPortPair, deviceId: number) {
    this.port = port;
    this.deviceId = deviceId;
    this.device = new WasmDeviceState(deviceId);
    this.boundHandler = this.handleMidiMessage.bind(this);
    this.port.input.addEventListener(
      "midimessage",
      this.boundHandler as EventListener,
    );
    // Poll Rust drain queue.
    this.drainTimer = setInterval(() => this.drainRust(), DRAIN_INTERVAL_MS);
  }

  destroy(): void {
    clearInterval(this.drainTimer);
    this.port.input.removeEventListener(
      "midimessage",
      this.boundHandler as EventListener,
    );
    this.dt1Listeners.clear();
    for (const req of this.pendingRequests) {
      if (req.timer) clearTimeout(req.timer);
    }
    this.pendingRequests = [];
  }

  // -----------------------------------------------------------------------
  // Drain loop — polls Rust queue and sends via Web MIDI
  // -----------------------------------------------------------------------

  private drainRust(): void {
    const now = performance.now();
    const msg = this.device.drain(now);
    if (msg) {
      this.port.output.send(msg);
    }
  }

  // -----------------------------------------------------------------------
  // RQ1 request/response (stays in TS — async Promise pattern)
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

      // Use Rust queue for throttled send; start timeout on send.
      this.device.sendRaw("rq1:" + key, bytes);
      // Start timeout immediately (the Rust queue will send it soon).
      pending.timer = setTimeout(() => {
        this.pendingRequests = this.pendingRequests.filter(
          (r) => r !== pending,
        );
        reject(new Error(`RQ1 timeout for address ${key}`));
      }, REQUEST_TIMEOUT_MS);
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

    // Resolve pending RQ1 requests.
    const reqIdx = this.pendingRequests.findIndex(
      (r) => r.addressKey === key,
    );
    if (reqIdx >= 0) {
      const req = this.pendingRequests[reqIdx]!;
      if (req.timer) clearTimeout(req.timer);
      this.pendingRequests.splice(reqIdx, 1);
      req.resolve(data);
    }

    // Feed to Rust state machine (handles echo suppression internally).
    this.device.handleDt1(addr, data, performance.now());

    // Notify TS-side DT1 listeners (for catalog handlers etc.).
    for (const cb of this.dt1Listeners) {
      cb(addr, data);
    }
  }

  // -----------------------------------------------------------------------
  // Convenience: mixer parameters (delegate to Rust DeviceState)
  // -----------------------------------------------------------------------

  setPartLevel(part: number, value: number): void {
    this.device.setPartLevel(part, value);
  }

  setPartPan(part: number, value: number): void {
    this.device.setPartPan(part, value);
  }

  setPartMute(part: number, muted: boolean): void {
    this.device.setPartMute(part, muted);
  }

  setPartReceiveChannel(part: number, channel: number): void {
    this.device.setPartReceiveChannel(part, channel);
  }

  setPartChorusSend(part: number, value: number): void {
    this.device.setPartChorusSend(part, value);
  }

  setPartReverbSend(part: number, value: number): void {
    this.device.setPartReverbSend(part, value);
  }

  setPartTone(part: number, msb: number, lsb: number, pc: number): void {
    this.device.changePartTone(part, msb, lsb, pc);
  }

  setMasterLevel(value: number): void {
    this.device.setMasterLevel(value);
  }

  switchStudioSet(pc: number): void {
    this.device.switchStudioSet(pc);
  }

  // -----------------------------------------------------------------------
  // EQ parameters
  // -----------------------------------------------------------------------

  async requestPartEq(part: number): Promise<Uint8Array> {
    return this.requestData(
      Array.from(part_eq_address(part, 0)),
      Array.from(part_eq_size()),
    );
  }

  setPartEqParam(part: number, paramOffset: number, value: number): void {
    this.device.setPartEqParam(part, paramOffset, value);
  }

  async requestMasterEq(): Promise<Uint8Array> {
    return this.requestData(
      Array.from(master_eq_address(0)),
      Array.from(master_eq_size()),
    );
  }

  setMasterEqParam(paramOffset: number, value: number): void {
    this.device.setMasterEqParam(paramOffset, value);
  }

  async requestMasterEqSwitch(): Promise<boolean> {
    const data = await this.requestData(
      Array.from(master_eq_switch_address()),
      Array.from(single_byte_size()),
    );
    return data[0] === 1;
  }

  setMasterEqSwitch(_enabled: boolean): void {
    this.device.toggleMasterEqSwitch();
  }

  // -----------------------------------------------------------------------
  // Chorus (FX1)
  // -----------------------------------------------------------------------

  async requestChorusCore(): Promise<Uint8Array> {
    return this.requestData(
      Array.from(chorus_address(0)),
      Array.from(chorus_core_size()),
    );
  }

  async requestChorusSwitch(): Promise<boolean> {
    const data = await this.requestData(
      Array.from(chorus_switch_address()),
      Array.from(single_byte_size()),
    );
    return data[0] === 1;
  }

  setChorusSwitch(_enabled: boolean): void {
    this.device.toggleChorusSwitch();
  }

  setChorusParam(offset: number, value: number): void {
    this.device.setChorusParam(offset, value);
  }

  setChorusNibParam(paramIndex: number, value: number): void {
    this.device.setChorusNibParam(paramIndex, value);
  }

  async requestChorusParams(): Promise<number[]> {
    const data = await this.requestData(
      Array.from(chorus_address(0x04)),
      [0x00, 0x00, 0x00, 0x50],
    );
    return Array.from(decode_nib_params(data, 20));
  }

  // -----------------------------------------------------------------------
  // Reverb (FX2)
  // -----------------------------------------------------------------------

  async requestReverbCore(): Promise<Uint8Array> {
    return this.requestData(
      Array.from(reverb_address(0)),
      Array.from(reverb_core_size()),
    );
  }

  async requestReverbSwitch(): Promise<boolean> {
    const data = await this.requestData(
      Array.from(reverb_switch_address()),
      Array.from(single_byte_size()),
    );
    return data[0] === 1;
  }

  setReverbSwitch(_enabled: boolean): void {
    this.device.toggleReverbSwitch();
  }

  setReverbParam(offset: number, value: number): void {
    this.device.setReverbParam(offset, value);
  }

  setReverbNibParam(paramIndex: number, value: number): void {
    this.device.setReverbNibParam(paramIndex, value);
  }

  async requestReverbParams(): Promise<number[]> {
    const data = await this.requestData(
      Array.from(reverb_address(0x03)),
      [0x00, 0x00, 0x00, 0x60],
    );
    return Array.from(decode_nib_params(data, 24));
  }

  // -----------------------------------------------------------------------
  // Ext Part
  // -----------------------------------------------------------------------

  async requestExtPartLevel(): Promise<number> {
    const data = await this.requestData(
      Array.from(ext_part_level_address()),
      Array.from(single_byte_size()),
    );
    return data[0]!;
  }

  async requestExtPartMute(): Promise<boolean> {
    const data = await this.requestData(
      Array.from(ext_part_mute_address()),
      Array.from(single_byte_size()),
    );
    return data[0] === 1;
  }

  setExtPartLevel(value: number): void {
    this.device.setExtLevel(value);
  }

  setExtPartMute(_muted: boolean): void {
    this.device.toggleExtMute();
  }

  // -----------------------------------------------------------------------
  // Drum Comp+EQ
  // -----------------------------------------------------------------------

  /** Read Drum Comp+EQ common params: switch (0x43), part (0x44), 6 output assigns (0x45-0x4A). */
  async requestDrumCompEqCommon(): Promise<{ enabled: boolean; part: number; outputAssigns: number[] }> {
    // Read 8 bytes from 0x43 to 0x4A inclusive.
    const data = await this.requestData(
      [0x18, 0x00, 0x00, 0x43],
      [0x00, 0x00, 0x00, 0x08],
    );
    return {
      enabled: data[0] === 1,
      part: data[1] ?? 9,
      outputAssigns: [data[2] ?? 0, data[3] ?? 0, data[4] ?? 0, data[5] ?? 0, data[6] ?? 0, data[7] ?? 0],
    };
  }

  /** Read the full 84-byte Comp+EQ block for the given part index. */
  async requestCompEqBlock(partIndex: number): Promise<Uint8Array> {
    // Address: temporary_tone_base(partIndex) + 00 08 00 00
    const total = partIndex * 0x20;
    const byte0 = 0x19 + Math.floor(total / 128);
    const byte1 = total % 128;
    return this.requestData(
      [byte0, byte1, 0x08, 0x00],
      [0x00, 0x00, 0x00, 0x54],
    );
  }

  // -----------------------------------------------------------------------
  // Request helpers (read from device)
  // -----------------------------------------------------------------------

  async requestStudioSetPC(): Promise<number> {
    const data = await this.requestData(
      Array.from(setup_studio_set_pc_address()),
      Array.from(single_byte_size()),
    );
    return data[0]!;
  }

  async requestStudioSetName(): Promise<string> {
    const data = await this.requestData(
      Array.from(studio_set_name_address()),
      Array.from(studio_set_name_size()),
    );
    return String.fromCharCode(...data).trimEnd();
  }

  async requestPartMixerState(part: number): Promise<Uint8Array> {
    return this.requestData(
      Array.from(part_receive_channel_address(part)),
      Array.from(part_mixer_size()),
    );
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
  // Catalog queries (stay in TS — event-driven with timeouts)
  // -----------------------------------------------------------------------

  async requestStudioSetNames(): Promise<Map<number, string>> {
    const names = new Map<number, string>();
    const msg = new Uint8Array(
      build_studio_set_catalog_request(this.deviceId, 0, 64),
    );

    return new Promise((resolve) => {
      let timeoutId: ReturnType<typeof setTimeout>;
      const absoluteTimeout = setTimeout(() => done(), 15000);

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

      this.device.sendRaw("catalog", msg);
      resetTimeout();
    });
  }

  requestToneCatalogPage(
    msb: number,
    lsb: number,
    start: number,
    count: number,
  ): Promise<{ msb: number; lsb: number; pc: number; name: string }[]> {
    const entries: { msb: number; lsb: number; pc: number; name: string }[] =
      [];
    const msg = new Uint8Array(
      build_tone_catalog_request(this.deviceId, msb, lsb, start, count),
    );

    return new Promise((resolve) => {
      let timeoutId: ReturnType<typeof setTimeout>;
      const absoluteTimeout = setTimeout(() => done(), 5000);

      const done = () => {
        clearTimeout(timeoutId);
        clearTimeout(absoluteTimeout);
        cleanup();
        resolve(entries);
      };

      const resetTimeout = () => {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(done, 500);
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
          entries.push({
            msb: entry.bank_msb,
            lsb: entry.bank_lsb,
            pc: entry.pc,
            name: entry.name(),
          });
          entry.free();
          if (entries.length >= count) {
            done();
            return;
          }
        }
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

      this.device.sendRaw(`tone-catalog:${msb}:${lsb}:${start}`, msg);
      resetTimeout();
    });
  }

  async requestToneCatalogForLsb(
    msb: number,
    lsb: number,
  ): Promise<{ msb: number; lsb: number; pc: number; name: string }[]> {
    const page1 = await this.requestToneCatalogPage(msb, lsb, 0, 64);
    const page2 = await this.requestToneCatalogPage(msb, lsb, 64, 64);
    return [...page1, ...page2];
  }

  // -----------------------------------------------------------------------
  // Standard MIDI messages (bypass queue — no throttle needed)
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
