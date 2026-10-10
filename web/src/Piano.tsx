import type { CSSProperties } from "react";
import { MAX_OCTAVE, MIN_OCTAVE, noteName, type Keyboard } from "./useKeyboard";
import css from "./Piano.module.css";

/** Octaves shown, starting at the Keyboard's octave. */
const OCTAVES = 3;
const BLACK = new Set([1, 3, 6, 8, 10]);
const POINTER = "pointer";

interface Props {
  keyboard: Keyboard;
}

/** On-screen piano. Click plays a note on the selected Part. */
export function Piano({ keyboard }: Props) {
  const { octave, shiftOctave, heldNotes, press, release } = keyboard;
  const first = octave * 12;
  const notes = Array.from({ length: OCTAVES * 12 + 1 }, (_, i) => first + i).filter((n) => n <= 127);
  const whites = notes.filter((n) => !BLACK.has(n % 12));

  return (
    <div className={css.piano}>
      <div className={css.octaveControl}>
        <button className={css.octaveBtn} onClick={() => shiftOctave(-1)} disabled={octave <= MIN_OCTAVE}>
          -
        </button>
        <span className={css.octaveLabel}>Oct {octave}</span>
        <button className={css.octaveBtn} onClick={() => shiftOctave(1)} disabled={octave >= MAX_OCTAVE}>
          +
        </button>
      </div>
      <div className={css.keys} style={{ "--whites": whites.length } as CSSProperties}>
        {notes.map((note) => {
          const black = BLACK.has(note % 12);
          const whiteIndex = whites.filter((w) => w < note).length;
          return (
            <button
              key={note}
              className={`${black ? css.black : css.white} ${heldNotes.has(note) ? css.held : ""}`}
              style={{ "--pos": whiteIndex } as CSSProperties}
              title={noteName(note)}
              onPointerDown={(e) => {
                e.currentTarget.releasePointerCapture(e.pointerId);
                press(POINTER, note);
              }}
              onPointerUp={() => release(POINTER)}
              onPointerLeave={() => release(POINTER)}
            />
          );
        })}
      </div>
    </div>
  );
}
