/**
 * Geometry shared by the renderer and the validator: footprints, Section
 * outlines and reading order. Control footprints come from `FOOTPRINT`
 * (`synth-ui/grid.ts`), the one table; Display minimums are below.
 */

import { DisplayKind } from "../../pkg/integral_wasm.js";
import { BLEED, FOOTPRINT, faderCells, type Cells, type Edge } from "../synth-ui/grid";
import type { FieldKey } from "./fields";
import type {
  CellRect,
  ControlLayout,
  DisplayLayout,
  ElementLayout,
  ItemLayout,
  Scope,
  SectionFrame,
  SectionLayout,
  SectionPlacement,
  VariantSet,
} from "./types";

/** A Section's border, drawn this far inside its own Cells, px at zoom 1 (#329). */
export const INSET = 4;

/**
 * The border's line, px: its corner radius, the width of its sides and
 * bottom, and of its top (the family accent). The renderer draws it and the
 * Silkscreen clip and edge padding follow it.
 */
export const BORDER = { radius: 6, width: 1, accent: 3 } as const;

/** Px from a Section's outer edge to the inside of its border line on `edge`. */
export function borderClearance(edge: Edge): number {
  return INSET + (edge === "top" ? BORDER.accent : BORDER.width);
}

/** Px an element is drawn in from each edge of its Cells. */
export type EdgePadding = Record<Edge, number>;

/**
 * How far in from each edge of its Cells `el` is drawn in `layout` (#392).
 * The border runs through the Cells along a Section's edges, `INSET` inside
 * them; a title row (`top`) or column (`side`) stands between it and the
 * content on that edge, the other edges have none. An element whose drawn
 * edge (`BLEED`) lies on a border Cell is drawn clear of the border's line
 * on that edge, so it loses a few px there, never a Cell.
 */
export function edgePadding<F extends FieldKey>(layout: SectionFrame, el: ElementLayout<F>): EdgePadding {
  const r = itemRect(el);
  const onBorder: Record<Edge, boolean> = {
    top: r.y === 0 && layout.titleEdge !== "top",
    left: r.x === 0 && layout.titleEdge !== "side",
    bottom: r.y + r.h === layout.h,
    right: r.x + r.w === layout.w,
  };
  const bleeds = (el.kind === "fader" && el.scale ? BLEED.levelFader : BLEED[el.kind]) as readonly Edge[];
  const pad = (e: Edge) => (onBorder[e] && bleeds.includes(e) ? borderClearance(e) : 0);
  return { top: pad("top"), right: pad("right"), bottom: pad("bottom"), left: pad("left") };
}

/** A Control's footprint in Cells. */
export function controlCells<F extends FieldKey>(control: ControlLayout<F>): Cells {
  switch (control.kind) {
    case "knob":
      return control.size === "lg" ? FOOTPRINT.knobLg : FOOTPRINT.knobSm;
    case "switch":
      return FOOTPRINT.switch;
    case "power":
      return FOOTPRINT.power;
    case "fader":
      return faderCells(control.h, control.scale);
    case "list":
      return { w: control.w ?? FOOTPRINT.optionListW, h: control.h };
    case "picker":
      return { w: control.w ?? FOOTPRINT.picker.w, h: FOOTPRINT.picker.h };
  }
}

/** The Cells an item covers, in its Section's content coordinates. */
export function itemRect<F extends FieldKey>(item: ItemLayout<F>): CellRect {
  if (item.kind === "display" || item.kind === "alternatives") {
    return { x: item.x, y: item.y, w: item.w, h: item.h };
  }
  return { x: item.x, y: item.y, ...controlCells(item) };
}

/**
 * Px each of two Displays side by side is drawn in from the edge they share
 * (#415), so a separator shows in the gap between them. A Display with no
 * Display beside it fills its Cells.
 */
export const DISPLAY_INSET = 3;

/** Two drawn Displays side by side: `left`'s right edge is `right`'s left edge, and their rows overlap. */
export interface DisplayPair {
  /** The Display on the left. */
  left: DisplayLayout;
  /** The Display on the right. */
  right: DisplayLayout;
}

/** Every pair of Displays in `elements` (the ones a Section draws) that share a vertical edge. */
export function adjacentDisplays<F extends FieldKey>(elements: readonly ElementLayout<F>[]): DisplayPair[] {
  const displays = elements.filter((el): el is DisplayLayout => el.kind === "display");
  return displays.flatMap((left) =>
    displays
      .filter((right) => left.x + left.w === right.x && left.y < right.y + right.h && right.y < left.y + left.h)
      .map((right) => ({ left, right })),
  );
}

/**
 * Minimum footprint of each Display kind, in Cells. A Display fills whatever
 * rectangle the layout gives it, never less than this.
 *
 * | Display | Min | Why |
 * |---|---|---|
 * | Pitch, Filter, Amp, Comp envelope | 4×2 | four segments over 96px, curve readable at 48px high |
 * | OSC, Filter and LFO scopes | 3×2 | one period over 72px; the old knob-cell scope |
 * | PCM structure diagrams | 3×2 | two partial boxes and the join, 72×48 |
 * | Comp transfer curve | 3×3 | a square plot; the old 88×60 curve, squared |
 * | Surround pad | 8×8 | a square room a finger can drag in, 192px |
 * | Wave picker | 6×2 | one Wave row in 144px, its name truncated; the picker fills its Cells up to its 220px |
 * | Strip name | 1×4 | one line of the old mixer's 15px name, run up 96px of a fader's side |
 */
export const DISPLAY_MIN_FOOTPRINT = {
  [DisplayKind.PitchEnvelope]: { w: 4, h: 2 },
  [DisplayKind.FilterEnvelope]: { w: 4, h: 2 },
  [DisplayKind.AmpEnvelope]: { w: 4, h: 2 },
  [DisplayKind.CompEnvelope]: { w: 4, h: 2 },
  [DisplayKind.OscScope]: { w: 3, h: 2 },
  [DisplayKind.FilterScope]: { w: 3, h: 2 },
  [DisplayKind.LfoScope]: { w: 3, h: 2 },
  [DisplayKind.ModLfoScope]: { w: 3, h: 2 },
  [DisplayKind.Lfo1Scope]: { w: 3, h: 2 },
  [DisplayKind.Lfo2Scope]: { w: 3, h: 2 },
  [DisplayKind.StepLfoScope]: { w: 3, h: 2 },
  [DisplayKind.Structure12]: { w: 3, h: 2 },
  [DisplayKind.Structure34]: { w: 3, h: 2 },
  [DisplayKind.CompTransferCurve]: { w: 3, h: 3 },
  [DisplayKind.SurroundPad]: { w: 8, h: 8 },
  [DisplayKind.WavePicker]: { w: 6, h: 2 },
  [DisplayKind.StripName]: { w: 1, h: 4 },
} as const satisfies Record<DisplayKind, Cells>;

/** Title Cells a Section adds above (`top`) or beside (`side`) its content. */
export function titleCells(frame: SectionFrame): { top: number; side: number } {
  return { top: frame.titleEdge === "top" ? 1 : 0, side: frame.titleEdge === "side" ? 1 : 0 };
}

/** The whole Section's footprint, title included: what a screen places. */
export function sectionCells(frame: SectionFrame): Cells {
  const { top, side } = titleCells(frame);
  return { w: frame.w + side, h: frame.h + top };
}

/** A row of Sections by placement id, left to right. */
export type SectionRow = readonly { id: string; section: SectionFrame }[];

/** `row`'s Sections abutting from column 0, their tops on `y`. */
export function placeRow(row: SectionRow, y: number): SectionPlacement[] {
  let x = 0;
  return row.map(({ id, section }) => {
    const placed = { id, section, x, y };
    x += sectionCells(section).w;
    return placed;
  });
}

/**
 * The Cell an arrow leaves or enters a side `n` Cells long at, from the
 * side's start (`routing.ts`): the middle one, the first of the two on an
 * even length.
 */
export const sideMiddle = (n: number): number => Math.floor((n - 1) / 2);

/**
 * `row`'s Sections left to right from column `x`, `gap` Cells apart, each
 * centred on Cell row `axis`: the middle of each one's left and right sides
 * lies on it, so arrows between neighbours run straight.
 */
export function placeLine(row: SectionRow, x: number, axis: number, gap: number): SectionPlacement[] {
  return row.map(({ id, section }) => {
    const { w, h } = sectionCells(section);
    const placed = { id, section, x, y: axis - sideMiddle(h) };
    x += w + gap;
    return placed;
  });
}

/** The Cells `row` covers: its Sections' widths summed, and the tallest. */
export function rowCells(row: SectionRow): Cells {
  return {
    w: row.reduce((w, { section }) => w + sectionCells(section).w, 0),
    h: Math.max(0, ...row.map(({ section }) => sectionCells(section).h)),
  };
}

/** `frame`'s title with each `{name}` replaced by the Scope's 1-based `name` index. */
export function sectionTitle(frame: SectionFrame, scope: Scope = {}): string {
  const t = frame.title;
  let out = "";
  let i = 0;
  while (i < t.length) {
    const open = t.indexOf("{", i);
    const close = open < 0 ? -1 : t.indexOf("}", open);
    if (close < 0) {
      out += t.slice(i);
      break;
    }
    const name = t.slice(open + 1, close);
    const index = scope[name];
    out += t.slice(i, open) + (index === undefined ? t.slice(open, close + 1) : String(index + 1));
    i = close + 1;
  }
  return out;
}

/** The layout a variant set draws for `key`: its hand layout, else the generated one. */
export function variantLayout<K extends FieldKey, F extends FieldKey>(
  set: VariantSet<K, F>,
  key: K,
): SectionLayout<F> {
  const body = set.variants.get(key) ?? set.generate(key);
  return {
    title: set.title,
    titleEdge: set.titleEdge,
    w: set.w,
    h: set.h,
    fields: set.fieldsOf(key),
    ...body,
  };
}

/** Compares rectangles in reading order: by top edge, then left edge. */
export function readingOrder(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return a.y - b.y || a.x - b.x;
}

/** Every element of `items`, every choice of each `alternatives` group included. */
export function allElements<F extends FieldKey>(items: readonly ItemLayout<F>[]): ElementLayout<F>[] {
  return items.flatMap((item) => (item.kind === "alternatives" ? item.options.flatMap((o) => o.items) : [item]));
}

/**
 * The elements a Section draws, in focus order (row-major): each
 * `alternatives` group contributes the alternative `pick` names, or its first.
 */
export function drawnElements<F extends FieldKey>(
  items: readonly ItemLayout<F>[],
  pick: (group: string) => string | undefined,
): ElementLayout<F>[] {
  const out: ElementLayout<F>[] = [];
  for (const item of items) {
    if (item.kind !== "alternatives") {
      out.push(item);
      continue;
    }
    const name = pick(item.group);
    const chosen = item.options.find((o) => o.name === name) ?? item.options[0];
    out.push(...(chosen?.items ?? []));
  }
  return out.sort(readingOrder);
}

/**
 * Looks up the placement ids of a screen's repeated rows (a synth's partials,
 * a drum key's WMTs): `idOf(row, section)` names Section `section` of row
 * `row` (0-based), for each of `rows()` rows. Built on first call, as the row
 * count comes from core. The lookup returns the row and Section an id names,
 * or `undefined` for another Section.
 */
export function rowSectionLookup<S extends string>(
  rows: () => number,
  sections: readonly S[],
  idOf: (row: number, section: S) => string,
): (id: string) => { row: number; section: S } | undefined {
  let ids: Map<string, { row: number; section: S }> | undefined;
  return (id) => {
    ids ??= new Map(
      Array.from({ length: rows() }, (_, row) =>
        sections.map((section) => [idOf(row, section), { row, section }] as const),
      ).flat(),
    );
    return ids.get(id);
  };
}
