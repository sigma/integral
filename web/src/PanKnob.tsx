import { useDragControl } from "./useDragControl";
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
  const drag = useDragControl({ value, min: 0, max: 127, defaultValue: 64, onChange });
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
        onPointerDown={drag.onPointerDown}
        onPointerMove={drag.onPointerMove}
        onPointerUp={drag.onPointerUp}
        onDoubleClick={drag.onDoubleClick}
        onKeyDown={drag.onKeyDown}
      >
        <circle cx="22" cy="22" r="19" fill="none" stroke="#3a3a5a" strokeWidth="1" />
        <defs>
          <radialGradient id="knobGrad" cx="40%" cy="35%">
            <stop offset="0%" stopColor="#888" />
            <stop offset="100%" stopColor="#333" />
          </radialGradient>
        </defs>
        <circle cx="22" cy="22" r="16" fill="url(#knobGrad)" />
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
