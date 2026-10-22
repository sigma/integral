import { mixer_field_name, mixer_field_spec, type MixerField, type ParameterFamily, type WasmFieldSpec } from "../pkg/integral_wasm.js";
import type { Binding } from "./layouts";

/** A mixer field's display name, range, default, Parameter family and 0 dB point, from core. */
export interface ParamMeta {
  name: string;
  min: number;
  max: number;
  defaultValue: number;
  /** The parameter's role in shaping the sound; picks its colour. */
  family: ParameterFamily;
  /** The raw value a level fader marks 0 dB; none for a field drawn without a 0 dB mark. */
  zeroDb?: number;
}

/** Core's specs, one wasm object per field however often it is read. */
const specs = new Map<MixerField, WasmFieldSpec>();
const metas = new Map<MixerField, ParamMeta>();

/** Core's spec of a mixer field. */
function specOf(field: MixerField): WasmFieldSpec {
  let spec = specs.get(field);
  if (!spec) {
    spec = mixer_field_spec(field);
    specs.set(field, spec);
  }
  return spec;
}

/** Look up a mixer field's metadata in core's registry. Fields are named by
 *  core's `MixerField` enum, so a misspelt field fails `tsc`. */
export function paramMeta(field: MixerField): ParamMeta {
  let meta = metas.get(field);
  if (!meta) {
    const { min, max, defaultValue, family, zeroDb } = specOf(field);
    meta = { name: mixer_field_name(field), min, max, defaultValue, family, zeroDb };
    metas.set(field, meta);
  }
  return meta;
}

/**
 * Display formatter for a mixer field, using core's value labels. It reads
 * the spec when it first formats, so a module may build one at import,
 * before the app has started the wasm core.
 */
export function paramFormatter(field: MixerField): (value: number) => string {
  return (value) => specOf(field).formatValue(value);
}

/** A Grid Control's binding to a mixer field: core's name, range and labels, at `value`. */
export function mixerBinding(field: MixerField, value: number, onChange: (value: number) => void): Binding {
  return { ...paramMeta(field), value, center: specOf(field).center, format: paramFormatter(field), onChange };
}
