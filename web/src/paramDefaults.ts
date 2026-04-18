import { mixerParamsJson } from "../pkg/integral_wasm.js";

interface ParamMeta {
  name: string;
  min: number;
  max: number;
  defaultValue: number;
}

const registry: Map<string, ParamMeta> = (() => {
  const defs: Array<{ id: string; name: string; min: number; max: number; default_value: number }> =
    JSON.parse(mixerParamsJson());
  const m = new Map<string, ParamMeta>();
  for (const d of defs) {
    m.set(d.id, { name: d.name, min: d.min, max: d.max, defaultValue: d.default_value });
  }
  return m;
})();

/** Look up param metadata by registry id. Falls back to sensible defaults. */
export function paramMeta(id: string): ParamMeta {
  return registry.get(id) ?? { name: id, min: 0, max: 127, defaultValue: 0 };
}
