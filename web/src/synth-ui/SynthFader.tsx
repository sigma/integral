import { useCallback, useRef, type ReactNode } from "react";
import { ADDON_LINE, FOOTPRINT, NAME, READOUT, cellsPx, lineStyle, type Px } from "./grid";
import { ControlAddon } from "./ControlAddon";
import { faderScale } from "./faderScale";
import css from "./SynthFader.module.css";

interface Props {
  value: number;
  min: number;
  max: number;
  defaultValue: number;
  label: string;
  onChange: (value: number) => void;
  formatValue?: (v: number) => string;
  /** Compact fader track height (~70px) vs standard (~100px). */
  compact?: boolean;
  /** Custom track height in pixels. Overrides compact. */
  trackHeight?: number;
  /** Hide the label zone (used when an envelope curve replaces it). */
  hideLabel?: boolean;
  /** Full name for tooltip. Defaults to label. */
  title?: string;
  /** Shown but not editable (a value the device fixes). */
  readOnly?: boolean;
  /**
   * N: fill a 2×N Grid footprint exactly, at scale 1, name and readout
   * included; the track takes what is left. Overrides compact and trackHeight.
   * Absent: today's natural size, for CSS layouts.
   */
  gridRows?: number;
  /**
   * On the Grid, the px box to fill instead of the whole footprint: the
   * footprint less the edge padding a Section's renderer gives it.
   */
  box?: Px;
  /**
   * On the Grid, draw the level scale (`faderScale`) beside the track in
   * place of its ticks: the track moves to the box's left, the marks and
   * their labels take the rest. The 0 dB mark of a range other than 0–127
   * is `defaultValue`.
   */
  scale?: boolean;
  /**
   * A line under the readout, such as the tempo-sync toggle. On the Grid the
   * track gives up an `ADDON_LINE` for it, so the footprint holds.
   */
  addon?: ReactNode;
}

const TRACK_HEIGHT = 100;
const TRACK_HEIGHT_COMPACT = 70;
const TICK_COUNT = 11;

export const FADER_LABEL_HEIGHT = 28; // px — fixed height for the label zone
/** Thumb height, border included, px. On the Grid the track is inset by half of it, so the thumb stays in the footprint. */
const THUMB_H = 10;

export function SynthFader({
  value,
  min,
  max,
  defaultValue,
  label,
  onChange,
  formatValue,
  compact,
  trackHeight,
  hideLabel,
  title,
  readOnly,
  gridRows,
  box: boxProp,
  scale = false,
  addon,
}: Props) {
  const grid = gridRows !== undefined;
  const box = grid ? (boxProp ?? cellsPx({ w: FOOTPRINT.faderW, h: gridRows })) : undefined;
  // On the Grid the slider area is what the name and readout leave; the track runs
  // half a thumb inside it at each end, so the thumb never leaves the footprint.
  const nameH = hideLabel ? 0 : grid ? NAME.line : FADER_LABEL_HEIGHT;
  const areaH = box
    ? box.height - nameH - READOUT.line - (addon === undefined ? 0 : ADDON_LINE)
    : (trackHeight ?? (compact ? TRACK_HEIGHT_COMPACT : TRACK_HEIGHT));
  const inset = grid ? THUMB_H / 2 : 0;
  const trackH = Math.max(0, areaH - 2 * inset);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startVal = useRef(0);

  const range = max - min || 1;
  const norm = (value - min) / range;
  const defaultNorm = (defaultValue - min) / range;
  const isDefault = value === defaultValue;

  const fillBottom = Math.min(norm, defaultNorm) * trackH;
  const fillTop = Math.max(norm, defaultNorm) * trackH;
  const fillHeight = fillTop - fillBottom;
  const thumbY = inset + (1 - norm) * trackH;

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      startY.current = e.clientY;
      startVal.current = value;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [value],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const dy = startY.current - e.clientY;
      const delta = (dy / trackH) * range;
      onChange(Math.round(Math.max(min, Math.min(max, startVal.current + delta))));
    },
    [min, max, range, trackH, onChange],
  );

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [defaultValue, onChange]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowUp" || e.key === "ArrowRight") {
      e.preventDefault();
      onChange(Math.min(max, value + (e.shiftKey ? 10 : 1)));
    } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
      e.preventDefault();
      onChange(Math.max(min, value - (e.shiftKey ? 10 : 1)));
    } else if (e.key === "Home") {
      e.preventDefault();
      onChange(max);
    } else if (e.key === "End") {
      e.preventDefault();
      onChange(min);
    }
  };

  const ticks = [];
  for (let i = 0; i < (scale ? 0 : TICK_COUNT); i++) {
    const y = (i / (TICK_COUNT - 1)) * trackH;
    const isMajor = i === 0 || i === TICK_COUNT - 1 || i === Math.floor(TICK_COUNT / 2);
    ticks.push(
      <div key={i} className={`${css.tick} ${isMajor ? css.tickMajor : ""}`} style={{ top: y }} />,
    );
  }

  const display = formatValue ? formatValue(value) : String(value);
  // The marks sit level with the thumb's centre at their value.
  const marks = scale
    ? faderScale({ min, max, format: formatValue, nominal: defaultValue }).map((mark) => (
        <div
          key={mark.value}
          className={`${css.mark} ${mark.zero ? css.markZero : ""}`}
          style={{ top: inset + (1 - (mark.value - min) / range) * trackH }}
        >
          <span className={css.markLabel}>{mark.label}</span>
        </div>
      ))
    : null;

  const trackArea = (
    <div
      className={css.trackArea}
      style={{ height: areaH }}
      tabIndex={0}
      role="slider"
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-label={label}
      aria-orientation="vertical"
      aria-readonly={readOnly}
      {...(readOnly
        ? {}
        : {
            onPointerDown: handlePointerDown,
            onPointerMove: handlePointerMove,
            onPointerUp: handlePointerUp,
            onDoubleClick: handleDoubleClick,
            onKeyDown: handleKeyDown,
          })}
    >
      <div className={css.ruler} style={grid ? { top: inset, bottom: inset } : undefined}>
        {ticks}
      </div>
      <div className={css.track} style={grid ? { top: inset, bottom: inset } : undefined}>
        {!isDefault && (
          <div className={css.fill} style={{ bottom: fillBottom, height: fillHeight }} />
        )}
      </div>
      <div
        className={`${css.thumb} ${!isDefault ? css.thumbActive : ""}`}
        style={{ top: thumbY, height: THUMB_H, marginTop: -THUMB_H / 2 }}
      />
    </div>
  );

  return (
    <div
      className={[
        css.fader,
        compact ? css.compact : "",
        readOnly ? css.readOnly : "",
        grid ? css.grid : "",
      ].join(" ")}
      style={box}
      title={title ?? label}
    >
      {/* Label zone — fixed height so faders align across groups */}
      {!hideLabel && (
        <div className={css.labelZone} style={{ height: nameH }}>
          <span className={css.label} style={grid ? lineStyle(NAME) : undefined}>
            {label}
          </span>
        </div>
      )}
      {/* Fader zone, and the scale beside it */}
      {marks ? (
        <div className={css.scaled} style={{ height: areaH }}>
          {trackArea}
          <div className={css.scale}>{marks}</div>
        </div>
      ) : (
        trackArea
      )}
      <span className={css.value} style={grid ? lineStyle(READOUT) : undefined}>
        {display}
      </span>
      {addon !== undefined && <ControlAddon grid={grid}>{addon}</ControlAddon>}
    </div>
  );
}
