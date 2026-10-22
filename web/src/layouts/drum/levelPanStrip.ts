/**
 * A drum kit key's LEVEL / PAN Section, as a mixer part strip
 * (`mixer/strip.ts`): Pan on top, the FX1 and FX2 sends side by side under
 * it, then the level fader, scale and all, down to the strip's last row.
 * One arrangement for every drum kit; each kit names its own fields.
 */

import { FOOTPRINT } from "../../synth-ui/grid";
import { ENV_CELLS } from "../envelope";
import type { FieldKey, FieldSpace } from "../fields";
import type { SectionLayout } from "../types";

/** The rows of one of the strip's knobs; Pan, then the sends under it. */
const KNOB_H = FOOTPRINT.knobSm.h;

/** The rows above the fader: Pan, then the sends. */
const KNOBS_H = 2 * KNOB_H;

/** What a {@link levelPanStrip} binds. */
export interface LevelPanStripSpec<F extends FieldKey> {
  /** The pan knob's field. */
  pan: F;
  /** The FX1 (Chorus) send's field. */
  fx1: F;
  /** The FX2 (Reverb) send's field. */
  fx2: F;
  /** The level fader's field. */
  level: F;
  /** The fields the four are of, from core. */
  fields: FieldSpace<F>;
  /** Content height in Cells, at least {@link LEVEL_PAN_MIN_H}: the key row it ends. */
  h: number;
}

/**
 * The shortest strip, in Cells: PCM Drum's, as tall as its key row of
 * envelopes, the shortest whose fader still reads as a mixer's. Every kit's
 * key row is at least this tall.
 */
export const LEVEL_PAN_MIN_H = ENV_CELLS.h;

/** The LEVEL / PAN strip for `spec`: 4 Cells wide, its fader filling what the knobs leave. */
export function levelPanStrip<F extends FieldKey>({
  pan,
  fx1,
  fx2,
  level,
  fields,
  h,
}: LevelPanStripSpec<F>): SectionLayout<F> {
  return {
    title: "LEVEL / PAN",
    titleEdge: "top",
    w: 4,
    h,
    fields,
    items: [
      { kind: "knob", field: pan, x: 1, y: 0 },
      { kind: "knob", field: fx1, x: 0, y: KNOB_H, label: "FX1" },
      { kind: "knob", field: fx2, x: 2, y: KNOB_H, label: "FX2" },
      { kind: "fader", field: level, x: 1, y: KNOBS_H, h: h - KNOBS_H, label: false, scale: true },
    ],
  };
}
