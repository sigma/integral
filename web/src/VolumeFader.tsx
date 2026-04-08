import { useCallback, useRef } from "react";
import css from "./VolumeFader.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  defaultValue?: number;
}

const PADDING = 8; // px padding top/bottom inside the track

export function VolumeFader({ value, onChange, defaultValue = 100 }: Props) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const valueFromY = useCallback((clientY: number) => {
    const track = trackRef.current;
    if (!track) return value;
    const rect = track.getBoundingClientRect();
    const usableHeight = rect.height - PADDING * 2;
    const y = clientY - rect.top - PADDING;
    // Invert: top = max, bottom = min
    const ratio = 1 - Math.max(0, Math.min(1, y / usableHeight));
    return Math.round(ratio * 127);
  }, [value]);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      e.currentTarget.setPointerCapture(e.pointerId);
      onChange(valueFromY(e.clientY));
    },
    [onChange, valueFromY],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      onChange(valueFromY(e.clientY));
    },
    [onChange, valueFromY],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [onChange, defaultValue]);

  // Position: 0 = bottom, 127 = top
  const pct = `${((1 - value / 127) * 100).toFixed(1)}%`;

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
        <div className={css.groove} />
        <div className={css.cap} style={{ top: `calc(${pct} + ${PADDING}px - 8px)` }} />
      </div>
      <span className={css.value}>{value}</span>
    </div>
  );
}
