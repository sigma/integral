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

  setBpm(bpm: number): void {
    this.edit((seq) => seq.setBpm(bpm));
  }

  private save(): void {
    if (!this.service) return;
    localStorage.setItem(STORAGE_KEY, bytesToBase64(this.seq.toSmf(this.service.device)));
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
