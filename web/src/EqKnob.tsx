import { useId, type ReactNode } from "react";
import type { ParameterFamily } from "../pkg/integral_wasm.js";
import { accentColour } from "./synth-ui/family";
import { useDragControl } from "./useDragControl";
import css from "./EqKnob.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  onChange: (value: number) => void;
  formatValue: (value: number) => string;
  /** Cap and selected-tick colour, when `family` is absent. Default: "#888". */
  color?: string;
  /** Parameter family: colours the cap and the selected tick. */
  family?: ParameterFamily;
  label?: string;
  /** Extra control below the value, out of the layout flow (e.g. a mode toggle). */
  addon?: ReactNode;
}

/** Ranges with at most this many values get one tick per value; wider ones read as continuous. */
export const MAX_TICKS = 24;

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
  family,
  label,
  addon,
}: Props) {
  const accent = accentColour(family, color);
  const gradientId = useId();
  const range = max - min;
  const pxPerStep = range > 0 ? Math.max(3, Math.min(8, 100 / range)) : 5;

  const drag = useDragControl({ value, min, max, defaultValue, onChange, pxPerStep });
  const angle = valueToAngle(value, min, max);
  const ticks = range > 0 && range < MAX_TICKS ? Array.from({ length: range + 1 }, (_, i) => min + i) : [];

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
          <radialGradient id={gradientId} cx="40%" cy="35%">
            <stop offset="0%" style={{ stopColor: accent }} />
            <stop offset="100%" stopColor="#222" />
          </radialGradient>
        </defs>
        {ticks.map((v) => (
          <line
            key={v}
            x1="22"
            y1="1"
            x2="22"
            y2="5"
            style={{ stroke: v === value ? accent : "#556" }}
            strokeWidth={v === value ? 1.5 : 1}
            transform={`rotate(${valueToAngle(v, min, max)} 22 22)`}
          />
        ))}
        <circle cx="22" cy="22" r="16" fill={`url(#${gradientId})`} />
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
      {addon && <div className={css.addon}>{addon}</div>}
    </div>
  );
}
