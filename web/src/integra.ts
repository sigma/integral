/**
 * IntegraService — thin MIDI bridge for the Roland INTEGRA-7.
 *
 * Business logic (state management, send queue, echo suppression) is
 * delegated to the Rust `WasmDeviceState`.  This TS layer handles:
 * - Web MIDI I/O (platform-specific)
 * - RQ1 request/response pairing (async Promises)
 * - Catalog query handlers (event-driven with timeouts)
 * - The drain loop (polls Rust queue via setInterval)
 * - Timers driving the Rust Expansion load machine
 * - I/O and timers driving the Rust Librarian (Pack export/import) machines
 */

import {
  build_rq1,
  parse_dt1,
  build_studio_set_catalog_request,
  build_tone_catalog_request,
  parse_catalog_entry,
  build_tone_write_request,
  build_studio_set_write_request,
  parse_write_reply,
  write_timeout_ms,
  expansionSlotsAddress,
  LoadActionKind,
  LoadOutcomeKind,
  WasmDeviceState,
  type WriteReply,
  LibrarianActionKind,
  Pack,
  PackExport,
  PackImport,
  type ExportProgress,
  type ExportSource,
  type ImportPlan,
  type ImportStatus,
  type LibrarianAction,
} from "../pkg/integral_wasm.js";
import { studioSetType } from "./librarian";
import type { MidiPortPair } from "./midi";

// ---------------------------------------------------------------------------
// MIDI protocol constants
// ---------------------------------------------------------------------------

/** SysEx start byte (0xF0). */
const SYSEX_START = 0xf0;

/** Note On status nibble (0x90). */
const NOTE_ON = 0x90;

/** Note Off status nibble (0x80). */
const NOTE_OFF = 0x80;

/** Mask to extract MIDI channel from a status byte. */
const CHANNEL_MASK = 0x0f;

/** Mask for 7-bit MIDI data values. */
const DATA_MASK = 0x7f;

/** Timeout for RQ1 responses after the message is actually sent (ms). */
const REQUEST_TIMEOUT_MS = 2000;

/** Maximum number of automatic retries for timed-out RQ1 requests. */
const MAX_RETRIES = 2;

/**
 * How long a sent slot read waits for its answer before the Expansion slots
 * become unknown (ms). An idle device answers within ~100 ms; one busy
 * loading answers nothing until done, which unknown state reflects.
 */
const SLOT_READ_TIMEOUT_MS = 2000;

/** Drain loop interval (ms). */
const DRAIN_INTERVAL_MS = 5;

type Dt1Callback = (address: Uint8Array, data: Uint8Array) => void;

/** How an Expansion load ended. */
export interface ExpansionLoadResult {
  outcome: LoadOutcomeKind;
  /** Slot values A–D read back (`Loaded`, `Failed`), else null. */
  slots: Uint8Array | null;
}

/** How a Pack import ended. */
export interface PackImportResult {
  /** Each entry's final status, indexed like the Pack's entries. */
  statuses: ImportStatus[];
  /** Why the import stopped early, if it did. */
  error: string | undefined;
}

/** Where a running Pack export stands. */
export interface PackExportProgress {
  /** Every source with its status, in Pack order. */
  progress: ExportProgress[];
  /** The User tones pulled in by Studio Set references, once known. */
  autoIncluded: ExportSource[];
}

/**
 * Call `report` at most once per animation frame after `schedule`, so
 * progress isn't rebuilt for every incoming packet.
 */
function frameThrottle(report: () => void): {
  schedule: () => void;
  cancel: () => void;
} {
  let frame: number | undefined;
  return {
    schedule: () => {
      frame ??= requestAnimationFrame(() => {
        frame = undefined;
        report();
      });
    },
    cancel: () => {
      if (frame !== undefined) cancelAnimationFrame(frame);
      frame = undefined;
    },
  };
}

/** A Librarian machine (`PackExport` or `PackImport`), as the driver sees it. */
interface LibrarianMachine {
  nextAction(): LibrarianAction;
  onRead(addr: Uint8Array, data: Uint8Array): boolean;
  onWriteReply?(addr: Uint8Array, data: Uint8Array): boolean;
  onTimeout(): void;
  cancel(): void;
}

interface PendingRequest {
  addressKey: string;
  resolve: (data: Uint8Array) => void;
  reject: (err: Error) => void;
  timer?: ReturnType<typeof setTimeout>;
  /** Drops the start of the timeout if the request is still queued. */
  cancelSent?: () => void;
}

/** A queued request and what to do once it leaves the queue. */
interface SentHook {
  bytes: Uint8Array;
  onSent: () => void;
}

/** High-level communication service for the INTEGRA-7. */
export class IntegraService {
  private port: MidiPortPair;
  readonly deviceId: number;
  /** Rust-side state machine (queue, echo suppression, state). */
  readonly device: WasmDeviceState;
  private dt1Listeners: Set<Dt1Callback> = new Set();
  /** Whether a user-memory write is awaiting its "finished" reply. */
  private writing = false;
  /** The slot read awaiting its answer, shared by concurrent callers. */
  private slotRead: Promise<Uint8Array | null> | null = null;
  /**
   * Queued requests and what to do once each leaves the queue: reply
   * timeouts start then, so a held queue (an Expansion load) or a busy one
   * (the initial mixer load) doesn't time them out.
   */
  private sentHooks: SentHook[] = [];
  private pendingRequests: PendingRequest[] = [];
  private unlisten: () => void;
  private drainTimer: ReturnType<typeof setInterval>;

  constructor(port: MidiPortPair, deviceId: number) {
    this.port = port;
    this.deviceId = deviceId;
    this.device = new WasmDeviceState(deviceId);
    // Identification opens the pair first, but do not depend on it. A pair
    // that fails to open now already answered identification, so a failure
    // here only means sends go nowhere, as on an unplugged port.
    this.port.open().catch(() => {});
    this.unlisten = this.port.listen((data) => this.handleMidiMessage(data));
    // Poll Rust drain queue.
    this.drainTimer = setInterval(() => this.drainRust(), DRAIN_INTERVAL_MS);
  }

  destroy(): void {
    clearInterval(this.drainTimer);
    this.unlisten();
    this.dt1Listeners.clear();
    this.sentHooks = [];
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
    if (!msg) return;
    this.sendSysex(msg);
    const i = this.sentHooks.findIndex((h) => sameBytes(msg, h.bytes));
    if (i >= 0) this.sentHooks.splice(i, 1)[0]!.onSent();
  }

  /**
   * Queue a raw SysEx request and call `onSent` once it leaves the queue.
   * Returns a function that drops `onSent` if it hasn't run yet.
   */
  private queueRaw(
    key: string,
    bytes: Uint8Array,
    onSent: () => void,
  ): () => void {
    const hook: SentHook = { bytes, onSent };
    this.sentHooks.push(hook);
    this.device.sendRaw(key, bytes);
    return () => {
      this.sentHooks = this.sentHooks.filter((h) => h !== hook);
    };
  }

  private sendSysex(msg: Uint8Array): void {
    console.log(
      `[midi-out] ${Array.from(msg)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join(" ")}`,
    );
    this.port.send(msg);
  }

  // -----------------------------------------------------------------------
  // RQ1 request/response (stays in TS — async Promise pattern)
  // -----------------------------------------------------------------------

  async requestData(address: number[], size: number[]): Promise<Uint8Array> {
    for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
      try {
        return await this.requestDataOnce(address, size);
      } catch (e) {
        if (attempt < MAX_RETRIES) {
          console.warn(
            `[midi] retry ${attempt + 1}/${MAX_RETRIES} for ${addressKey(address)}`,
          );
        } else {
          throw e;
        }
      }
    }
    // Unreachable, but TypeScript needs it.
    throw new Error("unreachable");
  }

  private requestDataOnce(
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
      const pending: PendingRequest = {
        addressKey: key,
        resolve: (data) => {
          pending.cancelSent?.();
          resolve(data);
        },
        reject,
      };
      this.pendingRequests.push(pending);

      // Throttled through the Rust queue; the timeout starts on send.
      pending.cancelSent = this.queueRaw("rq1:" + key, bytes, () => {
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

  private handleMidiMessage(raw: Uint8Array): void {
    if (raw.length < 14) return;
    if (raw[0] !== SYSEX_START) return;

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
    const reqIdx = this.pendingRequests.findIndex((r) => r.addressKey === key);
    if (reqIdx >= 0) {
      const req = this.pendingRequests[reqIdx]!;
      if (req.timer) clearTimeout(req.timer);
      this.pendingRequests.splice(reqIdx, 1);
      req.resolve(data);
    }

    // Feed to Rust state machine (handles echo suppression internally).
    const now = performance.now();
    if (!this.device.handleExpansionDt1(addr, data, now)) {
      this.device.handleDt1(addr, data, now);
    }

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
  // Chorus (FX1)
  // -----------------------------------------------------------------------

  setChorusSwitch(_enabled: boolean): void {
    this.device.toggleChorusSwitch();
  }

  setChorusParam(offset: number, value: number): void {
    this.device.setChorusParam(offset, value);
  }

  setChorusNibParam(paramIndex: number, value: number): void {
    this.device.setChorusNibParam(paramIndex, value);
  }

  // -----------------------------------------------------------------------
  // Reverb (FX2)
  // -----------------------------------------------------------------------

  setReverbSwitch(_enabled: boolean): void {
    this.device.toggleReverbSwitch();
  }

  setReverbParam(offset: number, value: number): void {
    this.device.setReverbParam(offset, value);
  }

  setReverbNibParam(paramIndex: number, value: number): void {
    this.device.setReverbNibParam(paramIndex, value);
  }

  // -----------------------------------------------------------------------
  // Ext Part
  // -----------------------------------------------------------------------

  setExtPartLevel(value: number): void {
    this.device.setExtLevel(value);
  }

  setExtPartMute(_muted: boolean): void {
    this.device.toggleExtMute();
  }

  // -----------------------------------------------------------------------
  // Drum Comp+EQ
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Motional Surround
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Request helpers (read from device)
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Catalog queries (stay in TS — event-driven with timeouts)
  // -----------------------------------------------------------------------

  async requestStudioSetNames(): Promise<Map<number, string>> {
    const names = new Map<number, string>();
    const { bankMsb: studioSetMsb, slotCount } = studioSetType();
    const msg = new Uint8Array(build_studio_set_catalog_request(this.deviceId));

    return new Promise((resolve) => {
      let timeoutId: ReturnType<typeof setTimeout> | undefined;
      let absoluteTimeout: ReturnType<typeof setTimeout> | undefined;

      const done = () => {
        clearTimeout(timeoutId);
        clearTimeout(absoluteTimeout);
        cancelSent();
        cleanup();
        resolve(names);
      };

      const resetTimeout = () => {
        if (absoluteTimeout === undefined) return; // still queued
        clearTimeout(timeoutId);
        timeoutId = setTimeout(done, 3000);
      };

      const handler = (raw: Uint8Array) => {
        if (raw.length < 14 || raw[0] !== SYSEX_START) return;

        let parsed;
        try {
          parsed = parse_dt1(new Uint8Array(raw));
        } catch {
          return;
        }

        const data = new Uint8Array(parsed.data());
        const entry = parse_catalog_entry(data);
        // Other catalog reads may be in flight: take only Studio Set entries.
        if (entry && entry.bank_msb !== studioSetMsb) {
          entry.free();
        } else if (entry) {
          names.set(entry.pc, entry.name());
          entry.free();
          if (names.size >= slotCount) {
            done();
            return;
          }
        }
        resetTimeout();
      };

      const cleanup = this.port.listen(handler);

      // Timeouts start once the request leaves the queue.
      const cancelSent = this.queueRaw("catalog", msg, () => {
        absoluteTimeout = setTimeout(done, 15000);
        resetTimeout();
      });
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
      let timeoutId: ReturnType<typeof setTimeout> | undefined;
      let absoluteTimeout: ReturnType<typeof setTimeout> | undefined;

      const done = () => {
        clearTimeout(timeoutId);
        clearTimeout(absoluteTimeout);
        cancelSent();
        cleanup();
        resolve(entries);
      };

      const resetTimeout = () => {
        if (absoluteTimeout === undefined) return; // still queued
        clearTimeout(timeoutId);
        timeoutId = setTimeout(done, 500);
      };

      const handler = (raw: Uint8Array) => {
        if (raw.length < 14 || raw[0] !== SYSEX_START) return;

        let parsed;
        try {
          parsed = parse_dt1(new Uint8Array(raw));
        } catch {
          return;
        }

        const data = new Uint8Array(parsed.data());
        const entry = parse_catalog_entry(data);
        // Other catalog reads may be in flight: take only this bank's entries.
        if (entry && (entry.bank_msb !== msb || entry.bank_lsb !== lsb)) {
          entry.free();
        } else if (entry) {
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

      const cleanup = this.port.listen(handler);

      // Timeouts start once the request leaves the queue.
      const cancelSent = this.queueRaw(
        `tone-catalog:${msb}:${lsb}:${start}`,
        msg,
        () => {
          absoluteTimeout = setTimeout(done, 5000);
          resetTimeout();
        },
      );
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
  // User-memory writes (undocumented — docs/midi/99-undocumented.md §4)
  // -----------------------------------------------------------------------

  /** Save a part's Temporary tone into the user slot (msb, lsb, pc). */
  saveToneToUser(
    part: number,
    msb: number,
    lsb: number,
    pc: number,
  ): Promise<void> {
    return this.writeToUser(
      new Uint8Array(
        build_tone_write_request(this.deviceId, part, msb, lsb, pc),
      ),
      (r) => r.part === part && r.msb === msb && r.lsb === lsb && r.pc === pc,
    );
  }

  /** Save the Temporary Studio Set into user slot `pc` (0-indexed). */
  saveStudioSetToUser(pc: number): Promise<void> {
    return this.writeToUser(
      new Uint8Array(build_studio_set_write_request(this.deviceId, pc)),
      (r) => r.part === undefined && r.pc === pc,
    );
  }

  /**
   * Send a write request and resolve once the device reports the write
   * `matches` describes finished (2–3 s). Rejects if another write is
   * still in flight: the device handles one at a time.
   */
  private writeToUser(
    msg: Uint8Array,
    matches: (reply: WriteReply) => boolean,
  ): Promise<void> {
    if (this.writing) {
      return Promise.reject(new Error("Another save is still in progress"));
    }
    if (this.device.expansionLoadTarget()) {
      return Promise.reject(new Error("An Expansion load is in progress"));
    }
    this.writing = true;
    return new Promise<void>((resolve, reject) => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const unsubscribe = this.onDt1((addr, data) => {
        const reply = parse_write_reply(addr, data);
        if (!reply) return;
        const done = reply.finished && matches(reply);
        reply.free();
        if (!done) return;
        clearTimeout(timer);
        cancelSent();
        unsubscribe();
        resolve();
      });
      // The timeout starts once the request leaves the queue.
      const cancelSent = this.queueRaw("write", msg, () => {
        timer = setTimeout(() => {
          unsubscribe();
          reject(new Error("No reply from the INTEGRA-7 while saving"));
        }, write_timeout_ms());
      });
    }).finally(() => {
      this.writing = false;
    });
  }

  // -----------------------------------------------------------------------
  // Expansion slots (undocumented — docs/midi/99-undocumented.md §5)
  // -----------------------------------------------------------------------

  /**
   * Read the loaded Expansion slots into the device state and resolve
   * with their slot values A–D, or null if unknown. No answer within
   * SLOT_READ_TIMEOUT_MS makes them unknown. During a load, sends nothing
   * (the device is deaf; the load reads the slots itself) and resolves
   * with the slots known before it.
   */
  readExpansionSlots(): Promise<Uint8Array | null> {
    if (this.device.expansionLoadTarget()) {
      return Promise.resolve(this.device.expansionSlots() ?? null);
    }
    if (this.slotRead) return this.slotRead;
    const replyKey = addressKey(Array.from(expansionSlotsAddress()));
    this.slotRead = new Promise<Uint8Array | null>((resolve) => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const unsubscribe = this.onDt1((addr) => {
        if (addressKey(Array.from(addr)) !== replyKey) return;
        clearTimeout(timer);
        cancelSent();
        unsubscribe();
        resolve(this.device.expansionSlots() ?? null);
      });
      const bytes = this.device.buildExpansionSlotsRequest();
      const cancelSent = this.queueRaw("expansion-slots", bytes, () => {
        timer = setTimeout(() => {
          unsubscribe();
          // A load started meanwhile owns the slots until it completes.
          if (!this.device.expansionLoadTarget()) {
            this.device.expansionSlotsTimedOut();
          }
          resolve(this.device.expansionSlots() ?? null);
        }, SLOT_READ_TIMEOUT_MS);
      });
    }).finally(() => {
      this.slotRead = null;
    });
    return this.slotRead;
  }

  /**
   * Load `target` (slot values A–D) into the Expansion slots and resolve
   * with how the load ended; a failed or timed-out load resolves too.
   * The send queue holds from request to completion; the machine's own
   * sends bypass it, paced by its waits. Rejects at once if the slots are
   * unknown, `target` is invalid, or a save or another load is running.
   */
  loadExpansions(target: ArrayLike<number>): Promise<ExpansionLoadResult> {
    if (this.writing) {
      return Promise.reject(new Error("A save is still in progress"));
    }
    try {
      this.device.startExpansionLoad(Uint8Array.from(target));
    } catch (e) {
      return Promise.reject(e instanceof Error ? e : new Error(String(e)));
    }
    return new Promise<ExpansionLoadResult>((resolve, reject) => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const step = (): void => {
        clearTimeout(timer);
        let action = this.device.nextLoadAction(performance.now());
        while (
          action?.kind === LoadActionKind.Send ||
          action?.kind === LoadActionKind.Poll
        ) {
          this.sendSysex(action.bytes);
          action.free();
          action = this.device.nextLoadAction(performance.now());
        }
        if (action?.kind === LoadActionKind.Wait) {
          timer = setTimeout(step, action.waitMs);
          action.free();
          return;
        }
        unsubscribe();
        if (!action || action.outcome === undefined) {
          action?.free();
          reject(new Error("The Expansion load stopped unexpectedly"));
          return;
        }
        const result = { outcome: action.outcome, slots: action.slots ?? null };
        action.free();
        resolve(result);
      };
      // Every DT1 may advance the load (ack, done, poll answer).
      const unsubscribe = this.onDt1(() => step());
      step();
    });
  }

  // -----------------------------------------------------------------------
  // Librarian: Pack export and import (docs/midi/99-undocumented.md §2, §4)
  // -----------------------------------------------------------------------

  /**
   * Export User slots `first` to `last` (0-indexed, inclusive) of `kind`
   * (a `LibrarianType` kind) into sound-export SVD bytes. Studio Sets carry
   * the User tones their Parts use.
   */
  async exportUserSlots(
    kind: string,
    first: number,
    last: number,
    signal?: AbortSignal,
  ): Promise<Uint8Array> {
    const sources: ExportSource[] = [];
    for (let slot = first; slot <= last; slot++) {
      sources.push({ source: "user", kind, slot });
    }
    const pack = await this.exportPack(sources, signal);
    try {
      return pack.toSvd();
    } finally {
      pack.free();
    }
  }

  /**
   * Read `sources` from the device into a Pack. Only reads. `onProgress`
   * hears where the export stands as it goes, and once more at the end.
   */
  async exportPack(
    sources: ExportSource[],
    signal?: AbortSignal,
    onProgress?: (progress: PackExportProgress) => void,
  ): Promise<Pack> {
    const machine = new PackExport(this.deviceId, sources);
    const report = (): void =>
      onProgress?.({
        progress: machine.progress(),
        autoIncluded: machine.autoIncluded(),
      });
    const throttle = frameThrottle(report);
    try {
      const error = await this.runLibrarian(
        machine,
        "pack-export",
        signal,
        onProgress && throttle.schedule,
      );
      report();
      const pack = error === undefined ? machine.pack() : undefined;
      if (!pack) throw new Error(error ?? "The export produced no Pack");
      return pack;
    } finally {
      throttle.cancel();
      machine.free();
    }
  }

  /**
   * Store `pack` into User memory as `plan` says, borrowing Part `part`
   * (0-indexed) to store tones. With `applyEffects`, a lone entry loaded
   * into Temporary also sets the Temporary Studio Set's Chorus and Reverb.
   * Resolves with each entry's status once the
   * borrowed Temporary areas are restored; refresh mixer state then.
   * `onProgress` hears each entry's status as the import goes.
   */
  async importPack(
    pack: Pack,
    plan: ImportPlan,
    part: number,
    applyEffects: boolean,
    signal?: AbortSignal,
    onProgress?: (statuses: ImportStatus[]) => void,
  ): Promise<PackImportResult> {
    return this.runImport(
      () => new PackImport(this.deviceId, pack, plan, part, applyEffects),
      signal,
      onProgress,
    );
  }

  /**
   * Store INIT into the User `slots` (0-indexed) of type `kind` through the
   * device's Init, borrowing Part `part` (0-indexed) to store tones.
   * Resolves like `importPack`, statuses indexed like `slots`.
   */
  async reinitSlots(
    kind: string,
    slots: number[],
    part: number,
    signal?: AbortSignal,
    onProgress?: (statuses: ImportStatus[]) => void,
  ): Promise<PackImportResult> {
    return this.runImport(
      () =>
        PackImport.reinit(this.deviceId, kind, Uint16Array.from(slots), part),
      signal,
      onProgress,
    );
  }

  /** Drive the import `create` makes to its end, then free it. */
  private async runImport(
    create: () => PackImport,
    signal?: AbortSignal,
    onProgress?: (statuses: ImportStatus[]) => void,
  ): Promise<PackImportResult> {
    if (this.writing) throw new Error("A save is still in progress");
    if (this.device.expansionLoadTarget()) {
      throw new Error("An Expansion load is in progress");
    }
    const machine = create();
    const throttle = frameThrottle(() => onProgress?.(machine.progress()));
    this.writing = true;
    try {
      const error = await this.runLibrarian(
        machine,
        "pack-import",
        signal,
        onProgress && throttle.schedule,
      );
      return { statuses: machine.progress(), error };
    } finally {
      throttle.cancel();
      this.writing = false;
      machine.free();
    }
  }

  /**
   * Drive `machine` to its end and resolve with its error, if any. Sends go
   * through the raw queue (no DT1 coalescing); a wait times out only once
   * everything queued before it has left the queue. `onStep` runs after
   * each step the machine takes.
   */
  private runLibrarian(
    machine: LibrarianMachine,
    key: string,
    signal?: AbortSignal,
    onStep?: () => void,
  ): Promise<string | undefined> {
    return new Promise((resolve) => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      let waitId = 0;
      /** Whether the awaited write's timeout runs (a cancel mustn't restart it). */
      let awaitingWrite = false;
      let flushed = Promise.resolve();
      const send = (bytes: Uint8Array): void => {
        flushed = new Promise((sent) => this.queueRaw(key, bytes, sent));
      };
      const disarm = (): void => {
        waitId++;
        awaitingWrite = false;
        clearTimeout(timer);
      };
      const arm = (ms: number): void => {
        disarm();
        const id = waitId;
        void flushed.then(() => {
          if (id !== waitId) return;
          timer = setTimeout(() => {
            awaitingWrite = false;
            machine.onTimeout();
            step();
          }, ms);
        });
      };
      const step = (): void => {
        for (;;) {
          const action = machine.nextAction();
          const { kind, bytes, timeoutMs, error } = action;
          action.free();
          switch (kind) {
            case LibrarianActionKind.Read:
              send(bytes);
              arm(REQUEST_TIMEOUT_MS);
              continue;
            case LibrarianActionKind.Send:
              disarm();
              send(bytes);
              continue;
            case LibrarianActionKind.Wait:
              onStep?.();
              return;
            case LibrarianActionKind.AwaitWrite:
              if (!awaitingWrite) {
                arm(timeoutMs);
                awaitingWrite = true;
              }
              onStep?.();
              return;
            case LibrarianActionKind.Done:
              disarm();
              unsubscribe();
              signal?.removeEventListener("abort", abort);
              // Done once the last sends (e.g. the restore) have left.
              void flushed.then(() => resolve(error));
              return;
          }
        }
      };
      const abort = (): void => {
        machine.cancel();
        step();
      };
      const unsubscribe = this.onDt1((addr, data) => {
        if (machine.onRead(addr, data)) {
          // Packets of a block keep its read alive.
          arm(REQUEST_TIMEOUT_MS);
          step();
        } else if (machine.onWriteReply?.(addr, data)) {
          disarm();
          step();
        }
      });
      signal?.addEventListener("abort", abort);
      if (signal?.aborted) machine.cancel();
      step();
    });
  }

  // -----------------------------------------------------------------------
  // Standard MIDI messages (bypass queue — no throttle needed)
  // -----------------------------------------------------------------------

  sendNoteOn(channel: number, note: number, velocity: number): void {
    this.port.send(
      new Uint8Array([
        NOTE_ON | (channel & CHANNEL_MASK),
        note & DATA_MASK,
        velocity & DATA_MASK,
      ]),
    );
  }

  sendNoteOff(channel: number, note: number): void {
    this.port.send(
      new Uint8Array([
        NOTE_OFF | (channel & CHANNEL_MASK),
        note & DATA_MASK,
        0,
      ]),
    );
  }

  /** Send a raw MIDI message at `timestamp` (`performance.now()` clock), or now if omitted. */
  sendMidi(bytes: number[], timestamp?: number): void {
    this.port.send(bytes, timestamp);
  }
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

function addressKey(address: number[]): string {
  return address.map((b) => b.toString(16).padStart(2, "0")).join("");
}
