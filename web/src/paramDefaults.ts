import { formatMixerParam, mixerParamsJson, type ParameterFamily } from "../pkg/integral_wasm.js";

interface ParamMeta {
  name: string;
  min: number;
  max: number;
  defaultValue: number;
  /** The parameter's role in shaping the sound; absent for an unknown id. */
  family?: ParameterFamily;
}

let registry: Map<string, ParamMeta> | null = null;

function getRegistry(): Map<string, ParamMeta> {
  if (!registry) {
    const defs: Array<{
      id: string; name: string; min: number; max: number; default_value: number; family: ParameterFamily;
    }> =
      JSON.parse(mixerParamsJson());
    registry = new Map<string, ParamMeta>();
    for (const d of defs) {
      registry.set(d.id, { name: d.name, min: d.min, max: d.max, defaultValue: d.default_value, family: d.family });
    }
  }
  return registry;
}

/** Look up param metadata by registry id. Falls back to sensible defaults. */
export function paramMeta(id: string): ParamMeta {
  return getRegistry().get(id) ?? { name: id, min: 0, max: 127, defaultValue: 0 };
}

/** Display formatter for a mixer param, using the registry's value labels. */
export function paramFormatter(id: string): (value: number) => string {
  return (value) => formatMixerParam(id, value);
}
