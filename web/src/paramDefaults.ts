import { mixer_field_name, mixer_field_spec, type MixerField, type ParameterFamily } from "../pkg/integral_wasm.js";

/** A mixer field's display name, range, default and Parameter family, from core. */
export interface ParamMeta {
  name: string;
  min: number;
  max: number;
  defaultValue: number;
  /** The parameter's role in shaping the sound; picks its colour. */
  family: ParameterFamily;
}

const metas = new Map<MixerField, ParamMeta>();

/** Look up a mixer field's metadata in core's registry. Fields are named by
 *  core's `MixerField` enum, so a misspelt field fails `tsc`. */
export function paramMeta(field: MixerField): ParamMeta {
  let meta = metas.get(field);
  if (!meta) {
    const { min, max, defaultValue, family } = mixer_field_spec(field);
    meta = { name: mixer_field_name(field), min, max, defaultValue, family };
    metas.set(field, meta);
  }
  return meta;
}

/**
 * Display formatter for a mixer field, using core's value labels. It looks
 * the spec up on first use, so a module may build one at import, before the
 * app has started the wasm core.
 */
export function paramFormatter(field: MixerField): (value: number) => string {
  let spec: ReturnType<typeof mixer_field_spec> | undefined;
  return (value) => (spec ??= mixer_field_spec(field)).formatValue(value);
}
