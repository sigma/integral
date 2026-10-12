import type { ReactNode } from "react";
import type { WasmFxParamDef } from "../pkg/integral_wasm.js";
import { SynthFader, SynthKnob, SynthSwitch } from "./synth-ui";
import { SyncToggle } from "./SyncToggle";
import css from "./ParamControl.module.css";

/** A spec param, paired with its note-mode view when it is tempo-syncable. */
export interface SpecParam {
  def: WasmFxParamDef;
  note: WasmFxParamDef | undefined;
}

/** How a skin draws a param. Range, format and slots always come from the spec. */
export interface ParamLook {
  label?: string;
  color?: string;
  as?: "knob" | "fader" | "switch";
}

/** Discrete ranges up to this many values default to a switch. */
const MAX_SWITCH_VALUES = 4;

/** Pair each def with its note view. The defs stay alive: their methods call into WASM. */
export function pairParams(defs: WasmFxParamDef[]): SpecParam[] {
  return defs.map((def) => ({ def, note: def.noteParam() }));
}

/** Whether the default control for `def` is a switch. */
export function isSwitch(def: WasmFxParamDef): boolean {
  return def.max - def.min + 1 <= MAX_SWITCH_VALUES;
}

interface Props {
  param: SpecParam;
  /** All stored slot values of the block. */
  slots: number[];
  onNibParam: (slot: number, value: number) => void;
  look?: ParamLook;
}

/** One control for one spec param; a msec/Hz-vs-note param edits whichever slot its switch selects. */
export function ParamControl({ param: { def, note }, slots, onNibParam, look = {} }: Props) {
  const active = note && def.noteMode(Int32Array.from(slots)) ? note : def;
  const shared = {
    label: look.label ?? def.name,
    title: def.name,
    value: slots[active.index] ?? active.defaultValue,
    onChange: (v: number) => onNibParam(active.index, v),
  };
  const format = (v: number) => active.formatValue(v);
  const range = { min: active.min, max: active.max, defaultValue: active.defaultValue, formatValue: format };
  const as = look.as ?? (isSwitch(active) ? "switch" : "knob");

  let control: ReactNode;
  if (as === "switch") {
    const options = Array.from({ length: active.max - active.min + 1 }, (_, i) => ({
      value: active.min + i,
      label: format(active.min + i),
    }));
    control = <SynthSwitch {...shared} options={options} vertical={options.length > 2} />;
  } else if (as === "fader") {
    control = <SynthFader {...shared} {...range} compact />;
  } else {
    control = <SynthKnob {...shared} {...range} color={look.color} />;
  }

  const sync = def.syncIndex;
  if (!note || sync === undefined) return control;
  return (
    <div className={css.synced}>
      {control}
      <SyncToggle
        labels={def.syncLabels}
        noteMode={active === note}
        onChange={(on) => onNibParam(sync, on ? 1 : 0)}
      />
    </div>
  );
}
