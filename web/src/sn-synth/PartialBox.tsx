/**
 * The boxes of an SN Synth partial's row, drawn from their layouts
 * (`layouts/sn-synth/partial`) for the row's Scope; its switch strip is
 * `PartialSwitch`. Everything about the state comes from core: values, ranges,
 * relevance, which OSC alternative shows, and which field a synced rate
 * knob edits; the knob draws its tempo-sync toggle. A partial core holds
 * off (`sns_partial_scope_is_active`, via `active`) dims its boxes, inert,
 * each as one Section; the switch strip stays live. Within a box, each
 * Control dims or hides by `sns_partial_is_active`.
 */

import { useMemo } from "react";
import {
  sns_osc_wave_pcm,
  sns_partial_is_active,
  sns_partial_sync,
  sns_partial_value,
  type ParameterFamily,
  SnsPartialField,
  type WasmSnsPartialSync,
} from "../../pkg/integral_wasm.js";
import { GridSection, specBinding, type Binding, type SectionLayout } from "../layouts";
import { memo } from "../layouts/fields";
import { OSC_WAVE_ALTERNATIVE, OSC_WAVE_GROUP, SNS_PARTIAL_FIELDS } from "../layouts/sn-synth/partial";
import { SnsPartialDisplay, partialSpec } from "./displays";
import type { PartialParamHandler, SnSynthPartial } from "./types";
import css from "../SnSynthEditor.module.css";

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
  /** Whether core holds the partial active: the box dims, inert, when not. */
  active: boolean;
  /** Writes one of the partial's fields. */
  setP: PartialParamHandler;
}

/** One box of an SN Synth partial's row. */
export function PartialBox({ idx, layout, family, partial, active, setP }: PartialBoxProps) {
  const { values, relevant } = useMemo(() => {
    const ids = SNS_PARTIAL_FIELDS.ids;
    return {
      values: new Map(ids.map((f) => [f, sns_partial_value(f, partial)])),
      relevant: new Map(ids.map((f) => [f, sns_partial_is_active(f, partial)])),
    };
  }, [partial]);
  const valueOf = (f: SnsPartialField) => values.get(f) ?? 0;

  // A rate knob edits its note field while its tempo-sync switch is on, and
  // draws that switch as its toggle (core's sync info).
  const bind = (field: SnsPartialField): Binding => {
    const sync = syncs().get(field);
    const noteMode = sync !== undefined && valueOf(sync.switchField) !== 0;
    const edited = sync && noteMode ? sync.noteField : field;
    const binding = specBinding(SNS_PARTIAL_FIELDS.nameOf(edited), partialSpec(edited), valueOf(edited), (v) =>
      setP(edited, v),
    );
    if (!sync) return binding;
    return {
      ...binding,
      sync: { labels: sync.labels, noteMode, onChange: (on) => setP(sync.switchField, on ? 1 : 0) },
    };
  };
  const isPcm = valueOf(SnsPartialField.OscWave) === sns_osc_wave_pcm();

  return (
    <div className={active ? undefined : css.off} data-scope-inactive={active ? undefined : true} inert={!active}>
      <GridSection
        layout={layout}
        scope={{ partial: idx }}
        bind={bind}
        family={family}
        isActive={(f) => relevant.get(f) ?? true}
        alternative={(group) =>
          group === OSC_WAVE_GROUP ? (isPcm ? OSC_WAVE_ALTERNATIVE.pcm : OSC_WAVE_ALTERNATIVE.scope) : undefined
        }
        renderDisplay={(kind) => <SnsPartialDisplay kind={kind} valueOf={valueOf} onChange={setP} />}
      />
    </div>
  );
}
