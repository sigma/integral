/**
 * Type-dependent FX parameter definitions.
 *
 * Each chorus/reverb type has a different set of meaningful parameters.
 * The param index maps to nibblized parameters at fixed SysEx offsets.
 */

export interface FxParamDef {
  index: number;
  name: string;
  min: number;
  max: number;
  defaultValue: number;
  format: (v: number) => string;
}

const fmtVal = (v: number) => String(v);
const fmtPct = (v: number) => `${v}%`;

// ---------------------------------------------------------------------------
// Chorus type parameter maps
// ---------------------------------------------------------------------------

const CHORUS_CHORUS: FxParamDef[] = [
  { index: 0, name: "Filter", min: 0, max: 2, defaultValue: 0, format: (v) => ["OFF", "LPF", "HPF"][v] ?? String(v) },
  { index: 1, name: "Cutoff", min: 200, max: 8000, defaultValue: 800, format: (v) => `${v}Hz` },
  { index: 2, name: "PreDly", min: 0, max: 100, defaultValue: 0, format: (v) => `${(v / 10).toFixed(1)}ms` },
  { index: 3, name: "Rate", min: 0, max: 1000, defaultValue: 100, format: (v) => `${(v / 100).toFixed(2)}Hz` },
  { index: 4, name: "Depth", min: 0, max: 127, defaultValue: 0, format: fmtVal },
  { index: 5, name: "Phase", min: 0, max: 180, defaultValue: 0, format: (v) => `${v}°` },
  { index: 6, name: "Fdbk", min: 0, max: 127, defaultValue: 0, format: fmtVal },
  { index: 7, name: "→Rev", min: 0, max: 127, defaultValue: 0, format: fmtVal },
];

const CHORUS_DELAY: FxParamDef[] = [
  { index: 0, name: "Dly L", min: 0, max: 1000, defaultValue: 200, format: (v) => `${v}ms` },
  { index: 1, name: "Dly R", min: 0, max: 1000, defaultValue: 200, format: (v) => `${v}ms` },
  { index: 2, name: "Dly C", min: 0, max: 1000, defaultValue: 200, format: (v) => `${v}ms` },
  { index: 3, name: "C Fdbk", min: -98, max: 98, defaultValue: 0, format: fmtPct },
  { index: 4, name: "HFDamp", min: 200, max: 8001, defaultValue: 8001, format: (v) => v > 8000 ? "BYP" : `${v}Hz` },
  { index: 5, name: "Lv L", min: 0, max: 127, defaultValue: 127, format: fmtVal },
  { index: 6, name: "Lv R", min: 0, max: 127, defaultValue: 127, format: fmtVal },
  { index: 7, name: "Lv C", min: 0, max: 127, defaultValue: 0, format: fmtVal },
  { index: 8, name: "→Rev", min: 0, max: 127, defaultValue: 0, format: fmtVal },
];

const CHORUS_GM2: FxParamDef[] = [
  { index: 0, name: "PreLPF", min: 0, max: 7, defaultValue: 0, format: fmtVal },
  { index: 1, name: "Level", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 2, name: "Fdbk", min: 0, max: 127, defaultValue: 0, format: fmtVal },
  { index: 3, name: "Delay", min: 0, max: 127, defaultValue: 0, format: fmtVal },
  { index: 4, name: "Rate", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 5, name: "Depth", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 6, name: "→Rev", min: 0, max: 127, defaultValue: 0, format: fmtVal },
];

export const CHORUS_PARAMS: Record<number, FxParamDef[]> = {
  0: [], // OFF
  1: CHORUS_CHORUS,
  2: CHORUS_DELAY,
  3: CHORUS_GM2,
};

export const CHORUS_TYPE_NAMES = ["OFF", "Chorus", "Delay", "GM2 Cho"];

// ---------------------------------------------------------------------------
// Reverb type parameter maps
// ---------------------------------------------------------------------------

const REVERB_ROOM_HALL_PLATE: FxParamDef[] = [
  { index: 0, name: "PreDly", min: 0, max: 100, defaultValue: 0, format: (v) => `${v}ms` },
  { index: 1, name: "Time", min: 1, max: 100, defaultValue: 30, format: (v) => `${(v / 10).toFixed(1)}s` },
  { index: 2, name: "Densty", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 3, name: "Diffus", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 4, name: "LFDamp", min: 0, max: 100, defaultValue: 0, format: fmtVal },
  { index: 5, name: "HFDamp", min: 0, max: 100, defaultValue: 50, format: fmtVal },
  { index: 6, name: "Spread", min: 0, max: 127, defaultValue: 64, format: fmtVal },
  { index: 7, name: "Tone", min: 0, max: 127, defaultValue: 64, format: fmtVal },
];

const REVERB_GM2: FxParamDef[] = [
  { index: 0, name: "Char", min: 0, max: 5, defaultValue: 0, format: fmtVal },
  { index: 1, name: "Time", min: 0, max: 127, defaultValue: 64, format: fmtVal },
];

export const REVERB_PARAMS: Record<number, FxParamDef[]> = {
  0: [], // OFF
  1: REVERB_ROOM_HALL_PLATE,
  2: REVERB_ROOM_HALL_PLATE,
  3: REVERB_ROOM_HALL_PLATE,
  4: REVERB_ROOM_HALL_PLATE,
  5: REVERB_ROOM_HALL_PLATE,
  6: REVERB_GM2,
};

export const REVERB_TYPE_NAMES = ["OFF", "Room 1", "Room 2", "Hall 1", "Hall 2", "Plate", "GM2 Rev"];

export const CHORUS_OUTPUT_NAMES = ["MAIN", "REV", "MAIN+REV"];
export const REVERB_OUTPUT_NAMES = ["A", "B", "C", "D"];
