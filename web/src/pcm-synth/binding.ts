/**
 * Values and bindings of a PCM Synth tone's fields. A partial's are for its Scope: each element of an
 * array field on its own, and a rate or time that holds tempo-synced notes
 * above its free values carrying its free/note toggle in the knob.
 */

import {
  pcms_common_spec,
  pcms_partial_count,
  pcms_partial_spec,
  pcms_partial_sync,
  pcms_pmt_spec,
  type PcmsCommonField,
  type PcmsPartialField,
  type PcmsPmtField,
} from "../../pkg/integral_wasm.js";
import { cached } from "../cached";
import { specBinding, type Binding } from "../layouts";
import {
  PCMS_COMMON_ENTRIES,
  PCMS_PARTIAL_ENTRIES,
  PCMS_PMT_ENTRIES,
  type PcmSynthCommon,
  type PcmSynthPartial,
  type PcmSynthPmt,
} from "./types";

/** A Partial field's spec, read from core once. */
export const partialSpec = cached(pcms_partial_spec);

/** Each field's tempo sync, from core; read on first use, not at import. */
const partialSync = cached(pcms_partial_sync);

/** Element `index` of `field` in `partial`, or the field's default before it loads. */
export function partialValue(partial: PcmSynthPartial, field: PcmsPartialField, index = 0): number {
  const v = partial[PCMS_PARTIAL_ENTRIES[field].key];
  return (Array.isArray(v) ? v[index] : v) ?? partialSpec(field).defaultValue;
}

/** `field` in `common`, or its default where the mirror doesn't hold it. */
export function commonValue(common: PcmSynthCommon, field: PcmsCommonField): number {
  const entry = PCMS_COMMON_ENTRIES[field];
  return entry ? common[entry.key] : pcms_common_spec(field).defaultValue;
}

/** PMT `field` common to all Partials in `pmt`, or its default where the mirror doesn't hold it. */
export function pmtValue(pmt: PcmSynthPmt, field: PcmsPmtField): number {
  const entry = PCMS_PMT_ENTRIES[field];
  return entry ? pmt[entry.key] : pcms_pmt_spec(field).defaultValue;
}

/**
 * The binding of element `index` of `field`, at `value`, written through
 * `onChange`. A field whose values from core's `noteFrom` up are tempo-synced
 * notes (an LFO rate, the partial delay) ranges over its free values or its
 * notes, whichever it holds, and its toggle switches between them, starting
 * from that mode's default.
 */
export function partialBinding(
  field: PcmsPartialField,
  index: number,
  value: number,
  onChange: (value: number) => void,
): Binding {
  const spec = partialSpec(field);
  const { name } = PCMS_PARTIAL_ENTRIES[field];
  const count = pcms_partial_count(field);
  const binding = specBinding(count > 1 ? `${name} (${index + 1} of ${count})` : name, spec, value, onChange);
  const sync = partialSync(field);
  if (!sync) return binding;
  const noteMode = value >= sync.noteFrom;
  const range = noteMode
    ? { min: sync.noteFrom, defaultValue: sync.noteDefault }
    : { max: sync.noteFrom - 1, defaultValue: spec.defaultValue };
  return {
    ...binding,
    ...range,
    sync: {
      labels: sync.labels,
      formatNumber: (v) => spec.formatNumber(v),
      noteMode,
      // One field holds both values, so switching mode starts from that mode's default.
      onChange: (on) => {
        if (on !== noteMode) onChange(on ? sync.noteDefault : spec.defaultValue);
      },
    },
  };
}
