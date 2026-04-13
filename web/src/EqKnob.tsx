import { useDragControl } from "./useDragControl";
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
  const range = max - min;
  const pxPerStep = range > 0 ? Math.max(3, Math.min(8, 100 / range)) : 5;

  const drag = useDragControl({ value, min, max, defaultValue, onChange, pxPerStep });
  const angle = valueToAngle(value, min, max);

  return (
    <div className={css.container}>
      {label && <span className={css.label}>{label}</span>}
      <svg
        className={css.knob}
        viewBox="0 0 44 44"
        tabIndex={0}
        role="slider"
        aria-valuenow={value}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-label={label ?? "knob"}
        onPointerDown={drag.onPointerDown}
        onPointerMove={drag.onPointerMove}
        onPointerUp={drag.onPointerUp}
        onDoubleClick={drag.onDoubleClick}
        onKeyDown={drag.onKeyDown}
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
