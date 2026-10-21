import type { WasmFieldSpec } from "../pkg/integral_wasm.js";

/** A field's values as switch or select options, labelled by its core format. */
export function switchOptions(spec: WasmFieldSpec) {
  return Array.from({ length: spec.max - spec.min + 1 }, (_, i) => {
    const value = spec.min + i;
    return { value, label: spec.formatValue(value) };
  });
}

/** The keys of `T` holding a number. */
export type NumberKey<T> = { [K in keyof T]: T[K] extends number ? K : never }[keyof T];

/** Where a field sits in a serde mirror of its block, and its full name. */
export interface FieldEntry<T, K extends keyof T = NumberKey<T>> {
  /** The field's key in the mirror. */
  key: K;
  /** The field's full name (its Control's tooltip). */
  name: string;
}
