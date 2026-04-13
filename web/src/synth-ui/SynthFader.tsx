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
  /** Compact fader track height (~70px) vs standard (~100px). */
  compact?: boolean;
  /** Custom track height in pixels. Overrides compact. */
  trackHeight?: number;
  /** Hide the label zone (used when an envelope curve replaces it). */
  hideLabel?: boolean;
  /** Full name for tooltip. Defaults to label. */
  title?: string;
}

const TRACK_HEIGHT = 100;
const TRACK_HEIGHT_COMPACT = 70;
const TICK_COUNT = 11;

export const FADER_LABEL_HEIGHT = 28; // px — fixed height for the label zone

export function SynthFader({
  value,
  min,
  max,
  defaultValue,
  label,
  onChange,
  formatValue,
  compact,
  trackHeight,
  hideLabel,
  title,
}: Props) {
  const trackH = trackHeight ?? (compact ? TRACK_HEIGHT_COMPACT : TRACK_HEIGHT);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startVal = useRef(0);

  const range = max - min || 1;
  const norm = (value - min) / range;
  const defaultNorm = (defaultValue - min) / range;
  const isDefault = value === defaultValue;

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
      onChange(Math.round(Math.max(min, Math.min(max, startVal.current + delta))));
    },
    [min, max, range, trackH, onChange],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [defaultValue, onChange]);

  const ticks = [];
  for (let i = 0; i < TICK_COUNT; i++) {
    const y = (i / (TICK_COUNT - 1)) * trackH;
    const isMajor = i === 0 || i === TICK_COUNT - 1 || i === Math.floor(TICK_COUNT / 2);
    ticks.push(
      <div key={i} className={`${css.tick} ${isMajor ? css.tickMajor : ""}`} style={{ top: y }} />,
    );
  }

  const display = formatValue ? formatValue(value) : String(value);

  return (
    <div className={`${css.fader} ${compact ? css.compact : ""}`} title={title ?? label}>
      {/* Label zone — fixed height so faders align across groups */}
      {!hideLabel && (
        <div className={css.labelZone} style={{ height: FADER_LABEL_HEIGHT }}>
          <span className={css.label}>{label}</span>
        </div>
      )}
      {/* Fader zone */}
      <div
        className={css.trackArea}
        style={{ height: trackH }}
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
        <div className={css.ruler}>{ticks}</div>
        <div className={css.track}>
          {!isDefault && (
            <div className={css.fill} style={{ bottom: fillBottom, height: fillHeight }} />
          )}
        </div>
        <div className={`${css.thumb} ${!isDefault ? css.thumbActive : ""}`} style={{ top: thumbY }} />
      </div>
      <span className={css.value}>{display}</span>
    </div>
  );
}
