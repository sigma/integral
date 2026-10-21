/**
 * The Chorus and Reverb Sections of the mixer's FX strips. Every type is
 * generated: one Control per 2-Cell row, zig-zagging down two columns like
 * the mixer EQ, so FX row N stays level with EQ row N. The footprint holds
 * the type with the most parameters, so changing the type never moves the
 * strip's type list below it.
 */

import { FOOTPRINT } from "../../synth-ui/grid";
import { fxFields, isTwoState, type FxBlock } from "../fields";
import type { ControlLayout, SectionBody, SectionFrame, VariantSet } from "../types";
import { fxVariantSet } from "./set";

/** Rows from one Control to the next: the old 48px knob pitch. */
const PITCH = 2;
/**
 * Controls the footprint holds: a layout choice, sized for today's largest
 * type. It is not read from core, so the footprint stays fixed; the validator
 * fails if a type needs more, and a test if none needs this many.
 */
export const STRIP_CAPACITY = 8;
/** Each zig-zag column is one small knob wide. */
const KNOB = FOOTPRINT.knobSm;

/** The strip's frame: two knob columns, tall enough for `STRIP_CAPACITY` zig-zagged Controls. */
function frame(title: string): SectionFrame {
  return { title, titleEdge: "top", w: 2 * KNOB.w, h: PITCH * (STRIP_CAPACITY - 1) + KNOB.h };
}

/** A type's Controls in core's order, zig-zagged: a switch for a two-state parameter, else a knob. */
function zigZag(block: FxBlock): (type: number) => SectionBody<string> {
  return (type) => {
    const fields = fxFields(block, type);
    const items = fields.ids.map(
      (field, i): ControlLayout<string> => ({
        kind: isTwoState(fields, field) ? "switch" : "knob",
        field,
        x: (i % 2) * KNOB.w,
        y: i * PITCH,
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
  zigZag("chorus"),
);

/** The Reverb Section, keyed by Reverb type. */
export const REVERB_SET: VariantSet<number, string> = fxVariantSet(
  "reverb",
  frame("Reverb"),
  new Map(),
  zigZag("reverb"),
);
