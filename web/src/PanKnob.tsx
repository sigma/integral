import { useCallback, useRef } from "react";
import css from "./PanKnob.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  label?: string;
}

/** Map 0–127 to rotation degrees. 0=full left (-135°), 64=center (0°), 127=full right (+135°). */
function valueToAngle(value: number): number {
  return ((value - 64) / 64) * 135;
}

/** Format pan value for display. */
function formatPan(value: number): string {
  if (value === 64) return "C";
  if (value < 64) return `L${64 - value}`;
  return `R${value - 64}`;
}

export function PanKnob({ value, onChange, label = "PAN" }: Props) {
  const dragging = useRef(false);
  const lastY = useRef(0);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      lastY.current = e.clientY;
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    [],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = lastY.current - e.clientY; // up = positive
      lastY.current = e.clientY;
      const newValue = Math.max(0, Math.min(127, value + dy));
      onChange(newValue);
    },
    [value, onChange],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const angle = valueToAngle(value);

  return (
    <div className={css.container}>
      <span className={css.label}>{label}</span>
      <svg
        className={css.knob}
        viewBox="0 0 48 48"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        {/* Knob body */}
        <circle cx="24" cy="24" r="18" fill="#2a2a3e" stroke="#555" strokeWidth="1.5" />
        {/* Indicator line */}
        <line
          x1="24"
          y1="24"
          x2="24"
          y2="10"
          stroke="#e0e0e0"
          strokeWidth="2"
          strokeLinecap="round"
          transform={`rotate(${angle} 24 24)`}
        />
        {/* Center dot */}
        <circle cx="24" cy="24" r="2" fill="#888" />
      </svg>
      <span className={css.value}>{formatPan(value)}</span>
    </div>
  );
}
