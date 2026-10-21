import { CELL, FOOTPRINT, NAME, cellsPx, lineStyle, optionListRows } from "./grid";
import css from "./SynthSwitch.module.css";

interface Props {
  value: number;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
  label?: string;
  /** Show a colored LED dot next to the active option. */
  led?: boolean;
  /** LED color when lit. Default: "#4dff4d". */
  ledColor?: string;
  /** Render as horizontal toggle (2 states) or vertical list. */
  vertical?: boolean;
  /** Full name for tooltip. Defaults to label. */
  title?: string;
  /**
   * Fill a Grid footprint exactly, at scale 1: a toggle is 2×1, name included;
   * a vertical list is 2×N with one Cell row per option, plus a row for its name.
   * Off: today's natural size, for CSS layouts.
   */
  grid?: boolean;
  /**
   * N for a vertical list: draw it on the Grid (implies `grid`) as 2×N, when the
   * layout gives it more rows than it needs. Never fewer than it needs: a short
   * N would clip options, so the list keeps its `optionListRows`.
   */
  gridRows?: number;
}

/** Margin around a toggle's button on the Grid, px. */
const TOGGLE_MARGIN = 2;

/** A toggle's button on the Grid: the Cell, less the name's line or a margin each side. */
function toggleButtonStyle(named: boolean): { width: string; height: number } {
  return {
    width: `calc(100% - ${2 * TOGGLE_MARGIN}px)`,
    height: named ? CELL - NAME.line : CELL - 2 * TOGGLE_MARGIN,
  };
}

export function SynthSwitch({
  value,
  options,
  onChange,
  label,
  led = true,
  ledColor = "#4dff4d",
  vertical = false,
  title,
  grid: gridProp = false,
  gridRows,
}: Props) {
  const grid = gridProp || gridRows !== undefined;
  const named = Boolean(label);
  const gridClass = grid ? css.grid : "";
  if (options.length === 2 && !vertical) {
    // Two-state toggle button
    const isOn = value === options[1]!.value;
    return (
      <div
        className={`${css.toggle} ${gridClass}`}
        style={grid ? cellsPx(FOOTPRINT.switch) : undefined}
        title={title ?? label}
      >
        {named && (
          <span className={css.label} style={grid ? lineStyle(NAME) : undefined}>
            {label}
          </span>
        )}
        <button
          className={`${css.toggleBtn} ${isOn ? css.toggleOn : ""}`}
          style={grid ? toggleButtonStyle(named) : undefined}
          onClick={() => onChange(isOn ? options[0]!.value : options[1]!.value)}
        >
          {led && <span className={css.led} style={isOn ? { background: ledColor } : undefined} />}
          <span>{isOn ? options[1]!.label : options[0]!.label}</span>
        </button>
      </div>
    );
  }

  // Multi-state vertical list
  const listRows = Math.max(gridRows ?? 0, optionListRows(options.length, named));
  const row = { height: CELL };
  return (
    <div
      className={`${css.multiSwitch} ${gridClass}`}
      style={
        grid
          ? cellsPx({ w: FOOTPRINT.optionListW, h: listRows })
          : undefined
      }
      title={title ?? label}
    >
      {named && (
        <span
          className={css.label}
          style={grid ? { ...lineStyle(NAME), lineHeight: `${CELL}px`, height: CELL } : undefined}
        >
          {label}
        </span>
      )}
      <div
        className={css.optionList}
        style={grid ? { height: options.length * CELL } : undefined}
        role="radiogroup"
        aria-label={title ?? label}
      >
        {options.map((opt) => {
          const isActive = opt.value === value;
          return (
            <button
              key={opt.value}
              role="radio"
              aria-checked={isActive}
              className={`${css.option} ${isActive ? css.optionActive : ""}`}
              style={grid ? row : undefined}
              onClick={() => onChange(opt.value)}
            >
              {led && (
                <span
                  className={css.led}
                  style={isActive ? { background: ledColor } : undefined}
                />
              )}
              <span>{opt.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
