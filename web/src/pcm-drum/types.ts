// ---------------------------------------------------------------------------
// Types mirroring the Rust serde output (PcmDrumCommon, PcmDrumPartial, etc.)
// ---------------------------------------------------------------------------

export interface PcmDrumCommon {
  kitName: string;
  kitLevel: number;
}

export interface PcmDrumWmt {
  wmtSwitch: number;
  waveGroupType: number;
  waveGroupId: number;
  waveNumberL: number;
  waveNumberR: number;
  waveGain: number;
  waveFxmSwitch: number;
  waveFxmColor: number;
  waveFxmDepth: number;
  waveTempoSync: number;
  coarseTune: number;
  fineTune: number;
  pan: number;
  randomPanSwitch: number;
  alternatePanSwitch: number;
  level: number;
  velocityRangeLower: number;
  velocityRangeUpper: number;
  velocityFadeLower: number;
  velocityFadeUpper: number;
}

export interface PcmDrumPartial {
  partialName: string;
  assignType: number;
  muteGroup: number;
  level: number;
  coarseTune: number;
  fineTune: number;
  randomPitchDepth: number;
  pan: number;
  randomPanDepth: number;
  alternatePanDepth: number;
  envMode: number;
  outputLevel: number;
  chorusSend: number;
  reverbSend: number;
  outputAssign: number;
  pitchBendRange: number;
  receiveExpression: number;
  receiveHold1: number;
  wmtVelocityControl: number;
  wmt: PcmDrumWmt[];
  pitchEnvDepth: number;
  pitchEnvVelocitySens: number;
  pitchEnvT1VelocitySens: number;
  pitchEnvT4VelocitySens: number;
  pitchEnvTime: number[];
  pitchEnvLevel: number[];
  tvfFilterType: number;
  tvfCutoffFrequency: number;
  tvfCutoffVelocityCurve: number;
  tvfCutoffVelocitySens: number;
  tvfResonance: number;
  tvfResonanceVelocitySens: number;
  tvfEnvDepth: number;
  tvfEnvVelocityCurve: number;
  tvfEnvVelocitySens: number;
  tvfEnvT1VelocitySens: number;
  tvfEnvT4VelocitySens: number;
  tvfEnvTime: number[];
  tvfEnvLevel: number[];
  tvaLevelVelocityCurve: number;
  tvaLevelVelocitySens: number;
  tvaEnvT1VelocitySens: number;
  tvaEnvT4VelocitySens: number;
  tvaEnvTime: number[];
  tvaEnvLevel: number[];
  oneShotMode: number;
}

export interface PcmDrumCommon2 {
  phraseNumber: number;
  tfxSwitch: number;
}

// ---------------------------------------------------------------------------
// Note name helpers
// ---------------------------------------------------------------------------

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

export function noteName(key: number): string {
  const name = NOTE_NAMES[key % 12]!;
  const octave = Math.floor(key / 12) - 1;
  return `${name}${octave}`;
}

export function isBlackKey(key: number): boolean {
  const n = key % 12;
  return n === 1 || n === 3 || n === 6 || n === 8 || n === 10;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export const FIRST_KEY = 21;
export const LAST_KEY = 108;

export function panFmt(raw: number): string {
  if (raw === 64) return "C";
  if (raw < 64) return `L${64 - raw}`;
  return `${raw - 64}R`;
}

export function signedFmt(raw: number, center: number): string {
  const v = raw - center;
  return v > 0 ? `+${v}` : String(v);
}

export const WAVE_GAIN_OPTIONS = [
  { value: 0, label: "-6dB" },
  { value: 1, label: "0dB" },
  { value: 2, label: "+6dB" },
  { value: 3, label: "+12dB" },
];

export const WAVE_GROUP_OPTIONS = [
  { value: 0, label: "INT" },
  { value: 1, label: "SRX" },
  { value: 2, label: "SRX" },
  { value: 3, label: "SRX" },
];

export const FILTER_TYPE_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "LPF" },
  { value: 2, label: "BPF" },
  { value: 3, label: "HPF" },
  { value: 4, label: "PKG" },
  { value: 5, label: "LPF2" },
  { value: 6, label: "LPF3" },
];

export const VELOCITY_CURVE_OPTIONS = [
  { value: 0, label: "FIXED" },
  { value: 1, label: "1" },
  { value: 2, label: "2" },
  { value: 3, label: "3" },
  { value: 4, label: "4" },
  { value: 5, label: "5" },
  { value: 6, label: "6" },
  { value: 7, label: "7" },
];

export const ASSIGN_TYPE_OPTIONS = [
  { value: 0, label: "MULTI" },
  { value: 1, label: "SINGLE" },
];

export const ENV_MODE_OPTIONS = [
  { value: 0, label: "NO-SUS" },
  { value: 1, label: "SUSTAIN" },
];

export const OUTPUT_ASSIGN_OPTIONS = [
  { value: 0, label: "PART" },
  { value: 1, label: "C+EQ1" },
  { value: 2, label: "C+EQ2" },
  { value: 3, label: "C+EQ3" },
  { value: 4, label: "C+EQ4" },
  { value: 5, label: "C+EQ5" },
  { value: 6, label: "C+EQ6" },
];

export const WMT_VELOCITY_CTRL_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "ON" },
  { value: 2, label: "RANDOM" },
];

export const ALT_PAN_OPTIONS = [
  { value: 0, label: "OFF" },
  { value: 1, label: "ON" },
  { value: 2, label: "REV" },
];

// ---------------------------------------------------------------------------
// SysEx offset helpers
// ---------------------------------------------------------------------------

/** Convert a linear byte index (0-194) to a 2-byte SysEx offset. */
export function linearToSysex(linear: number): number {
  return Math.floor(linear / 128) * 256 + (linear % 128);
}

/** WMT layer linear start indices. */
export const WMT_STARTS = [0x21, 0x3E, 0x5B, 0x78];

/** Compute the SysEx offset for a WMT field given the layer index and
 *  the field's byte position within the 29-byte layer. */
export function wmtFieldOffset(layer: number, fieldPos: number): number {
  return linearToSysex(WMT_STARTS[layer]! + fieldPos);
}
