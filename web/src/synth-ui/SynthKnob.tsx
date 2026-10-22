import type { ReactNode } from "react";
import { useDragControl } from "../useDragControl";
import { ParameterFamily } from "../../pkg/integral_wasm.js";
import { familyVar } from "./family";
import { FOOTPRINT, NAME, READOUT, cellsPx, lineStyle, type Cells, type Px } from "./grid";
import css from "./SynthKnob.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  label: string;
  onChange: (value: number) => void;
  formatValue?: (v: number) => string;
  /** Parameter family, from core: colours the lit LEDs only; pointer, label and value stay neutral. */
  family: ParameterFamily;
  /** "sm" = 52px dial, "lg" = 72px dial. Default: "sm". */
  size?: "sm" | "lg";
  /**
   * Fill the size's Grid footprint exactly, at scale 1: small 2×3, large 3×4
   * Cells, name and readout included. Off: today's natural size, for CSS layouts.
   */
  grid?: boolean;
  /**
   * On the Grid, the px box to fill instead of the whole footprint: the
   * footprint less the edge padding a Section's renderer gives it.
   */
  box?: Px;
  /**
   * On the Grid, the largest dial side, px: a Section draws each size of
   * knob at one side, the smallest any of them fits (`knobDialSide`).
   */
  dialSide?: number;
  /** Full name for tooltip. Defaults to label. */
  title?: string;
  /** Value the LED arc grows from, for relative controls. Default: 0 for signed ranges, else min. */
  center?: number;
  /**
   * The readout's unit, drawn after `formatValue`'s text on its line: a
   * tempo-syncable knob's free/note toggle.
   */
  unit?: ReactNode;
}

/** Ranges with at most this many values get one LED per value; wider ones read as continuous. */
const MAX_TICKS = 24;

const ANGLE_MIN = -135;
const ANGLE_MAX = 135;
/** LEDs on the ring of a wide-range knob; discrete ranges get one per value. */
const LED_COUNT = 21;

/** Dial drawing: SVG side, cap radius, LED ring radius, LED dot radius, pointer width. */
interface Dial {
  svg: number;
  knobR: number;
  ledR: number;
  dotR: number;
  pointer: number;
}

/** Each size's natural dial and its Grid footprint. */
const SIZES: Record<"sm" | "lg", { dial: Dial; footprint: Cells }> = {
  sm: { dial: { svg: 52, knobR: 17, ledR: 23, dotR: 1.5, pointer: 2 }, footprint: FOOTPRINT.knobSm },
  lg: { dial: { svg: 72, knobR: 24, ledR: 32, dotR: 2.2, pointer: 2.5 }, footprint: FOOTPRINT.knobLg },
};

/**
 * The side of a `size` knob's dial on the Grid in `box`: as large as the
 * width and the height left by name and readout allow, never larger than natural.
 */
export function knobDialSide(size: "sm" | "lg", box: Px): number {
  return Math.min(SIZES[size].dial.svg, box.width, box.height - NAME.line - READOUT.line);
}

/**
 * `dial` redrawn at side `svg`. Radii follow the side, so a footprint
 * changed in the table resizes the dial with it.
 */
function scaledDial(dial: Dial, svg: number): Dial {
  const k = svg / dial.svg;
  return { svg, knobR: dial.knobR * k, ledR: dial.ledR * k, dotR: dial.dotR * k, pointer: dial.pointer };
}

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
  size = "sm",
  title,
  center,
  family,
  grid = false,
  box: boxProp,
  dialSide,
  unit,
}: Props) {
  const lit = familyVar(family);
  const { footprint, dial: natural } = SIZES[size];
  const box = boxProp ?? cellsPx(footprint);
  const fits = knobDialSide(size, box);
  const dial = grid ? scaledDial(natural, Math.min(fits, dialSide ?? fits)) : natural;
  const { svg: svgSize, knobR, ledR } = dial;
  const cx = svgSize / 2;
  const cy = svgSize / 2;
  const range = max - min || 1;
  const ledCount = max > min && max - min + 1 <= MAX_TICKS ? max - min + 1 : LED_COUNT;

  const angle = valueToAngle(value, min, max);
  const norm = (value - min) / range;

  const drag = useDragControl({ value, min, max, defaultValue, onChange });

  // LED dots: lit from the center LED (nearest to `center`) to the value
  const from = Math.round((((center ?? (min < 0 && max > 0 ? 0 : min)) - min) / range) * (ledCount - 1));
  const to = norm * (ledCount - 1);
  const leds = [];
  for (let i = 0; i < ledCount; i++) {
    const ledNorm = i / (ledCount - 1);
    const ledAngle = ANGLE_MIN + ledNorm * (ANGLE_MAX - ANGLE_MIN);
    const rad = (ledAngle - 90) * (Math.PI / 180);
    const lx = cx + ledR * Math.cos(rad);
    const ly = cy + ledR * Math.sin(rad);
    const isLit = i >= Math.min(from, to) && i <= Math.max(from, to);
    leds.push(
      <circle
        key={i}
        cx={lx}
        cy={ly}
        r={dial.dotR}
        style={{ fill: isLit ? lit : "#2a2a3a" }}
      />,
    );
  }

  // Indicator line
  const indicatorR = knobR - 3;
  const rad = (angle - 90) * (Math.PI / 180);
  const ix = cx + indicatorR * Math.cos(rad);
  const iy = cy + indicatorR * Math.sin(rad);

  const display = formatValue ? formatValue(value) : String(value);

  const classes = [css.knob, size === "lg" ? css.lg : "", grid ? css.grid : ""].join(" ");

  return (
    <div className={classes} style={grid ? box : undefined} title={title ?? label}>
      <span className={css.label} style={grid ? lineStyle(NAME) : undefined}>
        {label}
      </span>
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
          className={css.pointer}
          strokeWidth={dial.pointer}
          strokeLinecap="round"
        />
      </svg>
      <span className={css.value} style={grid ? lineStyle(READOUT) : undefined}>
        {display}
        {unit !== undefined && <> {unit}</>}
      </span>
    </div>
  );
}
