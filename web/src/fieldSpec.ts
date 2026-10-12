import type { WasmFieldSpec } from "../pkg/integral_wasm.js";

/** A field's core spec as SynthKnob range props; throws if core doesn't specify the field. */
export function knobRange(spec: WasmFieldSpec | undefined) {
  if (!spec) throw new Error("field has no spec in core");
  const { min, max, defaultValue, center } = spec;
  return { min, max, defaultValue, center, formatValue: (v: number) => spec.formatValue(v) };
}
