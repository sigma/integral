/**
 * A switch strip: the narrow Section heading a row of boxes (a synth's
 * partial, a drum's WMT layer), its title down its side, reading bottom to
 * top, and its on/off a 1×1 power toggle at the top of the one column beside
 * it. Two Cells wide, as narrow as a Section holding a title and a Control
 * can be.
 */

import type { FieldKey, FieldSpace } from "./fields";
import type { SectionLayout } from "./types";

/** What a {@link switchStrip} names and binds. */
export interface SwitchStripSpec<F extends FieldKey> {
  /** The title template, filled from the Scope: `"Partial {partial}"`. */
  title: string;
  /** The two-state field the toggle writes; the caller's `bind` resolves it for the Scope. */
  field: F;
  /** The fields `field` is one of, from core. */
  fields: FieldSpace<F>;
  /** Content height in Cells: the row it heads. */
  h: number;
}

/** The switch strip for `spec`. */
export function switchStrip<F extends FieldKey>({ title, field, fields, h }: SwitchStripSpec<F>): SectionLayout<F> {
  return {
    title,
    titleEdge: "side",
    w: 1,
    h,
    fields,
    // The title already names the toggle.
    items: [{ kind: "power", field, x: 0, y: 0, label: false }],
  };
}
