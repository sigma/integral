/**
 * Draws one Section from its layout, on the Grid at zoom 1. Controls fill
 * their footprints at scale 1 (their `grid` variants); nothing is scaled to
 * fit. Elements go into the DOM in row-major order, so focus runs row by row
 * whatever order the layout lists them in.
 */

import type { CSSProperties, ReactNode } from "react";
import { Relevance, type DisplayKind, type ParameterFamily } from "../../pkg/integral_wasm.js";
import type { PickOption } from "../SearchPopover";
import { SyncUnit } from "../SyncUnit";
import { PowerButton, SynthFader, SynthKnob, SynthPicker, SynthSwitch, familyVar } from "../synth-ui";
import { knobDialSide } from "../synth-ui/SynthKnob";
import { BOTTOM_TO_TOP, CELL, cellsPx, type Px } from "../synth-ui/grid";
import type { FieldKey } from "./fields";
import {
  BORDER,
  DISPLAY_INSET,
  INSET,
  adjacentDisplays,
  drawnElements,
  edgePadding,
  itemRect,
  sectionCells,
  sectionTitle,
  titleCells,
} from "./geometry";
import type { ControlLayout, ElementLayout, Scope, SectionLayout, SectionSilk } from "./types";
import { drawnArrows, routeSectionArrow } from "./routing";
import { SilkArrows } from "./SilkArrows";
import css from "./GridSection.module.css";

/** One option of a Control that picks from a list: its raw value, label and (for a picker) family. */
export type BindingOption = PickOption;

/**
 * A tempo-syncable parameter's free/note toggle, from core's sync info. The
 * Control carries it; the layout knows nothing of it. Knobs and faders draw
 * it as their readout's unit: `formatNumber`'s text, then the mode's label.
 */
export interface SyncBinding {
  /** Toggle labels from core, `[free, note]`: the readout's unit in each mode. */
  labels: readonly string[];
  /** Readout of a raw value without its unit, from core. */
  formatNumber: (value: number) => string;
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
  /** Raw value a level fader's scale marks 0 dB, from core's spec; none without it. */
  zeroDb?: number;
  /** Readout of a raw value; default its number. */
  format?: (value: number) => string;
  /** Switch, list and picker options; default every value from `min` to `max`, formatted. */
  options?: readonly BindingOption[];
  /** Writes a new raw value. */
  onChange: (value: number) => void;
  /** Tempo sync, when the parameter has it: the Control draws its toggle as the readout's unit. */
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
  /**
   * Core's relevance of a field (element `index`, if named); default always
   * active. A switched-off Control dims and stays editable; an irrelevant one
   * dims, inert, or hides, as its layout says.
   */
  relevance?: (field: F, index?: number) => Relevance;
  /** The alternative to draw for a group, by name; default (or unknown) the first. */
  alternative?: (group: string) => string | undefined;
  /** Relevance of a Display's fields; default always active. An inactive Display dims, as an inactive Control does. */
  displayRelevance?: (display: DisplayKind) => Relevance;
  /** Draws a Display filling `size` px; default an empty rectangle. */
  renderDisplay?: (display: DisplayKind, size: { width: number; height: number }) => ReactNode;
  /** Accents the border's top edge. */
  family?: ParameterFamily;
  /** Extra style for the Section's root (e.g. hiding it). */
  style?: CSSProperties;
}

/** The border's widths as CSS: the accent on top, the line elsewhere. */
const BORDER_WIDTHS = `${BORDER.accent}px ${BORDER.width}px ${BORDER.width}px`;

const always = (): Relevance => Relevance.Active;

/**
 * Relevance of a Control whose state may not be loaded yet: active once
 * `loaded`, irrelevant (inert) until then, with nothing to edit.
 */
export function loadedRelevance(loaded: boolean): Relevance {
  return loaded ? Relevance.Active : Relevance.Irrelevant;
}
const none = (): undefined => undefined;

/** `b`'s options, or every value in its range, formatted. */
function optionsOf(b: Binding): BindingOption[] {
  if (b.options) return [...b.options];
  const out: BindingOption[] = [];
  for (let v = b.min; v <= b.max; v++) out.push({ value: v, label: b.format?.(v) ?? String(v) });
  return out;
}

/** The Control for a layout entry and its binding, filling `box` (its footprint, less any edge padding). */
function Control<F extends FieldKey>({
  control: c,
  binding: b,
  box,
  dialSide,
}: {
  control: ControlLayout<F>;
  binding: Binding;
  box: Px;
  /** The dial side of each size of knob in the Section. */
  dialSide: Record<"sm" | "lg", number>;
}) {
  const label = c.label === false ? undefined : (c.label ?? b.name);
  // A synced Control's readout is the number and, after it, its unit as the toggle.
  const format = b.sync?.formatNumber ?? b.format;
  const unit = b.sync && <SyncUnit labels={b.sync.labels} noteMode={b.sync.noteMode} onChange={b.sync.onChange} />;
  switch (c.kind) {
    case "knob":
      return (
        <SynthKnob
          grid
          box={box}
          dialSide={dialSide[c.size ?? "sm"]}
          size={c.size ?? "sm"}
          label={label ?? ""}
          title={b.name}
          value={b.value}
          min={b.min}
          max={b.max}
          defaultValue={b.defaultValue}
          center={b.center}
          family={b.family}
          formatValue={format}
          onChange={b.onChange}
          unit={unit}
        />
      );
    case "fader":
      return (
        <SynthFader
          gridRows={c.h}
          box={box}
          scale={c.scale}
          zeroDb={b.zeroDb}
          label={label ?? b.name}
          hideLabel={label === undefined}
          title={b.name}
          value={b.value}
          min={b.min}
          max={b.max}
          defaultValue={b.defaultValue}
          formatValue={format}
          onChange={b.onChange}
          unit={unit}
        />
      );
    case "switch":
      return (
        <SynthSwitch grid box={box} label={label} title={b.name} value={b.value} options={optionsOf(b)} onChange={b.onChange} />
      );
    case "power": {
      const { isOn, toggle } = twoState(b);
      // Named by its field, as a switch is; `aria-pressed` says whether it's on.
      return <PowerButton isOn={isOn} onToggle={toggle} title={b.name} className={css.powerControl} />;
    }
    case "list":
      return (
        <SynthSwitch
          vertical
          gridRows={c.h}
          box={box}
          label={label}
          title={b.name}
          value={b.value}
          options={optionsOf(b)}
          onChange={b.onChange}
        />
      );
    case "picker":
      return <SynthPicker box={box} label={label} title={b.name} value={b.value} options={optionsOf(b)} onChange={b.onChange} />;
  }
}

/** A two-state field as a power toggle: on at its maximum, and a toggle writes the other state. */
function twoState(b: Binding): { isOn: boolean; toggle: () => void } {
  const isOn = b.value === b.max;
  return { isOn, toggle: () => b.onChange(isOn ? b.min : b.max) };
}

/**
 * The title's power toggle for a two-state field (`twoState`). It never dims.
 * The wrapper carries `data-field` as a body Control's does, so a caller
 * finds it the same way.
 */
function TitleSwitch({ field, binding: b, title }: { field: FieldKey; binding: Binding; title: string }) {
  const { isOn, toggle } = twoState(b);
  return (
    <span className={css.titleSwitch} data-field={String(field)} data-title-switch="">
      <PowerButton isOn={isOn} onToggle={toggle} title={`Switch ${title} ${isOn ? "off" : "on"}`} className={css.power} />
    </span>
  );
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
  relevance = always,
  alternative = none,
  displayRelevance = always,
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
    borderWidth: BORDER_WIDTHS,
    ...(family === undefined ? {} : { "--section-accent": familyVar(family) }),
  } as CSSProperties;
  const title = sectionTitle(layout, scope);
  const elements = drawnElements(layout.items, alternative);
  // Displays side by side each give up DISPLAY_INSET on the edge they share; a separator fills the gap.
  const pairs = adjacentDisplays(elements);
  // `el` has a Display on its left when it is a pair's right one, and the other way round.
  const sharedEdge = (el: ElementLayout<F>, side: "left" | "right") =>
    pairs.some((p) => (side === "left" ? p.right : p.left) === el) ? DISPLAY_INSET : 0;
  // Each element's box: its Cells, less the edge padding that keeps it clear of the border.
  const drawn = elements.map((el) => {
    const rect = itemRect(el);
    const edge = edgePadding(layout, el);
    const pad = { ...edge, left: edge.left + sharedEdge(el, "left"), right: edge.right + sharedEdge(el, "right") };
    const full = cellsPx({ w: rect.w, h: rect.h });
    const size: Px = { width: full.width - pad.left - pad.right, height: full.height - pad.top - pad.bottom };
    const box = { left: ox + rect.x * CELL + pad.left, top: oy + rect.y * CELL + pad.top, ...size };
    const binding = el.kind === "display" ? undefined : bind(el.field, el.index);
    return { el, size, box, binding };
  });
  // Every knob of a size is drawn at the smallest dial any of them fits.
  const dialSide = { sm: Infinity, lg: Infinity };
  for (const { el, size } of drawn) {
    if (el.kind !== "knob") continue;
    const k = el.size ?? "sm";
    dialSide[k] = Math.min(dialSide[k], knobDialSide(k, size));
  }
  // A separator down the middle of each gap between Displays, where both are drawn.
  const boxOfElement = new Map(drawn.map((d) => [d.el, d.box]));
  const separators = pairs.map(({ left, right }) => {
    const [a, b] = [boxOfElement.get(left)!, boxOfElement.get(right)!];
    const top = Math.max(a.top, b.top);
    return { left: ox + right.x * CELL, top, height: Math.min(a.top + a.height, b.top + b.height) - top };
  });
  // An arrow is drawn when both its ends are.
  const arrows = drawnArrows(
    (layout.silkscreen ?? []).filter((s) => s.kind === "arrow"),
    (a) => routeSectionArrow(layout, a, elements),
  );

  return (
    <div className={css.section} data-grid-section={title} style={{ width: W, height: H, ...style }}>
      <div className={css.border} style={border} />
      <div
        className={`${css.title} ${top ? css.topTitle : ""}`}
        style={{
          left: side ? 0 : INSET,
          top: INSET,
          width: side ? CELL : W - 2 * INSET,
          height: top ? CELL - INSET : H - 2 * INSET,
          // A top title's transparent border lets the Section's accent and sides show through.
          ...(top ? { borderWidth: BORDER_WIDTHS } : {}),
          // A side title reads bottom to top, as the mixer's strip names do.
          ...(side ? BOTTOM_TO_TOP : {}),
        }}
      >
        <span className={css.titleText}>
          {title}
          {layout.titleSwitch !== undefined && (
            <TitleSwitch field={layout.titleSwitch} binding={bind(layout.titleSwitch)} title={title} />
          )}
        </span>
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
      {separators.map(({ left, top, height }) => (
        <div
          key={`separator ${left},${top}`}
          className={css.displaySeparator}
          data-display-separator=""
          style={{ left, top, height }}
        />
      ))}
      {drawn.map(({ el, box, size, binding }) => {
        if (el.kind === "display") {
          const rel = displayRelevance(el.display);
          const dim = rel !== Relevance.Active;
          return (
            <div
              key={elementKey(el)}
              className={`${css.element} ${dim ? css.dim : ""}`}
              data-display={el.display}
              data-inactive={dim ? "dim" : undefined}
              // Switched off, it stays editable, as on the hardware.
              inert={rel === Relevance.Irrelevant}
              style={box}
            >
              {renderDisplay?.(el.display, size)}
            </div>
          );
        }
        const rel = relevance(el.field, el.index);
        // Switched off, it dims whatever the layout says, so it can still be edited.
        const inactive =
          rel === Relevance.Active ? undefined : rel === Relevance.SwitchedOff ? "dim" : (el.inactive ?? "dim");
        return (
          <div
            key={elementKey(el)}
            className={`${css.element} ${inactive ? css[inactive] : ""}`}
            data-field={String(el.field)}
            data-index={el.index}
            data-inactive={inactive}
            inert={rel === Relevance.Irrelevant}
            style={box}
          >
            <Control control={el} binding={binding!} box={size} dialSide={dialSide} />
          </div>
        );
      })}
    </div>
  );
}
