import { deviceSpecJson } from "../pkg/integral_wasm.js";

/** An inclusive range of MIDI key numbers. */
interface KeyRange {
  first: number;
  last: number;
}

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
  comp_attack_times: string[];
  comp_release_times: string[];
  comp_ratios: string[];
  sn_synth_partial_count: number;
  pcm_synth_partial_count: number;
  mfx_control_count: number;
  pcm_drum_wmt_count: number;
  sn_drum_keys: KeyRange;
  sn_drum_inst_number_max: number;
  pcm_drum_keys: KeyRange;
  tone_name_length: number;
  studio_set_name_length: number;
}

let cached: DeviceSpec | null = null;

/** Returns the parsed INTEGRA-7 device specification (lazy-loaded from WASM). */
export function deviceSpec(): DeviceSpec {
  if (!cached) {
    cached = JSON.parse(deviceSpecJson()) as DeviceSpec;
  }
  return cached;
}
