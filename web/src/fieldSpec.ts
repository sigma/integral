import type { ParameterFamily, WasmFieldSpec } from "../pkg/integral_wasm.js";

/** The range, default, centre and format of a control, as SynthKnob and
 *  SynthFader take them. Callers without a core field spec build one literally. */
export interface KnobRangeProps {
  min: number;
  max: number;
  defaultValue: number;
  /** Value the LED arc grows from, for relative controls. */
  center?: number;
  formatValue?: (v: number) => string;
}

/** A knob's range props plus the parameter family that colours it. */
export interface KnobProps extends KnobRangeProps {
  family: ParameterFamily;
}

/** A field's core spec as SynthKnob range props. */
export function knobRange(spec: WasmFieldSpec): KnobRangeProps {
  const { min, max, defaultValue, center } = spec;
  return { min, max, defaultValue, center, formatValue: (v: number) => spec.formatValue(v) };
}

/** A field's core spec as SynthKnob props: its range and its Parameter family. */
export function knobProps(spec: WasmFieldSpec): KnobProps {
  return { ...knobRange(spec), family: spec.family };
}

/** A field's values as switch or select options, labelled by its core format. */
export function switchOptions(spec: WasmFieldSpec) {
  return Array.from({ length: spec.max - spec.min + 1 }, (_, i) => {
    const value = spec.min + i;
    return { value, label: spec.formatValue(value) };
  });
}
