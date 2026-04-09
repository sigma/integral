/**
 * useMidiKeyboard — maps computer keyboard to MIDI notes.
 *
 * Keys: a w s e d f t g y h u j k
 * Maps: C C# D D# E F F# G G# A A# B C
 *
 * Sends noteOn on keydown, noteOff on keyup. Tracks held keys to
 * avoid retriggering on key repeat. Octave adjustable with z/x keys.
 */

import { useEffect, useState, useRef, useCallback } from "react";
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

interface Options {
  service: IntegraService | null;
  channel: number; // MIDI channel 0-15
  enabled: boolean;
}

export function useMidiKeyboard({ service, channel, enabled }: Options) {
  const [octave, setOctave] = useState(4);
  const held = useRef(new Set<string>());

  const noteForKey = useCallback(
    (key: string): number | null => {
      const offset = KEY_MAP[key];
      if (offset === undefined) return null;
      const note = octave * 12 + offset;
      return note >= 0 && note <= 127 ? note : null;
    },
    [octave],
  );

  useEffect(() => {
    if (!enabled || !service) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't capture when typing in inputs/selects
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLSelectElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      const key = e.key.toLowerCase();

      // Octave shift
      if (key === "z") {
        setOctave((o) => Math.max(0, o - 1));
        return;
      }
      if (key === "x") {
        setOctave((o) => Math.min(9, o + 1));
        return;
      }

      if (held.current.has(key)) return; // key repeat
      const note = noteForKey(key);
      if (note === null) return;

      held.current.add(key);
      service.sendNoteOn(channel, note, 100);
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if (!held.current.has(key)) return;
      held.current.delete(key);

      const note = noteForKey(key);
      if (note === null) return;
      service.sendNoteOff(channel, note);
    };

    // Release all on blur (e.g. tab switch)
    const handleBlur = () => {
      for (const key of held.current) {
        const note = noteForKey(key);
        if (note !== null) {
          service.sendNoteOff(channel, note);
        }
      }
      held.current.clear();
    };

    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);
    window.addEventListener("blur", handleBlur);

    return () => {
      handleBlur(); // release all on unmount
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("blur", handleBlur);
    };
  }, [enabled, service, channel, noteForKey]);

  return { octave, setOctave };
}
