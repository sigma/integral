/**
 * The Chorus and Reverb parameter Sections, at the top of the mixer's FX
 * columns where the other strips have their EQ, and as tall (#392). Every
 * type is generated, staggered as the EQ is (#405): one Control per 2-Cell
 * row, in core's order, zig-zagging down two columns from the EQ's first
 * knob's Cell, so every FX knob is level with, and in the column of, an EQ
 * knob, and every knob gets the same footprint and the same dial. The
 * footprint holds the type with the most parameters, so changing the type
 * never moves anything.
 */

import { FOOTPRINT } from "../../synth-ui/grid";
import { EQ_LAYOUT } from "../mixer/eq";
import { fxFields, isTwoState, type FxBlock } from "../fields";
import type { ControlLayout, SectionBody, SectionFrame, VariantSet } from "../types";
import { fxVariantSet } from "./set";

/** Rows from one Control to the next, as in the EQ: each knob overlaps the one before it by a row. */
const PITCH = 2;
/**
 * Controls the footprint holds: a layout choice, sized for today's largest
 * type. It is not read from core, so the footprint stays fixed; the validator
 * fails if a type needs more, and a test if none needs this many.
 */
export const STRIP_CAPACITY = 8;
/** Each column is one small knob wide. */
const KNOB = FOOTPRINT.knobSm;

/** The EQ's first knob: the stagger starts on its Cell. */
const FIRST_KNOB = (() => {
  const knob = EQ_LAYOUT.items.find((i) => i.kind === "knob");
  if (!knob) throw new Error("the mixer EQ has no knob");
  return { x: knob.x, y: knob.y };
})();

/** The rows the staggered Controls reach: the last one starts a pitch below the one before it. */
export const STRIP_ROWS = FIRST_KNOB.y + (STRIP_CAPACITY - 1) * PITCH + KNOB.h;

/** The Section's frame: two knob columns, as tall as the mixer EQ beside it. */
function frame(title: string): SectionFrame {
  return { title, titleEdge: "top", w: 2 * KNOB.w, h: EQ_LAYOUT.h };
}

/** A type's Controls in core's order, zig-zagging down two columns: a switch for a two-state parameter, else a knob. */
function rows(block: FxBlock): (type: number) => SectionBody<string> {
  return (type) => {
    const fields = fxFields(block, type);
    const items = fields.ids.map(
      (field, i): ControlLayout<string> => ({
        kind: isTwoState(fields, field) ? "switch" : "knob",
        field,
        x: FIRST_KNOB.x + (i % 2) * KNOB.w,
        y: FIRST_KNOB.y + i * PITCH,
      }),
    );
    return { items };
  };
}

/** The Chorus Section, keyed by Chorus type. */
export const CHORUS_SET: VariantSet<number, string> = fxVariantSet(
  "chorus",
  frame("Chorus"),
  new Map(),
  rows("chorus"),
);

/** The Reverb Section, keyed by Reverb type. */
export const REVERB_SET: VariantSet<number, string> = fxVariantSet(
  "reverb",
  frame("Reverb"),
  new Map(),
  rows("reverb"),
);
