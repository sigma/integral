import { useCallback, useRef } from "react";
import css from "./SynthFader.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  label: string;
  onChange: (value: number) => void;
  formatValue?: (v: number) => string;
  /** Compact height (~70px track) vs standard (~100px). */
  compact?: boolean;
}

const TRACK_HEIGHT = 100;
const TRACK_HEIGHT_COMPACT = 70;
const TICK_COUNT = 11; // number of ruler tick marks

export function SynthFader({
  value,
  min,
  max,
  defaultValue,
  label,
  onChange,
  formatValue,
  compact,
}: Props) {
  const trackH = compact ? TRACK_HEIGHT_COMPACT : TRACK_HEIGHT;
  const dragging = useRef(false);
  const startY = useRef(0);
  const startVal = useRef(0);

  const range = max - min || 1;
  const norm = (value - min) / range; // 0..1
  const defaultNorm = (defaultValue - min) / range;
  const isDefault = value === defaultValue;

  // Fill: from default to current value (red when not default)
  const fillBottom = Math.min(norm, defaultNorm) * trackH;
  const fillTop = Math.max(norm, defaultNorm) * trackH;
  const fillHeight = fillTop - fillBottom;

  const thumbY = (1 - norm) * trackH;

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      startY.current = e.clientY;
      startVal.current = value;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [value],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = startY.current - e.clientY;
      const delta = (dy / trackH) * range;
      const newVal = Math.round(
        Math.max(min, Math.min(max, startVal.current + delta)),
      );
      onChange(newVal);
    },
    [min, max, range, trackH, onChange],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [defaultValue, onChange]);

  // Generate tick marks
  const ticks = [];
  for (let i = 0; i < TICK_COUNT; i++) {
    const y = (i / (TICK_COUNT - 1)) * trackH;
    const isMajor = i === 0 || i === TICK_COUNT - 1 || i === Math.floor(TICK_COUNT / 2);
    ticks.push(
      <div
        key={i}
        className={`${css.tick} ${isMajor ? css.tickMajor : ""}`}
        style={{ top: y }}
      />,
    );
  }

  const display = formatValue ? formatValue(value) : String(value);

  return (
    <div className={`${css.fader} ${compact ? css.compact : ""}`}>
      <span className={css.label}>{label}</span>
      <div
        className={css.trackArea}
        style={{ height: trackH }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
      >
        {/* Ruler ticks */}
        <div className={css.ruler}>{ticks}</div>

        {/* Track groove */}
        <div className={css.track}>
          {/* Fill bar (red when non-default) */}
          {!isDefault && (
            <div
              className={css.fill}
              style={{
                bottom: fillBottom,
                height: fillHeight,
              }}
            />
          )}
        </div>

        {/* Thumb */}
        <div
          className={`${css.thumb} ${!isDefault ? css.thumbActive : ""}`}
          style={{ top: thumbY }}
        />
      </div>
      <span className={css.value}>{display}</span>
    </div>
  );
}
