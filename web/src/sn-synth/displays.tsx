/**
 * The SN Synth partial row's Displays, drawn from core's Display bindings
 * (`display_binding`): which fields each reads is core's, and this module
 * only turns the values read into a picture. The PCM Wave picker is the one
 * interactive Display: it writes the field it reads.
 */

import {
  DisplayKind,
  DisplayProp,
  ToneType,
  display_binding,
  sns_partial_spec,
  type SnsPartialField,
  type WasmDisplaySlot,
  type WasmFieldSpec,
} from "../../pkg/integral_wasm.js";
import { cached } from "../cached";
import { EnvelopeScope } from "../synth-ui";
import { LfoScope } from "../synth-ui/LfoScope";
import { WavePicker } from "../WavePicker";
import { snSynthWaveSource } from "../waveCatalog";
import { OscWaveScope } from "./OscWaveScope";
import type { PartialParamHandler } from "./types";
import css from "../SnSynthEditor.module.css";

/** Each kind's slots on the SN Synth editor; bindings are fixed, so read once. */
const slotsOf = cached((kind: DisplayKind): WasmDisplaySlot[] => display_binding(kind, ToneType.SnSynth) ?? []);

/** A Partial field's spec, read from core once. */
export const partialSpec: (field: SnsPartialField) => WasmFieldSpec = cached(sns_partial_spec);

/** One prop's value, as a Display draws it. */
interface PropValue {
  /** Raw value. */
  raw: number;
  /** 0..1 across its field's range. */
  unit: number;
  /** Core's name for the raw value. */
  label: string;
  /** The field read, if any. */
  field?: SnsPartialField;
}

/** The value of each prop of `kind`, read through `valueOf`. */
function propValues(kind: DisplayKind, valueOf: (field: SnsPartialField) => number): Map<DisplayProp, PropValue> {
  const out = new Map<DisplayProp, PropValue>();
  for (const slot of slotsOf(kind)) {
    const field = slot.snsPartial;
    const spec = field === undefined ? undefined : partialSpec(field);
    const raw = slot.fixed ?? (field === undefined ? 0 : valueOf(field));
    const unit = spec ? (raw - spec.min) / (spec.max - spec.min || 1) : 0;
    out.set(slot.prop, { raw, unit, label: spec?.formatValue(raw) ?? String(raw), field });
  }
  return out;
}

/** Props of {@link SnsPartialDisplay}. */
export interface SnsPartialDisplayProps {
  /** Which Display. */
  kind: DisplayKind;
  /** The partial's raw value of a field. */
  valueOf: (field: SnsPartialField) => number;
  /** Writes a field of the partial (the PCM Wave picker). */
  onChange: PartialParamHandler;
}

/** One Display of an SN Synth partial, filling its Cells. */
export function SnsPartialDisplay({ kind, valueOf, onChange }: SnsPartialDisplayProps) {
  const props = propValues(kind, valueOf);
  const unit = (prop: DisplayProp) => props.get(prop)?.unit;
  const raw = (prop: DisplayProp) => props.get(prop)?.raw ?? 0;
  const label = (prop: DisplayProp) => props.get(prop)?.label ?? "";
  switch (kind) {
    case DisplayKind.PitchEnvelope:
    case DisplayKind.FilterEnvelope:
    case DisplayKind.AmpEnvelope:
      return (
        <EnvelopeScope
          className={css.fill}
          stages={{
            attack: unit(DisplayProp.Attack) ?? 0,
            decay: unit(DisplayProp.Decay),
            sustain: unit(DisplayProp.Sustain),
            release: unit(DisplayProp.Release),
          }}
        />
      );
    case DisplayKind.OscScope:
      return (
        <OscWaveScope
          className={css.fill}
          wave={label(DisplayProp.Wave)}
          pulseWidth={raw(DisplayProp.PulseWidth)}
          pwmDepth={raw(DisplayProp.PwmDepth)}
          pwShift={raw(DisplayProp.PwShift)}
          superSawDetune={raw(DisplayProp.SuperSawDetune)}
        />
      );
    case DisplayKind.LfoScope:
    case DisplayKind.ModLfoScope:
      return <LfoScope className={css.fill} shape={label(DisplayProp.Shape)} rate={raw(DisplayProp.Rate)} />;
    case DisplayKind.WavePicker: {
      const wave = props.get(DisplayProp.WaveNumberL);
      const field = wave?.field;
      return (
        <div className={css.picker}>
          <WavePicker
            source={snSynthWaveSource()}
            stereo={false}
            l={wave?.raw ?? 0}
            onChange={(_side, n) => field !== undefined && onChange(field, n)}
          />
        </div>
      );
    }
    default:
      return null;
  }
}
