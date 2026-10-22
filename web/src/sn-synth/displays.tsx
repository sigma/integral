/**
 * The SN Synth partial row's Displays, drawn from core's Display bindings
 * (`display_binding`): which fields each reads is core's, and this module
 * only turns the values read into a picture. The PCM Wave picker is a
 * Control, not one of them.
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
import { FilterScope, SNS_FILTER_RESPONSES } from "../synth-ui/FilterScope";
import { LfoScope } from "../synth-ui/LfoScope";
import { OscWaveScope } from "./OscWaveScope";
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
}

/** The value of each prop of `kind`, read through `valueOf`. */
function propValues(kind: DisplayKind, valueOf: (field: SnsPartialField) => number): Map<DisplayProp, PropValue> {
  const out = new Map<DisplayProp, PropValue>();
  for (const slot of slotsOf(kind)) {
    const field = slot.snsPartial;
    const spec = field === undefined ? undefined : partialSpec(field);
    const raw = slot.fixed ?? (field === undefined ? 0 : valueOf(field));
    const unit = spec ? (raw - spec.min) / (spec.max - spec.min || 1) : 0;
    out.set(slot.prop, { raw, unit, label: spec?.formatValue(raw) ?? String(raw) });
  }
  return out;
}

/** Props of {@link SnsPartialDisplay}. */
export interface SnsPartialDisplayProps {
  /** Which Display. */
  kind: DisplayKind;
  /** The partial's raw value of a field. */
  valueOf: (field: SnsPartialField) => number;
}

/** One Display of an SN Synth partial, filling its Cells. */
export function SnsPartialDisplay({ kind, valueOf }: SnsPartialDisplayProps) {
  const props = propValues(kind, valueOf);
  const unit = (prop: DisplayProp) => props.get(prop)?.unit;
  const raw = (prop: DisplayProp) => props.get(prop)?.raw ?? 0;
  const label = (prop: DisplayProp) => props.get(prop)?.label ?? "";
  /** -1..1 across the prop's field's range, 0 at its center. */
  const signed = (prop: DisplayProp) => 2 * (unit(prop) ?? 0.5) - 1;
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
    case DisplayKind.FilterScope:
      return (
        <FilterScope
          className={css.fill}
          type={label(DisplayProp.FilterType)}
          slope={label(DisplayProp.Slope)}
          cutoff={raw(DisplayProp.Cutoff)}
          resonance={raw(DisplayProp.Resonance)}
          hpf={raw(DisplayProp.HpfCutoff)}
          modulation={{
            envDepth: signed(DisplayProp.EnvDepth),
            envVelSens: signed(DisplayProp.EnvVelSens),
            aftertouch: signed(DisplayProp.AftertouchSens),
            lfo: signed(DisplayProp.LfoDepth),
            modLfo: signed(DisplayProp.ModLfoDepth),
            keyfollow: signed(DisplayProp.Keyfollow),
          }}
          responses={SNS_FILTER_RESPONSES}
        />
      );
    case DisplayKind.LfoScope:
    case DisplayKind.ModLfoScope:
      return <LfoScope className={css.fill} shape={label(DisplayProp.Shape)} rate={raw(DisplayProp.Rate)} />;
    default:
      return null;
  }
}
