/** Bindings built from core field specs, for callers resolving a Scope's fields. */

import { catalog_options, type FieldCatalog, type WasmFieldSpec } from "../../pkg/integral_wasm.js";
import { cached } from "../cached";
import type { Binding, BindingOption } from "./GridSection";

/** A catalog's entries as options, read from core once. */
const catalogOptions = cached((catalog: FieldCatalog): readonly BindingOption[] =>
  catalog_options(catalog).map((o) => {
    const option = { value: o.value, label: o.label, family: o.family };
    o.free();
    return option;
  }),
);

/**
 * The {@link Binding} of a field from its core spec: range, default, centre,
 * 0 dB mark, family and readout, at `value`, written through `onChange`. A
 * field core names from a catalog (a Wave, an Inst) lists the catalog's
 * entries, unless `options` overrides them.
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
    zeroDb: spec.zeroDb,
    format: (v) => spec.formatValue(v),
    // A field naming its values from a catalog (a Wave, an Inst) lists its entries.
    options: options ?? (spec.catalog === undefined ? undefined : catalogOptions(spec.catalog)),
    onChange,
  };
}
