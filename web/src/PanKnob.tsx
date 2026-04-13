import { useCallback, useRef } from "react";
import css from "./PanKnob.module.css";

interface Props {
  value: number;
  onChange: (value: number) => void;
  label?: string;
  style?: React.CSSProperties;
}

function valueToAngle(value: number): number {
  return ((value - 64) / 64) * 135;
}

function formatPan(value: number): string {
  if (value === 64) return "C";
  if (value < 64) return `L${64 - value}`;
  return `R${value - 64}`;
}

export function PanKnob({ value, onChange, label = "PAN", style }: Props) {
  const dragging = useRef(false);
  const lastY = useRef(0);

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
      const newValue = Math.max(0, Math.min(127, Math.round(value + dy)));
      onChange(newValue);
    },
    [value, onChange],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(64); // center
  }, [onChange]);

  const angle = valueToAngle(value);

  return (
    <div className={css.container} style={style}>
      <span className={css.label}>{label}</span>
      <svg
        className={css.knob}
        viewBox="0 0 44 44"
        tabIndex={0}
        role="slider"
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={127}
        aria-label={label}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDoubleClick={handleDoubleClick}
        onKeyDown={(e) => {
          if (e.key === "ArrowUp" || e.key === "ArrowRight") {
            e.preventDefault();
            onChange(Math.min(127, value + (e.shiftKey ? 10 : 1)));
          } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
            e.preventDefault();
            onChange(Math.max(0, value - (e.shiftKey ? 10 : 1)));
          } else if (e.key === "Home") {
            e.preventDefault();
            onChange(0);
          } else if (e.key === "End") {
            e.preventDefault();
            onChange(127);
          }
        }}
      >
        {/* Outer ring */}
        <circle cx="22" cy="22" r="19" fill="none" stroke="#3a3a5a" strokeWidth="1" />
        {/* Knob body — gradient to look 3D */}
        <defs>
          <radialGradient id="knobGrad" cx="40%" cy="35%">
            <stop offset="0%" stopColor="#888" />
            <stop offset="100%" stopColor="#333" />
          </radialGradient>
        </defs>
        <circle cx="22" cy="22" r="16" fill="url(#knobGrad)" />
        {/* Indicator line */}
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
      <span className={css.value}>{formatPan(value)}</span>
    </div>
  );
}
