/**
 * The boxes of an SN Synth partial's row, drawn from their layouts
 * (`layouts/sn-synth/partial`) for the row's Scope; its switch strip is
 * `PartialSwitch`. Everything about the state comes from core: values, ranges,
 * relevance, which OSC alternatives show, and which field a synced rate
 * knob edits; the knob draws its tempo-sync toggle. A partial core holds
 * switched off (`sns_partial_scope_relevance`, via `scopeRelevance`) dims its boxes,
 * each as one Section, and they stay editable. Within a box, each Control
 * dims or hides by `sns_partial_relevance`.
 */

import { useMemo } from "react";
import {
  sns_osc_wave_pcm,
  Relevance,
  sns_partial_relevance,
  sns_partial_sync,
  sns_partial_value,
  type ParameterFamily,
  SnsPartialField,
  type WasmSnsPartialSync,
} from "../../pkg/integral_wasm.js";
import { GridSection, ScopeGate, specBinding, type Binding, type SectionLayout } from "../layouts";
import { memo } from "../layouts/fields";
import {
  OSC_VARIATION_ALTERNATIVE,
  OSC_VARIATION_GROUP,
  OSC_WAVE_ALTERNATIVE,
  OSC_WAVE_GROUP,
  SNS_PARTIAL_FIELDS,
} from "../layouts/sn-synth/partial";
import { SnsPartialDisplay, partialSpec } from "./displays";
import type { PartialParamHandler, SnSynthPartial } from "./types";

/** Each rate field's tempo sync, from core; read on first use, not at import. */
const syncs = memo(
  () => new Map<SnsPartialField, WasmSnsPartialSync | undefined>(SNS_PARTIAL_FIELDS.ids.map((f) => [f, sns_partial_sync(f)])),
);

/** Props of {@link PartialBox}. */
export interface PartialBoxProps {
  /** Which partial, 0-based: the box's Scope. */
  idx: number;
  /** Which box. */
  layout: SectionLayout<SnsPartialField>;
  /** The family its top border takes. */
  family: ParameterFamily;
  /** The partial's state, as core serialized it. */
  partial: SnSynthPartial;
  /** Core's relevance of the partial's Scope: the box dims when it isn't active. */
  scopeRelevance: Relevance;
  /** Writes one of the partial's fields. */
  setP: PartialParamHandler;
}

/** One box of an SN Synth partial's row. */
export function PartialBox({ idx, layout, family, partial, scopeRelevance, setP }: PartialBoxProps) {
  const { values, relevance } = useMemo(() => {
    const ids = SNS_PARTIAL_FIELDS.ids;
    return {
      values: new Map(ids.map((f) => [f, sns_partial_value(f, partial)])),
      relevance: new Map(ids.map((f) => [f, sns_partial_relevance(f, partial)])),
    };
  }, [partial]);
  const valueOf = (f: SnsPartialField) => values.get(f) ?? 0;

  // A rate knob edits its note field while its tempo-sync switch is on, and
  // draws that switch as its toggle (core's sync info).
  const bind = (field: SnsPartialField): Binding => {
    const sync = syncs().get(field);
    const noteMode = sync !== undefined && valueOf(sync.switchField) !== 0;
    const edited = sync && noteMode ? sync.noteField : field;
    const spec = partialSpec(edited);
    const binding = specBinding(SNS_PARTIAL_FIELDS.nameOf(edited), spec, valueOf(edited), (v) => setP(edited, v));
    if (!sync) return binding;
    return {
      ...binding,
      sync: {
        labels: sync.labels,
        formatNumber: (v) => spec.formatNumber(v),
        noteMode,
        onChange: (on) => setP(sync.switchField, on ? 1 : 0),
      },
    };
  };
  const isPcm = valueOf(SnsPartialField.OscWave) === sns_osc_wave_pcm();
  const alternative = (group: string) => {
    if (group === OSC_WAVE_GROUP) return isPcm ? OSC_WAVE_ALTERNATIVE.pcm : OSC_WAVE_ALTERNATIVE.scope;
    // S-Saw takes Var's place while core holds it relevant (an SP-SAW wave).
    if (group === OSC_VARIATION_GROUP) {
      return relevance.get(SnsPartialField.SuperSawDetune) === Relevance.Active
        ? OSC_VARIATION_ALTERNATIVE.superSaw
        : OSC_VARIATION_ALTERNATIVE.variation;
    }
    return undefined;
  };

  return (
    <ScopeGate relevance={scopeRelevance}>
      <GridSection
        layout={layout}
        scope={{ partial: idx }}
        bind={bind}
        family={family}
        relevance={(f) => relevance.get(f) ?? Relevance.Active}
        alternative={alternative}
        renderDisplay={(kind) => <SnsPartialDisplay kind={kind} valueOf={valueOf} />}
      />
    </ScopeGate>
  );
}
