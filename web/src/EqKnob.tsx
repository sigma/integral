import { useCallback, useRef } from "react";
import css from "./EqKnob.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  onChange: (value: number) => void;
  formatValue: (value: number) => string;
  color?: string;
}

function valueToAngle(value: number, min: number, max: number): number {
  const range = max - min;
  if (range === 0) return 0;
  return ((value - min) / range - 0.5) * 270;
}

export function EqKnob({
  value,
  min,
  max,
  defaultValue,
  onChange,
  formatValue,
  color = "#888",
}: Props) {
  const dragging = useRef(false);
  const lastY = useRef(0);
  const range = max - min;

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    dragging.current = true;
    lastY.current = e.clientY;
    e.currentTarget.setPointerCapture(e.pointerId);
  }, []);

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = lastY.current - e.clientY;
      lastY.current = e.clientY;
      // Scale sensitivity: full range over ~100px of drag
      const step = dy * (range / 100);
      const newValue = Math.max(min, Math.min(max, Math.round(value + step)));
      onChange(newValue);
    },
    [value, onChange, min, max, range],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [onChange, defaultValue]);

  const angle = valueToAngle(value, min, max);

  return (
    <div className={css.container}>
      <svg
        className={css.knob}
        viewBox="0 0 32 32"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
      >
        <circle cx="16" cy="16" r="13" fill="none" stroke="#3a3a5a" strokeWidth="1" />
        <defs>
          <radialGradient id={`eqKnobGrad-${color}`} cx="40%" cy="35%">
            <stop offset="0%" stopColor={color} />
            <stop offset="100%" stopColor="#222" />
          </radialGradient>
        </defs>
        <circle cx="16" cy="16" r="11" fill={`url(#eqKnobGrad-${color})`} />
        <line
          x1="16"
          y1="16"
          x2="16"
          y2="6"
          stroke="#fff"
          strokeWidth="1.5"
          strokeLinecap="round"
          transform={`rotate(${angle} 16 16)`}
        />
      </svg>
      <span className={css.value}>{formatValue(value)}</span>
    </div>
  );
}
