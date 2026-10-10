/**
 * useKeyboard — the app-wide Keyboard: computer-keyboard note mapping plus
 * the on-screen piano, sharing one octave and one set of held notes.
 *
 * Keys: a w s e d f t g y h u j k
 * Maps: C C# D D# E F F# G G# A A# B C
 *
 * z / x shift the octave. Notes and transport keys also feed Step input. Each held note remembers the service and channel it
 * started on, so its Note Off goes there even if the selected Part changes.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import type { IntegraService } from "./integra";

const KEY_MAP: Record<string, number> = {
  a: 0,  // C
  w: 1,  // C#
  s: 2,  // D
  e: 3,  // D#
  d: 4,  // E
  f: 5,  // F
  t: 6,  // F#
  g: 7,  // G
  y: 8,  // G#
  h: 9,  // A
  u: 10, // A#
  j: 11, // B
  k: 12, // C (next octave)
};

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/** Display name of a MIDI note, e.g. 60 → "C4". */
export function noteName(note: number): string {
  return `${NOTE_NAMES[note % 12]}${Math.floor(note / 12) - 1}`;
}

export const MIN_OCTAVE = 0;
export const MAX_OCTAVE = 9;
const DEFAULT_OCTAVE = 4;
const VELOCITY = 100;

/** Focus targets whose keystrokes are never notes. */
const TEXT_ENTRY = 'input, select, textarea, [contenteditable]:not([contenteditable="false"]), [role="slider"]';

/** Whether a key event belongs to the Keyboard rather than to text entry or a shortcut. */
function isNoteKey(e: KeyboardEvent): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey) return false;
  return !(e.target instanceof Element && e.target.closest(TEXT_ENTRY));
}

interface Held {
  note: number;
  channel: number;
  service: IntegraService;
}

/** Receives Keyboard notes and keys for Step input (the Sequencer engine). */
export interface StepInput {
  noteDown: (note: number) => void;
  noteUp: (note: number, hold?: boolean) => void;
  /** Returns true if the key was consumed. */
  handleKey: (key: string, repeat: boolean) => boolean;
}

export interface Keyboard {
  octave: number;
  shiftOctave: (delta: number) => void;
  /** Notes currently held, for display. */
  heldNotes: ReadonlySet<number>;
  /** Start `note` for `source` (one note per source). */
  press: (source: string, note: number) => void;
  /** Release whatever `source` holds; `hold` keeps it in the pending chord. */
  release: (source: string, hold?: boolean) => void;
}

/** Live play on `channel` (the selected Part's receive channel), plus Step input. */
export function useKeyboard(
  service: IntegraService | null,
  channel: number,
  stepInput: StepInput | null,
): Keyboard {
  const [octave, setOctave] = useState(DEFAULT_OCTAVE);
  const [heldNotes, setHeldNotes] = useState<ReadonlySet<number>>(new Set());
  const held = useRef(new Map<string, Held>());
  const latest = useRef({ service, channel, octave, stepInput });
  latest.current = { service, channel, octave, stepInput };

  const sync = () => setHeldNotes(new Set([...held.current.values()].map((h) => h.note)));

  const press = useCallback((source: string, note: number) => {
    const { service, channel } = latest.current;
    if (!service || held.current.has(source) || note < 0 || note > 127) return;
    held.current.set(source, { note, channel, service });
    service.sendNoteOn(channel, note, VELOCITY);
    latest.current.stepInput?.noteDown(note);
    sync();
  }, []);

  const release = useCallback((source: string, hold = false) => {
    const h = held.current.get(source);
    if (!h) return;
    held.current.delete(source);
    h.service.sendNoteOff(h.channel, h.note);
    latest.current.stepInput?.noteUp(h.note, hold);
    sync();
  }, []);

  const shiftOctave = useCallback((delta: number) => {
    setOctave((o) => Math.min(MAX_OCTAVE, Math.max(MIN_OCTAVE, o + delta)));
  }, []);

  useEffect(() => {
    const releaseAll = () => {
      // Losing focus is not a deliberate release: keep the chord pending.
      for (const source of [...held.current.keys()]) release(source, true);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isNoteKey(e)) return;
      if (latest.current.stepInput?.handleKey(e.key, e.repeat)) {
        e.preventDefault();
        return;
      }
      const key = e.key.toLowerCase();
      if (key === "z") return shiftOctave(-1);
      if (key === "x") return shiftOctave(1);
      const offset = KEY_MAP[key];
      if (offset === undefined) return;
      press(`key:${key}`, latest.current.octave * 12 + offset);
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      // Space toggles the transport on keydown; don't let it also click a focused button.
      if (e.key === " " && isNoteKey(e)) e.preventDefault();
      release(`key:${e.key.toLowerCase()}`);
    };

    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);
    window.addEventListener("blur", releaseAll);
    return () => {
      releaseAll();
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("blur", releaseAll);
    };
  }, [press, release, shiftOctave]);

  return { octave, shiftOctave, heldNotes, press, release };
}
