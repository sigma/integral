import type { WasmFieldSpec } from "../pkg/integral_wasm.js";

/** A field's core spec as SynthKnob range props. */
export function knobRange(spec: WasmFieldSpec) {
  const { min, max, defaultValue, center } = spec;
  return { min, max, defaultValue, center, formatValue: (v: number) => spec.formatValue(v) };
}

/** A field's core spec as SynthKnob props: its range and its Parameter family. */
export function knobProps(spec: WasmFieldSpec) {
  return { ...knobRange(spec), family: spec.family };
}

/** A field's values as switch or select options, labelled by its core format. */
export function switchOptions(spec: WasmFieldSpec) {
  return Array.from({ length: spec.max - spec.min + 1 }, (_, i) => {
    const value = spec.min + i;
    return { value, label: spec.formatValue(value) };
  });
}
