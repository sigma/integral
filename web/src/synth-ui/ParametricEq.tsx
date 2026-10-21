import { useCallback, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { ParameterFamily } from "../../pkg/integral_wasm.js";
import { EqKnob } from "../EqKnob";
import box from "../PanelBox.module.css";
import { zoomOf } from "./grid";
import css from "./ParametricEq.module.css";

/** One EQ knob, bound to a parameter. Discrete ranges show one tick per value. */
export interface EqControl {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  onChange: (value: number) => void;
  formatValue: (value: number) => string;
}

/** One EQ band: gain and frequency, plus Q for peaking bands. */
export interface EqBand {
  label: string;
  /** Parameter family colouring the band's knobs; neutral when absent. */
  family?: ParameterFamily;
  gain: EqControl;
  freq: EqControl;
  q?: EqControl;
}

interface Props {
  /** Bands from highest to lowest (2–4). */
  bands: EqBand[];
  style?: CSSProperties;
}

/** A grid row: one knob, or an empty spacer. Row `i` sits in column `i % 2`. */
interface Slot {
  band: number;
  label?: string;
  control?: EqControl;
}

/**
 * Knobs zig-zag down two columns, one per row: frequency (and Q) on the left,
 * gain on the right, so each band starts in whichever column the previous one
 * ended. A middle band without Q gets a spacer, which keeps a cell free for
 * its label between the two dividers.
 */
function layout(bands: EqBand[]): Slot[] {
  const slots: Slot[] = [];
  bands.forEach((b, band) => {
    const freq = { band, label: "Freq", control: b.freq };
    const gain = { band, label: "Gain", control: b.gain };
    slots.push(...(slots.length % 2 === 0 ? [freq, gain] : [gain, freq]));
    if (b.q) slots.push({ band, label: "Q", control: b.q });
    else if (band > 0 && band < bands.length - 1) slots.push({ band });
  });
  return slots;
}

/**
 * Rows of the dividers below each band but the last, per column: one past the
 * band's last knob in that column, i.e. halfway to the next band's first.
 */
function dividerRows(slots: Slot[], bandCount: number): number[][] {
  return Array.from({ length: bandCount - 1 }, (_, b) =>
    [0, 1].map((col) => slots.reduce((last, s, i) => (s.band === b && i % 2 === col ? i : last), 0) + 1),
  );
}

/** Measured slot centres, relative to the knob area. */
interface Geometry {
  w: number;
  h: number;
  xs: number[];
  ys: number[];
}

export function ParametricEq({ bands, style }: Props) {
  const slots = layout(bands);
  const areaRef = useRef<HTMLDivElement>(null);
  const slotRefs = useRef<(HTMLDivElement | null)[]>([]);
  slotRefs.current.length = slots.length;
  const [geo, setGeo] = useState<Geometry | null>(null);

  const measure = useCallback(() => {
    const area = areaRef.current;
    if (!area) return;
    const w = area.offsetWidth;
    const h = area.offsetHeight;
    if (w === 0 || h === 0) return;
    const a = area.getBoundingClientRect();
    // Rects are on screen; `w`, `h` and the plot are layout px (a page's zoom).
    const zoom = zoomOf(area);
    const rects = slotRefs.current.map((el) => el?.getBoundingClientRect());
    setGeo({
      w,
      h,
      xs: rects.map((r) => (r ? (r.left + r.width / 2 - a.left) / zoom : 0)),
      ys: rects.map((r) => (r ? (r.top + r.height / 2 - a.top) / zoom : 0)),
    });
  }, []);

  useLayoutEffect(() => {
    requestAnimationFrame(measure);
    const area = areaRef.current;
    if (!area) return;
    const observer = new ResizeObserver(measure);
    observer.observe(area);
    return () => observer.disconnect();
  }, [measure]);

  const dividers = dividerRows(slots, bands.length);
  // A band's label takes the first empty cell (beside one of its knobs) that
  // no divider crosses.
  const crossed = (row: number, col: number) => dividers.some((d) => d[col] === row);
  const labelCells = bands.map((_, b) => {
    const row = slots.findIndex((s, i) => s.band === b && !crossed(i, (i + 1) % 2));
    return row < 0 ? null : { row, col: (row + 1) % 2 };
  });

  return (
    <div className={box.box} style={style}>

      <div className={css.knobArea} ref={areaRef}>
        {geo && geo.ys.length === slots.length && (
          <svg className={css.dividers} viewBox={`0 0 ${geo.w} ${geo.h}`}>
            {dividers.map(([l = 0, r = 0], i) => {
              const ly = geo.ys[l];
              const ry = geo.ys[r];
              return (
                <path key={i} d={`M 0 ${ly} L ${geo.xs[0]} ${ly} L ${geo.xs[1]} ${ry} L ${geo.w} ${ry}`}
                  fill="none" stroke="#3a3a5a" strokeWidth="2" />
              );
            })}
            {labelCells.map((cell, b) => cell && (
              <text key={b} x={geo.xs[cell.col]} y={geo.ys[cell.row]! + 6} textAnchor="middle"
                fill="#555" fontSize="18" fontWeight="900" fontFamily="sans-serif">
                {bands[b]!.label}
              </text>
            ))}
          </svg>
        )}

        <div className={css.knobs}>
          {slots.map((s, i) => (
            <div
              key={i}
              ref={(el) => { slotRefs.current[i] = el; }}
              style={{ gridColumn: (i % 2) + 1, gridRow: i + 1 }}
            >
              {s.control && <EqKnob label={s.label} family={bands[s.band]!.family} {...s.control} />}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
