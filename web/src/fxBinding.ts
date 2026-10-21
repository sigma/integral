import { useMemo } from "react";
import type { WasmFxParamDef } from "../pkg/integral_wasm.js";
import { fxParamDefs, type Binding, type FxBlock } from "./layouts";

/** A param def, paired with its note-mode view when it is tempo-syncable. */
interface FxParam {
  def: WasmFxParamDef;
  note: WasmFxParamDef | undefined;
}

/**
 * Binds the params of one FX type, by core's name, to the block's stored
 * slots. A msec/Hz-vs-note param edits whichever slot its switch selects, and
 * carries its toggle as the Binding's `sync`. The defs stay alive with the
 * memo: their methods call into WASM.
 */
export function useFxBind(
  block: FxBlock,
  type: number,
  slots: readonly number[],
  onNibParam: (slot: number, value: number) => void,
): (field: string) => Binding {
  const params = useMemo(
    () => new Map<string, FxParam>(fxParamDefs(block, type).map((def) => [def.name, { def, note: def.noteParam() }])),
    [block, type],
  );
  const stored = Int32Array.from(slots);
  return (field) => {
    const param = params.get(field);
    if (!param) throw new Error(`${block} type ${type} has no param ${field}`);
    const { def, note } = param;
    const active = note && def.noteMode(stored) ? note : def;
    const sync = def.syncIndex;
    return {
      name: def.name,
      value: slots[active.index] ?? active.defaultValue,
      min: active.min,
      max: active.max,
      defaultValue: active.defaultValue,
      family: def.family,
      format: (v) => active.formatValue(v),
      onChange: (v) => onNibParam(active.index, v),
      sync:
        note && sync !== undefined
          ? { labels: def.syncLabels, noteMode: active === note, onChange: (on) => onNibParam(sync, on ? 1 : 0) }
          : undefined,
    };
  };
}
