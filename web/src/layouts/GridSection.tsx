/**
 * Draws one Section from its layout, on the Grid at zoom 1. Controls fill
 * their footprints at scale 1 (their `grid` variants); nothing is scaled to
 * fit. Elements go into the DOM in row-major order, so focus runs row by row
 * whatever order the layout lists them in.
 */

import type { CSSProperties, ReactNode } from "react";
import type { DisplayKind, ParameterFamily } from "../../pkg/integral_wasm.js";
import { SyncToggle } from "../SyncToggle";
import { SynthFader, SynthKnob, SynthSwitch, familyVar } from "../synth-ui";
import { CELL, cellsPx, type Cells } from "../synth-ui/grid";
import type { FieldKey } from "./fields";
import { INSET, drawnElements, itemRect, sectionCells, sectionTitle, titleCells } from "./geometry";
import type { ControlLayout, ElementLayout, Scope, SectionLayout, SectionSilk } from "./types";
import { drawnArrows, routeSectionArrow } from "./routing";
import { SilkArrows } from "./SilkArrows";
import css from "./GridSection.module.css";

/** One option of a Control that picks from a list. */
export interface BindingOption {
  /** Raw value. */
  value: number;
  /** What the Control shows for it. */
  label: string;
}

/**
 * A tempo-syncable parameter's free/note toggle, from core's sync info. The
 * Control carries it; the layout knows nothing of it. Knobs and faders draw it.
 */
export interface SyncBinding {
  /** Toggle labels from core, `[free, note]`. */
  labels: readonly string[];
  /** Whether the Control edits the note value now. */
  noteMode: boolean;
  /** Switches between the free value and the note. */
  onChange: (noteMode: boolean) => void;
}

/** What a Control needs from its parameter, resolved by the caller for its Scope. */
export interface Binding {
  /** Core's name for the field; shown unless the layout overrides or hides it. */
  name: string;
  /** Current raw value. */
  value: number;
  /** Minimum raw value. */
  min: number;
  /** Maximum raw value. */
  max: number;
  /** Default raw value. */
  defaultValue: number;
  /** Colours the Control's LEDs. */
  family?: ParameterFamily;
  /** Raw value a knob's arc grows from. */
  center?: number;
  /** Readout of a raw value; default its number. */
  format?: (value: number) => string;
  /** Switch and list options; default every value from `min` to `max`, formatted. */
  options?: readonly BindingOption[];
  /** Writes a new raw value. */
  onChange: (value: number) => void;
  /** Tempo sync, when the parameter has it: the Control draws its toggle. */
  sync?: SyncBinding;
}

/** Props of {@link GridSection}. */
export interface GridSectionProps<F extends FieldKey> {
  /** What to draw. */
  layout: SectionLayout<F>;
  /**
   * Resolves a field for this instance's Scope: element `index` of it when the
   * Control names one (an array field), reading and writing that element only.
   */
  bind: (field: F, index?: number) => Binding;
  /** This instance's indices; fills the title template. */
  scope?: Scope;
  /** Whether core counts a field (element `index`, if named) active; default always. Inactive Controls dim or hide. */
  isActive?: (field: F, index?: number) => boolean;
  /** The alternative to draw for a group, by name; default (or unknown) the first. */
  alternative?: (group: string) => string | undefined;
  /** Draws a Display filling `size` px; default an empty rectangle. */
  renderDisplay?: (display: DisplayKind, size: { width: number; height: number }) => ReactNode;
  /** Accents the border's top edge. */
  family?: ParameterFamily;
  /** Extra style for the Section's root (e.g. hiding it). */
  style?: CSSProperties;
}

/** The border's corner radius and side width, px; the Silkscreen clip follows both. */
const BORDER = { radius: 6, width: 1 } as const;

const always = (): true => true;
const none = (): undefined => undefined;

/** `b`'s options, or every value in its range, formatted. */
function optionsOf(b: Binding): BindingOption[] {
  if (b.options) return [...b.options];
  const out: BindingOption[] = [];
  for (let v = b.min; v <= b.max; v++) out.push({ value: v, label: b.format?.(v) ?? String(v) });
  return out;
}

/** The Control for a layout entry and its binding, at its footprint. */
function Control<F extends FieldKey>({ control: c, binding: b }: { control: ControlLayout<F>; binding: Binding }) {
  const label = c.label === false ? undefined : (c.label ?? b.name);
  const addon = b.sync && <SyncToggle labels={b.sync.labels} noteMode={b.sync.noteMode} onChange={b.sync.onChange} />;
  switch (c.kind) {
    case "knob":
      return (
        <SynthKnob
          grid
          size={c.size ?? "sm"}
          label={label ?? ""}
          title={b.name}
          value={b.value}
          min={b.min}
          max={b.max}
          defaultValue={b.defaultValue}
          center={b.center}
          family={b.family}
          formatValue={b.format}
          onChange={b.onChange}
          addon={addon}
        />
      );
    case "fader":
      return (
        <SynthFader
          gridRows={c.h}
          label={label ?? b.name}
          hideLabel={label === undefined}
          title={b.name}
          value={b.value}
          min={b.min}
          max={b.max}
          defaultValue={b.defaultValue}
          formatValue={b.format}
          onChange={b.onChange}
          addon={addon}
        />
      );
    case "switch":
      return (
        <SynthSwitch grid label={label} title={b.name} value={b.value} options={optionsOf(b)} onChange={b.onChange} />
      );
    case "list":
      return (
        <SynthSwitch
          vertical
          gridRows={c.h}
          label={label}
          title={b.name}
          value={b.value}
          options={optionsOf(b)}
          onChange={b.onChange}
        />
      );
  }
}

/** A rule or group name, in px from the Section's origin; arrows are drawn by `SilkArrows`. */
function Silk<F extends FieldKey>({ silk: s, ox, oy }: { silk: SectionSilk<F>; ox: number; oy: number }) {
  if (s.kind === "rule") {
    const points = s.points.map(([x, y]) => `${ox + x * CELL},${oy + y * CELL}`).join(" ");
    return <polyline className={css.rule} points={points} />;
  }
  if (s.kind === "label") {
    return (
      <text className={s.style === "title" ? css.groupTitle : css.groupName} x={ox + s.x * CELL} y={oy + s.y * CELL}>
        {s.text}
      </text>
    );
  }
  return null;
}

/** A stable React key for an element. */
function elementKey<F extends FieldKey>(el: ElementLayout<F>): string {
  const what = el.kind === "display" ? `d${el.display}` : `f${String(el.field)}${el.index === undefined ? "" : `[${el.index}]`}`;
  return `${what}@${el.x},${el.y}`;
}

/** One Section, drawn from its layout. */
export function GridSection<F extends FieldKey>({
  layout,
  bind,
  scope,
  isActive = always,
  alternative = none,
  renderDisplay,
  family,
  style,
}: GridSectionProps<F>) {
  const { top, side } = titleCells(layout);
  const { width: W, height: H } = cellsPx(sectionCells(layout));
  const ox = side * CELL;
  const oy = top * CELL;
  // The border's top takes the family accent through a custom property the CSS reads.
  const border = {
    inset: INSET,
    borderRadius: BORDER.radius,
    // Sides and bottom only: the CSS draws the thicker accent top.
    borderLeftWidth: BORDER.width,
    borderRightWidth: BORDER.width,
    borderBottomWidth: BORDER.width,
    ...(family === undefined ? {} : { "--section-accent": familyVar(family) }),
  } as CSSProperties;
  const title = sectionTitle(layout, scope);
  const elements = drawnElements(layout.items, alternative);
  // An arrow is drawn when both its ends are.
  const arrows = drawnArrows(
    (layout.silkscreen ?? []).filter((s) => s.kind === "arrow"),
    (a) => routeSectionArrow(layout, a, elements),
  );

  return (
    <div className={css.section} data-grid-section={title} style={{ width: W, height: H, ...style }}>
      <div className={css.border} style={border} />
      <div
        className={`${css.title} ${side ? css.sideTitle : top ? css.topTitle : ""}`}
        style={{
          left: side ? 0 : INSET,
          top: INSET,
          width: side ? CELL : W - 2 * INSET,
          height: top ? CELL - INSET : H - 2 * INSET,
        }}
      >
        {title}
      </div>
      {/* Silkscreen stops at the drawn border, which sits INSET inside the Section's Cells. */}
      <svg
        className={css.silkscreen}
        width={W}
        height={H}
        style={{ clipPath: `inset(${INSET + BORDER.width}px round ${BORDER.radius}px)` }}
      >
        {(layout.silkscreen ?? []).map((s, i) => (
          <Silk key={i} silk={s} ox={ox} oy={oy} />
        ))}
        <SilkArrows arrows={arrows} ox={ox} oy={oy} />
      </svg>
      {elements.map((el) => {
        const rect = itemRect(el);
        const cells: Cells = { w: rect.w, h: rect.h };
        const box = { left: ox + rect.x * CELL, top: oy + rect.y * CELL, ...cellsPx(cells) };
        if (el.kind === "display") {
          return (
            <div key={elementKey(el)} className={css.element} data-display={el.display} style={box}>
              {renderDisplay?.(el.display, cellsPx(cells))}
            </div>
          );
        }
        const inactive = isActive(el.field, el.index) ? undefined : (el.inactive ?? "dim");
        return (
          <div
            key={elementKey(el)}
            className={`${css.element} ${inactive ? css[inactive] : ""}`}
            data-field={String(el.field)}
            data-index={el.index}
            data-inactive={inactive}
            inert={inactive !== undefined}
            style={box}
          >
            <Control control={el} binding={bind(el.field, el.index)} />
          </div>
        );
      })}
    </div>
  );
}
