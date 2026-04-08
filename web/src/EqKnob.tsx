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
  label?: string;
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
  label,
}: Props) {
  const dragging = useRef(false);
  const lastY = useRef(0);
  const accumulator = useRef(0);
  const range = max - min;
  // Pixels per step: scale so small ranges are still usable
  const pxPerStep = range > 0 ? Math.max(3, Math.min(8, 100 / range)) : 5;

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    dragging.current = true;
    lastY.current = e.clientY;
    accumulator.current = 0;
    e.currentTarget.setPointerCapture(e.pointerId);
  }, []);

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = lastY.current - e.clientY;
      lastY.current = e.clientY;
      accumulator.current += dy;

      // Convert accumulated pixels to integer steps
      const steps = Math.trunc(accumulator.current / pxPerStep);
      if (steps !== 0) {
        accumulator.current -= steps * pxPerStep;
        const newValue = Math.max(min, Math.min(max, value + steps));
        onChange(newValue);
      }
    },
    [value, onChange, min, max, pxPerStep],
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
      {label && <span className={css.label}>{label}</span>}
      <svg
        className={css.knob}
        viewBox="0 0 44 44"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
      >
        <circle cx="22" cy="22" r="19" fill="none" stroke="#3a3a5a" strokeWidth="1" />
        <defs>
          <radialGradient id={`eqKnobGrad-${color}`} cx="40%" cy="35%">
            <stop offset="0%" stopColor={color} />
            <stop offset="100%" stopColor="#222" />
          </radialGradient>
        </defs>
        <circle cx="22" cy="22" r="16" fill={`url(#eqKnobGrad-${color})`} />
        <line
          x1="22"
          y1="22"
          x2="22"
          y2="9"
          stroke="#fff"
          strokeWidth="2"
          strokeLinecap="round"
          transform={`rotate(${angle} 22 22)`}
        />
      </svg>
      <span className={css.value}>{formatValue(value)}</span>
    </div>
  );
}
