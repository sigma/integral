import { deviceSpecJson } from "../pkg/integral_wasm.js";

interface DeviceSpec {
  name: string;
  part_count: number;
  comp_eq_unit_count: number;
  output_assigns: string[];
  comp_eq_output_assigns: string[];
  chorus_type_names: string[];
  chorus_output_names: string[];
  reverb_type_names: string[];
  reverb_output_names: string[];
  surround_room_types: string[];
  surround_room_sizes: string[];
}

let cached: DeviceSpec | null = null;

/** Returns the parsed INTEGRA-7 device specification (lazy-loaded from WASM). */
export function deviceSpec(): DeviceSpec {
  if (!cached) {
    cached = JSON.parse(deviceSpecJson()) as DeviceSpec;
  }
  return cached;
}
