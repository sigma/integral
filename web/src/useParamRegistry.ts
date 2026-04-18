import { mixerParamsJson } from "../pkg/integral_wasm.js";

interface ParamDef {
  id: string;
  name: string;
  min: number;
  max: number;
  default_value: number;
}

let cachedParams: Map<string, ParamDef> | null = null;

export function useParamRegistry(): Map<string, ParamDef> {
  if (!cachedParams) {
    const defs: ParamDef[] = JSON.parse(mixerParamsJson());
    cachedParams = new Map(defs.map((d) => [d.id, d]));
  }
  return cachedParams;
}
