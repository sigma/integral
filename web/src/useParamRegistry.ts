import { mixerParamsJson, type ParameterFamily } from "../pkg/integral_wasm.js";

interface ParamDef {
  id: string;
  name: string;
  min: number;
  max: number;
  default_value: number;
  /** The parameter's role in shaping the sound; picks its colour. */
  family: ParameterFamily;
}

let cachedParams: Map<string, ParamDef> | null = null;

export function useParamRegistry(): Map<string, ParamDef> {
  if (!cachedParams) {
    const defs: ParamDef[] = JSON.parse(mixerParamsJson());
    cachedParams = new Map(defs.map((d) => [d.id, d]));
  }
  return cachedParams;
}
