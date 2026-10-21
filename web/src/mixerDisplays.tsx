/**
 * The mixer's Displays, drawn from the fields core binds them to (#352): a
 * layout names the Display, core's `display_binding` names its fields, and
 * the strip's binding gives their values in its Scope.
 */

import type { ReactNode } from "react";
import { DisplayKind, DisplayProp, display_binding, type MixerField } from "../pkg/integral_wasm.js";
import { paramFormatter, paramMeta } from "./paramDefaults";
import { envelopePath } from "./synth-ui";
import { TransferCurve } from "./TransferCurve";
import scope from "./synth-ui/Scope.module.css";

/** One prop of a mixer Display and the field it reads. */
interface DisplaySlot {
  prop: DisplayProp;
  field: MixerField;
}

const slotCache = new Map<DisplayKind, readonly DisplaySlot[]>();

/** The mixer fields `kind` reads, by prop, from core; empty when the mixer has no such Display. */
export function mixerDisplaySlots(kind: DisplayKind): readonly DisplaySlot[] {
  let slots = slotCache.get(kind);
  if (!slots) {
    slots = (display_binding(kind, undefined) ?? []).flatMap((slot) => {
      const { prop, mixer: field } = slot;
      slot.free();
      return field === undefined ? [] : [{ prop, field }];
    });
    slotCache.set(kind, slots);
  }
  return slots;
}

/** A prop's field and its current raw value. */
interface Read {
  field: MixerField;
  value: number;
}

/** A read's value scaled to 0–1 over its field's range. */
function normalized({ field, value }: Read): number {
  const { min, max } = paramMeta(field);
  return (value - min) / (max - min || 1);
}

/** The numeric ratio a compressor ratio value names ("inf:1" is a limiter). */
function ratioOf({ field, value }: Read): number {
  const r = parseFloat(paramFormatter(field)(value));
  return Number.isNaN(r) ? Infinity : r;
}

/** Room left between a Display and its Section's border, px. */
const PAD = { x: 6, y: 2 } as const;

/**
 * Draws mixer Display `kind` filling `size`, its fields read through
 * `value`; nothing for a Display the mixer doesn't have.
 */
export function renderMixerDisplay(
  kind: DisplayKind,
  size: { width: number; height: number },
  value: (field: MixerField) => number,
): ReactNode {
  const reads = new Map<DisplayProp, Read>();
  for (const { prop, field } of mixerDisplaySlots(kind)) reads.set(prop, { field, value: value(field) });
  const width = size.width - 2 * PAD.x;
  const height = size.height - 2 * PAD.y;
  const frame = { padding: `${PAD.y}px ${PAD.x}px` };
  switch (kind) {
    case DisplayKind.CompTransferCurve: {
      const threshold = reads.get(DisplayProp.Threshold);
      const ratio = reads.get(DisplayProp.Ratio);
      if (!threshold || !ratio) return null;
      return (
        <div style={frame}>
          <TransferCurve
            threshold={threshold.value}
            ratio={ratioOf(ratio)}
            max={paramMeta(threshold.field).max}
            width={width}
            height={height}
          />
        </div>
      );
    }
    case DisplayKind.CompEnvelope: {
      const attack = reads.get(DisplayProp.Attack);
      const decay = reads.get(DisplayProp.Decay);
      if (!attack || !decay) return null;
      // Plots table index, not ms: the time tables are non-linear.
      const d = envelopePath({ attack: normalized(attack), decay: normalized(decay) });
      return (
        <div style={frame}>
          <svg
            className={scope.screen}
            style={{ display: "block", width, height, minWidth: 0, margin: 0 }}
            viewBox="-0.05 -0.15 1.1 1.3"
            preserveAspectRatio="none"
          >
            <path d={d} fill="none" stroke="#fc8" strokeWidth="2" vectorEffect="non-scaling-stroke" />
          </svg>
        </div>
      );
    }
    default:
      return null;
  }
}
