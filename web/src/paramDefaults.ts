import { mixerParamsJson } from "../pkg/integral_wasm.js";

interface ParamMeta {
  name: string;
  min: number;
  max: number;
  defaultValue: number;
}

let registry: Map<string, ParamMeta> | null = null;

function getRegistry(): Map<string, ParamMeta> {
  if (!registry) {
    const defs: Array<{ id: string; name: string; min: number; max: number; default_value: number }> =
      JSON.parse(mixerParamsJson());
    registry = new Map<string, ParamMeta>();
    for (const d of defs) {
      registry.set(d.id, { name: d.name, min: d.min, max: d.max, defaultValue: d.default_value });
    }
  }
  return registry;
}

/** Look up param metadata by registry id. Falls back to sensible defaults. */
export function paramMeta(id: string): ParamMeta {
  return getRegistry().get(id) ?? { name: id, min: 0, max: 127, defaultValue: 0 };
}
