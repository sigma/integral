import { useCallback, useRef } from "react";
import css from "./VolumeFader.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  defaultValue?: number;
  /** Range minimum. Default 0. */
  min?: number;
  /** Range maximum. Default 127. */
  max?: number;
  /** Formats the value readout and ruler labels. */
  formatValue?: (v: number) => string;
  /** Accessible name. Default "Volume". */
  label?: string;
}

/** The track's height flexes with the window, so positions are fractions of its travel. */
const CAP_HEIGHT = 18;
const PAD = 8;

/** CSS `top` for a point `offset` px below the cap's top edge at `value`. */
function valueToTop(value: number, min: number, max: number, offset = 0): string {
  const frac = 1 - (value - min) / (max - min);
  return `calc(${PAD + offset}px + (100% - ${2 * PAD + CAP_HEIGHT}px) * ${frac})`;
}

function topToValue(top: number, height: number, min: number, max: number): number {
  const ratio = 1 - (top - PAD) / (height - 2 * PAD - CAP_HEIGHT);
  return Math.round(Math.max(min, Math.min(max, min + ratio * (max - min))));
}

/** MIDI value ruler marks for the default 0–127 range. */
const RULER_MARKS = [127, 100, 80, 60, 40, 20, 0];

/** Ruler marks: the MIDI marks for 0–127, else quarters of the range. */
function rulerMarks(min: number, max: number): number[] {
  if (min === 0 && max === 127) return RULER_MARKS;
  return [0, 0.25, 0.5, 0.75, 1].map((f) => Math.round(max - f * (max - min)));
}

export function VolumeFader({
  value,
  onChange,
  defaultValue = 100,
  min = 0,
  max = 127,
  formatValue,
  label = "Volume",
}: Props) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const valueFromClientY = useCallback(
    (clientY: number) => {
      const track = trackRef.current;
      if (!track) return value;
      const rect = track.getBoundingClientRect();
      return topToValue(clientY - rect.top, rect.height, min, max);
    },
    [value, min, max],
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

  const capTop = valueToTop(value, min, max);

  return (
    <div className={css.container}>
      <div
        className={css.track}
        ref={trackRef}
        tabIndex={0}
        role="slider"
        aria-valuenow={value}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-label={label}
        aria-orientation="vertical"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
        onKeyDown={(e) => {
          if (e.key === "ArrowUp" || e.key === "ArrowRight") {
            e.preventDefault();
            onChange(Math.min(max, value + (e.shiftKey ? 10 : 1)));
          } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
            e.preventDefault();
            onChange(Math.max(min, value - (e.shiftKey ? 10 : 1)));
          } else if (e.key === "Home") {
            e.preventDefault();
            onChange(max);
          } else if (e.key === "End") {
            e.preventDefault();
            onChange(min);
          }
        }}
      >
        {/* Ruler marks */}
        {rulerMarks(min, max).map((midi) => {
          const top = valueToTop(midi, min, max, CAP_HEIGHT / 2);
          const isZeroDb = midi === defaultValue;
          return (
            <div
              key={midi}
              className={`${css.rulerMark} ${isZeroDb ? css.rulerZero : ""}`}
              style={{ top }}
            >
              <span className={css.rulerLabel}>
                {formatValue ? formatValue(midi) : isZeroDb ? "0dB" : midi}
              </span>
            </div>
          );
        })}
        <div className={css.groove} />
        <div className={css.cap} style={{ top: capTop }} />
      </div>
      <span className={css.value}>{formatValue ? formatValue(value) : value}</span>
    </div>
  );
}
