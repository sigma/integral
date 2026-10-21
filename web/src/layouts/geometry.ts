/**
 * Geometry shared by the renderer and the validator: footprints, Section
 * outlines and reading order. Control footprints come from `FOOTPRINT`
 * (`synth-ui/grid.ts`), the one table; Display minimums are below.
 */

import { DisplayKind } from "../../pkg/integral_wasm.js";
import { FOOTPRINT, type Cells } from "../synth-ui/grid";
import type { FieldKey } from "./fields";
import type {
  CellRect,
  ControlLayout,
  ElementLayout,
  ItemLayout,
  Scope,
  SectionFrame,
  SectionLayout,
  VariantSet,
} from "./types";

/** A Section's border, drawn this far inside its own Cells, px at zoom 1 (#329). */
export const INSET = 4;

/** A Control's footprint in Cells. */
export function controlCells<F extends FieldKey>(control: ControlLayout<F>): Cells {
  switch (control.kind) {
    case "knob":
      return control.size === "lg" ? FOOTPRINT.knobLg : FOOTPRINT.knobSm;
    case "switch":
      return FOOTPRINT.switch;
    case "fader":
      return { w: FOOTPRINT.faderW, h: control.h };
    case "list":
      return { w: FOOTPRINT.optionListW, h: control.h };
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
