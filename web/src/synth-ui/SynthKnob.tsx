import { useDragControl } from "../useDragControl";
import css from "./SynthKnob.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  label: string;
  onChange: (value: number) => void;
  formatValue?: (v: number) => string;
  /** Accent color for the LED arc. Default: "#fc8" (orange). */
  color?: string;
  /** "sm" = 44px, "lg" = 64px. Default: "sm". */
  size?: "sm" | "lg";
  /** Full name for tooltip. Defaults to label. */
  title?: string;
}

const ANGLE_MIN = -135;
const ANGLE_MAX = 135;
const LED_COUNT = 21;

function valueToAngle(value: number, min: number, max: number): number {
  const norm = (value - min) / (max - min || 1);
  return ANGLE_MIN + norm * (ANGLE_MAX - ANGLE_MIN);
}

export function SynthKnob({
  value,
  min,
  max,
  defaultValue,
  label,
  onChange,
  formatValue,
  color = "#fc8",
  size = "sm",
  title,
}: Props) {
  const svgSize = size === "lg" ? 72 : 52;
  const cx = svgSize / 2;
  const cy = svgSize / 2;
  const knobR = size === "lg" ? 24 : 17;
  const ledR = size === "lg" ? 32 : 23;
  const range = max - min || 1;

  const angle = valueToAngle(value, min, max);
  const norm = (value - min) / range;

  const drag = useDragControl({ value, min, max, defaultValue, onChange });

  // LED dots
  const leds = [];
  for (let i = 0; i < LED_COUNT; i++) {
    const ledNorm = i / (LED_COUNT - 1);
    const ledAngle = ANGLE_MIN + ledNorm * (ANGLE_MAX - ANGLE_MIN);
    const rad = (ledAngle - 90) * (Math.PI / 180);
    const lx = cx + ledR * Math.cos(rad);
    const ly = cy + ledR * Math.sin(rad);
    const isLit = ledNorm <= norm;
    leds.push(
      <circle
        key={i}
        cx={lx}
        cy={ly}
        r={size === "lg" ? 2.2 : 1.5}
        fill={isLit ? color : "#2a2a3a"}
      />,
    );
  }

  // Indicator line
  const indicatorR = knobR - 3;
  const rad = (angle - 90) * (Math.PI / 180);
  const ix = cx + indicatorR * Math.cos(rad);
  const iy = cy + indicatorR * Math.sin(rad);

  const display = formatValue ? formatValue(value) : String(value);

  return (
    <div className={`${css.knob} ${size === "lg" ? css.lg : ""}`} title={title ?? label}>
      <span className={css.label}>{label}</span>
      <svg
        className={css.svg}
        viewBox={`0 0 ${svgSize} ${svgSize}`}
        width={svgSize}
        height={svgSize}
        tabIndex={0}
        role="slider"
        aria-valuenow={value}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-label={label}
        onPointerDown={drag.onPointerDown}
        onPointerMove={drag.onPointerMove}
        onPointerUp={drag.onPointerUp}
        onDoubleClick={drag.onDoubleClick}
        onKeyDown={drag.onKeyDown}
      >
        {/* LED arc */}
        {leds}
        {/* Knob body */}
        <defs>
          <radialGradient id={`kg-${size}`} cx="40%" cy="35%">
            <stop offset="0%" stopColor="#777" />
            <stop offset="100%" stopColor="#333" />
          </radialGradient>
        </defs>
        <circle cx={cx} cy={cy} r={knobR} fill={`url(#kg-${size})`} />
        <circle cx={cx} cy={cy} r={knobR} fill="none" stroke="#555" strokeWidth="1" />
        {/* Indicator line */}
        <line
          x1={cx}
          y1={cy}
          x2={ix}
          y2={iy}
          stroke={color}
          strokeWidth={size === "lg" ? 2.5 : 2}
          strokeLinecap="round"
        />
      </svg>
      <span className={css.value}>{display}</span>
    </div>
  );
}
