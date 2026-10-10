/**
 * App-level Sequencer engine.
 *
 * Rust (`WasmSequencer`) owns the Pattern and the timing math. This layer owns
 * the clock, the Web MIDI sends and `localStorage`:
 * - A dedicated Worker ticks every 25 ms; Worker timers are not throttled in
 *   background tabs.
 * - Each tick sends everything due in the next 100 ms with timestamped sends.
 * - A Web Lock is held while playing so Energy Saver does not freeze the tab.
 */

import { useSyncExternalStore } from "react";
import { WasmSequencer } from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";

const TICK_MS = 25;
const HORIZON_MS = 100;
const STORAGE_KEY = "integral.pattern";
const LOCK_NAME = "integral-sequencer";
const DEFAULT_VELOCITY = 100;

/** Worker that posts a tick every TICK_MS while started (`true`), silent when `false`. */
const TICK_WORKER = `let id;
onmessage = (e) => {
  clearInterval(id);
  if (e.data) id = setInterval(() => postMessage(0), ${TICK_MS});
};`;

interface TimedMidi {
  tMs: number;
  bytes: number[];
}

export interface NoteView {
  pitch: number;
  velocity: number;
  length: number;
}

export interface StepView {
  step: number;
  notes: NoteView[];
}

export interface TrackView {
  length: number;
  steps: StepView[];
}

/** Everything the UI renders, read from Rust after each change. */
export interface SequencerView {
  playing: boolean;
  bpm: number;
  /** Cursor Step; its Track is the selected Part's Track. */
  cursor: number;
  /** Step input is armed: played notes are written at the Cursor. */
  armed: boolean;
  /** Velocity written by Step input. */
  defaultVelocity: number;
  tracks: TrackView[];
}

function bytesToBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes));
}

function base64ToBytes(text: string): Uint8Array {
  return Uint8Array.from(atob(text), (c) => c.charCodeAt(0));
}

export class SequencerEngine {
  readonly seq = new WasmSequencer();
  private service: IntegraService | null = null;
  private worker: Worker;
  private releaseLock: (() => void) | null = null;
  private listeners = new Set<() => void>();
  private cursor = 0;
  private track = 0;
  private armed = false;
  private defaultVelocity = DEFAULT_VELOCITY;
  /** Notes held for Step input, and the chord they will write. */
  private held = new Set<number>();
  private pending = new Set<number>();
  private pendingLength = 1;
  private snapshot: SequencerView;

  constructor() {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        this.seq.loadSmf(base64ToBytes(saved));
      } catch (e) {
        console.warn("[sequencer] ignoring saved pattern:", e);
      }
    }
    this.snapshot = this.read();
    const url = URL.createObjectURL(new Blob([TICK_WORKER], { type: "text/javascript" }));
    this.worker = new Worker(url);
    this.worker.onmessage = () => this.tick();
    window.addEventListener("pagehide", () => this.stop(false));
  }

  /** Point sends at a (re)connected device, or at nothing. Playback continues. */
  setService(service: IntegraService | null): void {
    this.service = service;
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getSnapshot = (): SequencerView => this.snapshot;

  /** The global Step sounding now, or undefined when stopped. */
  playStep(): number | undefined {
    return this.seq.stepAt(performance.now());
  }

  setCursor(step: number): void {
    this.cursor = step;
    this.changed();
  }

  /** The selected Part's Track, where Step input writes. */
  setTrack(track: number): void {
    this.track = track;
  }

  setArmed(armed: boolean): void {
    this.armed = armed;
    this.held.clear();
    this.pending.clear();
    this.changed();
  }

  setDefaultVelocity(velocity: number): void {
    this.defaultVelocity = velocity;
    this.changed();
  }

  /** A Keyboard note went down. While armed it joins the pending chord. */
  noteDown(note: number): void {
    if (!this.armed) return;
    if (this.pending.size === 0) this.pendingLength = 1;
    this.held.add(note);
    this.pending.add(note);
  }

  /**
   * A Keyboard note went up. Releasing the last held note writes the pending
   * chord at the Cursor, unless `hold` (piano shift-click) keeps it pending.
   */
  noteUp(note: number, hold = false): void {
    if (!this.armed) return;
    this.held.delete(note);
    if (hold || this.held.size > 0 || this.pending.size === 0) return;
    const pitches = Uint8Array.from(this.pending);
    const length = this.pendingLength;
    this.pending.clear();
    this.edit((seq) => seq.setStep(this.track, this.cursor, pitches, this.defaultVelocity, length));
    this.moveCursor(length);
  }

  /** Transport and Step input keys. Returns true if the key was consumed. */
  handleKey(key: string, repeat: boolean): boolean {
    if (key === " ") {
      if (!repeat) this.togglePlay();
      return true;
    }
    if (!this.armed) return false;
    const holding = this.held.size > 0;
    if (key === "ArrowRight") {
      if (holding) this.pendingLength = Math.min(WasmSequencer.maxSteps(), this.pendingLength + 1);
      else this.moveCursor(1);
      return true;
    }
    if (holding) return false;
    if (key === "ArrowLeft") {
      this.moveCursor(-1);
      return true;
    }
    if (key === "Backspace") {
      this.edit((seq) => seq.setStep(this.track, this.cursor, new Uint8Array(), 0, 1));
      this.moveCursor(-1);
      return true;
    }
    return false;
  }

  togglePlay(): void {
    if (this.seq.isPlaying()) this.stop();
    else this.play();
  }

  /** Move the Cursor, wrapping within the Track's length. */
  private moveCursor(delta: number): void {
    const length = this.snapshot.tracks[this.track]?.length ?? 1;
    this.cursor = (((this.cursor + delta) % length) + length) % length;
    this.changed();
  }

  /** Apply a Pattern edit, persist it and re-render. */
  edit(apply: (seq: WasmSequencer) => void): void {
    apply(this.seq);
    this.save();
    this.changed();
  }

  play(): void {
    if (this.seq.isPlaying()) return;
    this.seq.play(performance.now());
    this.worker.postMessage(true);
    void navigator.locks?.request(
      LOCK_NAME,
      () => new Promise<void>((resolve) => (this.releaseLock = resolve)),
    );
    this.tick();
    this.changed();
  }

  /** Stop and silence every Track. Untimed sends are for `pagehide`. */
  stop(timed = true): void {
    if (!this.seq.isPlaying()) return;
    this.worker.postMessage(false);
    this.releaseLock?.();
    this.releaseLock = null;
    if (this.service) {
      const events = this.seq.stop(this.service.device, performance.now()) as TimedMidi[];
      this.send(events, timed);
    }
    this.changed();
  }

  /** Send All Notes Off on every MIDI channel, now. */
  panic(): void {
    for (const bytes of WasmSequencer.panic() as number[][]) {
      this.service?.sendMidi(bytes);
    }
  }

  /** The Pattern as `.mid` bytes, or null without a device to read channels from. */
  toSmf(): Uint8Array | null {
    return this.service ? this.seq.toSmf(this.service.device) : null;
  }

  /** Replace the Pattern with a `.mid` file. Throws (Pattern unchanged) if it is rejected. */
  loadSmf(bytes: Uint8Array): void {
    this.edit((seq) => seq.loadSmf(bytes));
  }

  setBpm(bpm: number): void {
    this.edit((seq) => seq.setBpm(bpm));
  }

  private save(): void {
    const bytes = this.toSmf();
    if (bytes) localStorage.setItem(STORAGE_KEY, bytesToBase64(bytes));
  }

  private tick(): void {
    if (!this.service) return;
    const events = this.seq.due(this.service.device, performance.now(), HORIZON_MS) as TimedMidi[];
    this.send(events, true);
  }

  private send(events: TimedMidi[], timed: boolean): void {
    for (const e of events) {
      this.service?.sendMidi(e.bytes, timed ? e.tMs : undefined);
    }
  }

  private read(): SequencerView {
    return {
      playing: this.seq.isPlaying(),
      bpm: this.seq.bpm(),
      cursor: this.cursor,
      armed: this.armed,
      defaultVelocity: this.defaultVelocity,
      tracks: this.seq.tracks() as TrackView[],
    };
  }

  private changed(): void {
    this.snapshot = this.read();
    for (const l of this.listeners) l();
  }
}

let engine: SequencerEngine | null = null;

/** The app's single Sequencer engine. Call only after WASM has loaded. */
export function sequencerEngine(): SequencerEngine {
  engine ??= new SequencerEngine();
  return engine;
}

/** Subscribe a component to the Sequencer's state. */
export function useSequencer(engine: SequencerEngine): SequencerView {
  return useSyncExternalStore(engine.subscribe, engine.getSnapshot);
}
