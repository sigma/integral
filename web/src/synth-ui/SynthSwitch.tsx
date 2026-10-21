import { CELL, FOOTPRINT, NAME, cellsPx, lineStyle, optionListRows, type Px } from "./grid";
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
  /**
   * On the Grid, the px box to fill instead of the whole footprint: the
   * footprint less the edge padding a Section's renderer gives it.
   */
  box?: Px;
}

/** Margin around a toggle's button on the Grid, px. */
const TOGGLE_MARGIN = 2;

/** A toggle's button on the Grid: its box `height`, less the name's line or a margin each side. */
function toggleButtonStyle(named: boolean, height: number): { width: string; height: number } {
  return {
    width: `calc(100% - ${2 * TOGGLE_MARGIN}px)`,
    height: named ? height - NAME.line : height - 2 * TOGGLE_MARGIN,
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
  box,
}: Props) {
  const grid = gridProp || gridRows !== undefined;
  const named = Boolean(label);
  const gridClass = grid ? css.grid : "";
  if (options.length === 2 && !vertical) {
    // Two-state toggle button
    const isOn = value === options[1]!.value;
    const toggleBox = box ?? cellsPx(FOOTPRINT.switch);
    return (
      <div
        className={`${css.toggle} ${gridClass}`}
        style={grid ? toggleBox : undefined}
        title={title ?? label}
      >
        {named && (
          <span className={css.label} style={grid ? lineStyle(NAME) : undefined}>
            {label}
          </span>
        )}
        <button
          className={`${css.toggleBtn} ${isOn ? css.toggleOn : ""}`}
          style={grid ? toggleButtonStyle(named, toggleBox.height) : undefined}
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
  const listBox = box ?? cellsPx({ w: FOOTPRINT.optionListW, h: listRows });
  // One Cell per row and for the name. A box cut short (edge padding) takes the
  // px from the name's row first, down to its text line, then from the rows.
  const nameH = named ? Math.min(CELL, Math.max(NAME.line, listBox.height - options.length * CELL)) : 0;
  const row = { height: Math.min(CELL, (listBox.height - nameH) / options.length) };
  return (
    <div
      className={`${css.multiSwitch} ${gridClass}`}
      style={grid ? listBox : undefined}
      title={title ?? label}
    >
      {named && (
        <span
          className={css.label}
          style={grid ? { ...lineStyle(NAME), lineHeight: `${nameH}px`, height: nameH } : undefined}
        >
          {label}
        </span>
      )}
      <div
        className={css.optionList}
        style={grid ? { height: options.length * row.height } : undefined}
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
