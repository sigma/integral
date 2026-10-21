/** Bindings built from core field specs, for callers resolving a Scope's fields. */

import type { WasmFieldSpec } from "../../pkg/integral_wasm.js";
import type { Binding, BindingOption } from "./GridSection";

/**
 * The {@link Binding} of a field from its core spec: range, default, centre,
 * family and readout, at `value`, written through `onChange`.
 */
export function specBinding(
  name: string,
  spec: WasmFieldSpec,
  value: number,
  onChange: (value: number) => void,
  options?: readonly BindingOption[],
): Binding {
  return {
    name,
    value,
    min: spec.min,
    max: spec.max,
    defaultValue: spec.defaultValue,
    family: spec.family,
    center: spec.center,
    format: (v) => spec.formatValue(v),
    options,
    onChange,
  };
}
