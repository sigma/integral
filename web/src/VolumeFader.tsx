import { useCallback, useRef } from "react";
import css from "./VolumeFader.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  defaultValue?: number;
}

/** Must match .track height in CSS. */
const TRACK_HEIGHT = 588;
const CAP_HEIGHT = 18;
const PAD = 8;
const MIN_TOP = PAD;
const MAX_TOP = TRACK_HEIGHT - PAD - CAP_HEIGHT;

function valueToTop(value: number): number {
  return MIN_TOP + (MAX_TOP - MIN_TOP) * (1 - value / 127);
}

function topToValue(top: number): number {
  const ratio = 1 - (top - MIN_TOP) / (MAX_TOP - MIN_TOP);
  return Math.round(Math.max(0, Math.min(127, ratio * 127)));
}

/** MIDI value ruler marks */
const RULER_MARKS = [127, 100, 80, 60, 40, 20, 0];

export function VolumeFader({ value, onChange, defaultValue = 100 }: Props) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const valueFromClientY = useCallback(
    (clientY: number) => {
      const track = trackRef.current;
      if (!track) return value;
      const rect = track.getBoundingClientRect();
      const top = clientY - rect.top;
      return topToValue(top);
    },
    [value],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      e.currentTarget.setPointerCapture(e.pointerId);
      onChange(valueFromClientY(e.clientY));
    },
    [onChange, valueFromClientY],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      onChange(valueFromClientY(e.clientY));
    },
    [onChange, valueFromClientY],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [onChange, defaultValue]);

  const capTop = valueToTop(value);

  return (
    <div className={css.container}>
      <div
        className={css.track}
        ref={trackRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
      >
        {/* Ruler marks */}
        {RULER_MARKS.map((midi) => {
          const top = valueToTop(midi) + CAP_HEIGHT / 2;
          const isZeroDb = midi === 100;
          return (
            <div
              key={midi}
              className={`${css.rulerMark} ${isZeroDb ? css.rulerZero : ""}`}
              style={{ top }}
            >
              <span className={css.rulerLabel}>
                {isZeroDb ? "0dB" : midi}
              </span>
            </div>
          );
        })}
        <div className={css.groove} />
        <div className={css.cap} style={{ top: capTop }} />
      </div>
      <span className={css.value}>{value}</span>
    </div>
  );
}
