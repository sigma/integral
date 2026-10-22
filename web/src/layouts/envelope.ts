/**
 * A PCM breakpoint envelope on the Grid, shared by the PCM Synth and PCM
 * Drum editors: a row of knobs, the curve, then levels L0–L4 on even
 * columns over times T1–T4 between them, so each time sits under the two
 * levels it joins. The faders come from the Display's core binding, one per
 * element of its time and level array fields.
 */

import { DisplayProp, type DisplayKind, type ToneType, type WasmDisplaySlot } from "../../pkg/integral_wasm.js";
import { displaySlots } from "./displays";
import { controlCells } from "./geometry";
import type { FieldKey } from "./fields";
import type { ControlLayout, ItemLayout } from "./types";

/** Rows of an envelope block, from its top: its knobs, its curve, its level faders and its time faders. */
const ENV_ROWS = { knobs: 0, curve: 3, levels: 5, times: 9 } as const;

/** An envelope block's footprint in Cells: five fader columns, and its four bands. */
export const ENV_CELLS = { w: 10, h: 13 } as const;

/** Height of an envelope's faders, in Cells. */
const ENV_FADER_H = 4;

/** What sets an envelope block apart: its Display, where core binds it, and the knobs over its curve. */
export interface EnvelopeSpec<F extends FieldKey> {
  /** The curve, whose core binding names the faders. */
  display: DisplayKind;
  /** The editor whose binding it is. */
  toneType: ToneType;
  /** The field of the editor's block a slot reads (one of the slot's getters). */
  fieldOf: (slot: WasmDisplaySlot) => F | undefined;
  /** The knobs over the curve, left to right; five at most. */
  knobs: readonly { field: F; label: string }[];
}

/**
 * One fader per stored value of `prop` in the Display's core binding: the
 * n-th value (0-based) on column `2n + x`, named `name` and its number
 * counted from `first`. Values the device fixes (a TVA's L0 and L4) get no
 * fader; only the curve draws them.
 */
function envelopeFaders<F extends FieldKey>(
  spec: EnvelopeSpec<F>,
  prop: DisplayProp,
  name: string,
  first: number,
  x: number,
  y: number,
): ControlLayout<F>[] {
  return displaySlots(spec.display, spec.toneType)
    .filter((s) => s.prop === prop)
    .flatMap((slot, n) => {
      const field = spec.fieldOf(slot);
      if (field === undefined) return [];
      const label = `${name}${n + first}`;
      return [{ kind: "fader", field, index: slot.index, x: 2 * n + x, y, h: ENV_FADER_H, label }];
    });
}

/**
 * Where an envelope block's curve sits across it: over exactly the columns of
 * its faders (`faders`, #415), or a Cell in from each side of the block
 * (`block`), clear of a border the block abuts.
 */
export type CurveSpan = "faders" | "block";

/**
 * The items of an envelope block at `(x, y)` in its Section, {@link ENV_CELLS}
 * in all, its curve placed by `curve`. Reads core's Display binding, so call
 * it on first use, not at import.
 */
export function envelopeItems<F extends FieldKey>(
  spec: EnvelopeSpec<F>,
  x: number,
  y: number,
  curve: CurveSpan = "block",
): ItemLayout<F>[] {
  const faders = [
    ...envelopeFaders(spec, DisplayProp.Levels, "L", 0, x, y + ENV_ROWS.levels),
    ...envelopeFaders(spec, DisplayProp.Times, "T", 1, x + 1, y + ENV_ROWS.times),
  ];
  const [from, to] =
    curve === "faders"
      ? [Math.min(...faders.map((f) => f.x)), Math.max(...faders.map((f) => f.x + controlCells(f).w))]
      : [x + 1, x + ENV_CELLS.w - 1];
  return [
    ...spec.knobs.map(
      (k, i): ControlLayout<F> => ({ kind: "knob", field: k.field, x: x + 2 * i, y: y + ENV_ROWS.knobs, label: k.label }),
    ),
    { kind: "display", display: spec.display, x: from, y: y + ENV_ROWS.curve, w: to - from, h: 2 },
    ...faders,
  ];
}
